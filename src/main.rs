use clap::Parser;
use codec_sv2::{Frame, HandshakeRole, StandardEitherFrame, StandardSv2Frame};
use demand_share_accounting_ext::parser::PoolExtMessages;
use demand_sv2_connection::noise_connection_tokio::Connection;
use errors::Error;
use jd_connection::{setup_jd, test_invalid_jd_message, test_unexpected_jd_message};
use key_utils::Secp256k1PublicKey;
use lazy_static::lazy_static;
use mining_pool_connection::{
    setup, test_invalid_mining_message, test_invalid_setup, test_unexpected_message,
    test_unexpected_mining_message,
};
use noise_sv2::Initiator;
use std::{
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};

use tokio::{
    net::TcpStream,
    sync::mpsc::{channel, Receiver, Sender},
    time::timeout,
};

mod errors;
mod jd_connection;
mod mining_pool_connection;
mod utils;

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
const TEST_TIMEOUT: Duration = Duration::from_secs(10);

type Message = PoolExtMessages<'static>;
type MiningFrame = StandardSv2Frame<Message>;
type EitherFrame = StandardEitherFrame<Message>;

lazy_static! {
    static ref ARGS: Args = Args::parse();
}

#[derive(Parser)]
struct Args {
    #[clap(long = "pool", short = 'p')]
    pool_address: String,
    #[clap(
        long,
        short = 'k',
        default_value = "9auqWEzQDVyd2oe1JVGFLMLHZtCo2FFqZwtKA5gd9xbuEu7PH72"
    )]
    pub_key: String,
}

async fn start(pool: SocketAddr, auth_key: Secp256k1PublicKey) {
    if let Err(e) = run_pre_setup_tests(pool, auth_key).await {
        eprintln!("Pre-setup tests failed: {}", e);
        return;
    }

    let (receiver, relay_up_abortable, relay_down_abortable) =
        match test_mining_connection_setup(pool, auth_key).await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Mining connection failed: {}", e);
                return;
            }
        };

    let jd_receiver = match establish_jd_connection(pool, auth_key).await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("JD connection failed: {}", e);
            return;
        }
    };

    // Start background message loggers
    let mining_logger =
        utils::log_incoming_messages(receiver, "Mining", relay_up_abortable, relay_down_abortable);
    let jd_logger = utils::log_incoming_jd_messages(jd_receiver, "JD");

    // Keep loggers running until connections close
    tokio::join!(mining_logger, jd_logger);
}

async fn run_pre_setup_tests(pool: SocketAddr, auth_key: Secp256k1PublicKey) -> Result<(), Error> {
    println!("[TEST 1] Sending random bytes before Noise handshake...");
    test_random_bytes(pool, "pre-handshake").await?;

    let (mut receiver, sender) = create_noise_connection(pool, auth_key).await?;

    println!("[TEST 2] Sending random bytes after Noise handshake...");
    test_random_bytes(pool, "post-handshake").await?;

    println!("[TEST 3] Sending unexpected SV2 message after handshake...");
    test_unexpected_message(&sender, &mut receiver, "post-handshake").await?;

    println!("[TEST 4] Sending invalid SetupConnection message...");
    test_invalid_setup(&sender, &mut receiver).await?;

    Ok(())
}

async fn create_noise_connection(
    pool: SocketAddr,
    auth_key: Secp256k1PublicKey,
) -> Result<
    (
        Receiver<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
        Sender<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
    ),
    Error,
> {
    let stream = TcpStream::connect(pool).await.map_err(Error::Io)?;
    let initiator = Initiator::from_raw_k(auth_key.into_bytes())
        .map_err(|e| Error::Noise(errors::NoiseErrorKind::Noise(e)))?;

    let (receiver, sender, _, _) = Connection::new(stream, HandshakeRole::Initiator(initiator))
        .await
        .map_err(|e| Error::Noise(errors::NoiseErrorKind::Handshake(e)))?;

    Ok((receiver, sender))
}

async fn test_random_bytes(pool: SocketAddr, phase: &str) -> Result<(), Error> {
    let mut stream = TcpStream::connect(pool).await.map_err(Error::Io)?;
    utils::send_random_bytes(&mut stream, 100).await?;

    match timeout(TEST_TIMEOUT, utils::receive_response(&mut stream)).await {
        Ok(Some(res)) => {
            println!("Response to random bytes ({phase}): {:?}", &res);
        }
        Ok(None) => println!("No response received for random bytes ({phase})"),
        Err(_) => println!("Timeout waiting for response to random bytes ({phase})"),
    }
    Ok(())
}

async fn test_mining_connection_setup(
    pool: SocketAddr,
    auth_key: Secp256k1PublicKey,
) -> Result<(Receiver<Message>, utils::AbortOnDrop, utils::AbortOnDrop), Error> {
    let (mut receiver, sender) = create_noise_connection(pool, auth_key).await?;

    setup(&sender, &mut receiver).await?;

    println!("[TEST 5] Sending random bytes after SetupConnection.Success...");
    test_random_bytes(pool, "post-SetupConnection").await?;

    println!("[TEST 6] Sending unexpected SubmitSharesExtended message...");
    test_unexpected_mining_message(&sender, &mut receiver).await?;

    println!("[TEST 7] Sending invalid OpenExtendedMiningChannel...");
    test_invalid_mining_message(&sender, &mut receiver).await?;

    let (send_from_down, recv_from_down) = channel(100);
    let (_send_to_up, recv_to_up) = channel(100);

    let relay_up_task = mining_pool_connection::relay_up(recv_to_up, sender.clone());
    let relay_down_task = mining_pool_connection::relay_down(receiver, send_from_down);

    Ok((recv_from_down, relay_up_task, relay_down_task))
}

async fn establish_jd_connection(
    pool: SocketAddr,
    auth_key: Secp256k1PublicKey,
) -> Result<Receiver<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>, Error> {
    let (mut receiver, sender) = create_noise_connection(pool, auth_key).await?;

    setup_jd(&sender, &mut receiver, pool).await?;

    println!("[TEST 8] Sending random bytes after JD SetupConnection.Success...");
    test_random_bytes(pool, "JD post-SetupConnection").await?;

    println!("[TEST 9] Sending unexpected DeclareMiningJob message...");
    test_unexpected_jd_message(&sender, &mut receiver).await?;

    println!("[TEST 10] Sending invalid AllocateMiningJobToken...");
    test_invalid_jd_message(&mut receiver, &sender).await?;

    Ok(receiver)
}

#[tokio::main]
async fn main() {
    let pool = ARGS
        .pool_address
        .to_socket_addrs()
        .expect("Invalid pool address")
        .next()
        .expect("Could not resolve pool address");

    let auth_key: Secp256k1PublicKey = ARGS.pub_key.parse().expect("Invalid public key");
    start(pool, auth_key).await;
}
