use codec_sv2::{Frame, StandardSv2Frame};
use demand_share_accounting_ext::parser::PoolExtMessages;
use rand::distributions::DistString;
use roles_logic_sv2::{
    common_messages_sv2::{Protocol, SetupConnection},
    mining_sv2::OpenExtendedMiningChannel,
    parsers::Mining,
};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::timeout,
};

use crate::{
    errors::Error,
    utils::{self, AbortOnDrop},
    EitherFrame, CONNECTION_TIMEOUT, TEST_TIMEOUT,
};
pub type Message = PoolExtMessages<'static>;
pub type StdFrame = StandardSv2Frame<Message>;

pub async fn setup(
    sender: &Sender<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
    receiver: &mut Receiver<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
) -> Result<(), Error> {
    let setup_msg =
        PoolExtMessages::Common(roles_logic_sv2::parsers::CommonMessages::SetupConnection(
            get_mining_setup_connection_msg(true, 2, 2),
        ));
    let frame: StdFrame = setup_msg.try_into()?;
    sender
        .send(frame.into())
        .await
        .map_err(|e| Error::Send(e.to_string()))?;
    let response = timeout(CONNECTION_TIMEOUT, receiver.recv())
        .await
        .map_err(|_| Error::Timeout)?
        .ok_or(Error::UnexpectedMessage)?;

    let mut frame: StdFrame = response.try_into()?;
    let header = frame.get_header().ok_or(Error::UnexpectedMessage)?;
    let payload = frame.payload();

    let msg: roles_logic_sv2::parsers::CommonMessages<'_> =
        (header.msg_type(), payload).try_into()?;

    match msg {
        roles_logic_sv2::parsers::CommonMessages::SetupConnectionSuccess(_) => Ok(()),
        _ => Err(Error::UnexpectedMessage),
    }
}

pub async fn test_unexpected_message(
    sender: &Sender<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
    receiver: &mut Receiver<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
    phase: &str,
) -> Result<(), Error> {
    let msg = PoolExtMessages::Mining(open_channel());
    let frame: StdFrame = msg.try_into()?;
    sender
        .send(frame.into())
        .await
        .map_err(|e| Error::Send(e.to_string()))?;

    match timeout(TEST_TIMEOUT, receiver.recv()).await {
        Ok(Some(res)) => println!("Response to unexpected message ({phase}): {:?}", res),
        _ => println!("No response to unexpected message ({phase})"),
    }
    Ok(())
}

pub async fn test_invalid_setup(
    sender: &Sender<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
    receiver: &mut Receiver<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
) -> Result<(), Error> {
    // Invalid min and max version
    let invalid_msg =
        PoolExtMessages::Common(roles_logic_sv2::parsers::CommonMessages::SetupConnection(
            get_mining_setup_connection_msg(true, 10, 5),
        ));
    let frame: StdFrame = invalid_msg.try_into()?;
    sender
        .send(frame.into())
        .await
        .map_err(|e| Error::Send(e.to_string()))?;

    match timeout(TEST_TIMEOUT, receiver.recv()).await {
        Ok(Some(res)) => println!("Response to invalid SetupConnection: {:?}", res),
        _ => println!("No response to invalid SetupConnection"),
    }
    Ok(())
}

pub async fn test_unexpected_mining_message(
    sender: &Sender<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
    receiver: &mut Receiver<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
) -> Result<(), Error> {
    let unexpected_msg = PoolExtMessages::Mining(
        roles_logic_sv2::parsers::Mining::SubmitSharesExtended(utils::submit_share_msg()),
    );
    let frame: StdFrame = unexpected_msg.try_into()?;
    sender
        .send(frame.into())
        .await
        .map_err(|e| Error::Send(e.to_string()))?;

    match timeout(TEST_TIMEOUT, receiver.recv()).await {
        Ok(Some(res)) => println!("Response to unexpected mining message: {:?}", res),
        _ => println!("No response to unexpected mining message"),
    }
    Ok(())
}

pub async fn test_invalid_mining_message(
    sender: &Sender<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
    receiver: &mut Receiver<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
) -> Result<(), Error> {
    // set invalid min_extranonce_size
    let invalid_msg =
        roles_logic_sv2::parsers::Mining::OpenExtendedMiningChannel(OpenExtendedMiningChannel {
            request_id: 1,
            user_identity: "".to_string().try_into()?,
            nominal_hash_rate: 0.0,
            max_target: binary_sv2::u256_from_int(u64::MAX),
            min_extranonce_size: 8000,
        });
    let frame: StdFrame = PoolExtMessages::Mining(invalid_msg).try_into()?;
    sender
        .send(frame.into())
        .await
        .map_err(|e| Error::Send(e.to_string()))?;

    match timeout(TEST_TIMEOUT, receiver.recv()).await {
        Ok(Some(res)) => println!("Response to invalid mining message: {:?}", res),
        _ => println!("No response to invalid mining message"),
    }
    Ok(())
}

pub fn get_mining_setup_connection_msg(
    work_selection: bool,
    min_version: u16,
    max_version: u16,
) -> SetupConnection<'static> {
    let endpoint_host = "0.0.0.0"
        .to_string()
        .into_bytes()
        .try_into()
        .expect("Internal error: conversion failed");
    let vendor = String::new()
        .try_into()
        .expect("Internal error: conversion failed for vendor");
    let hardware_version = String::new()
        .try_into()
        .expect("Internal error: conversion failed for hardware_version");
    let firmware = String::new()
        .try_into()
        .expect("Internal error: conversion failed for firmware");
    let flags = if work_selection {
        0b0000_0000_0000_0000_0000_0000_0000_0110
    } else {
        0b0000_0000_0000_0000_0000_0000_0000_0100
    };
    let token = std::env::var("TOKEN").expect("TOKEN environment variable not set");
    let device_id = rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
    let device_id = format!("{}::POOLED::{}", device_id, token)
        .try_into()
        .expect("Internal error: conversion failed for device_id");
    SetupConnection {
        protocol: Protocol::MiningProtocol,
        min_version,
        max_version,
        flags,
        endpoint_host,
        endpoint_port: 50,
        vendor,
        hardware_version,
        firmware,
        device_id,
    }
}

pub fn relay_up(
    mut recv: Receiver<PoolExtMessages<'static>>,
    send: Sender<EitherFrame>,
) -> AbortOnDrop {
    let task = tokio::spawn(async move {
        while let Some(msg) = recv.recv().await {
            let std_frame: Result<StdFrame, _> = msg.try_into();
            if let Ok(std_frame) = std_frame {
                let either_frame: EitherFrame = std_frame.into();
                if send.send(either_frame).await.is_err() {
                    eprintln!("Mining upstream failed");
                    break;
                };
            } else {
                panic!("Internal Mining downstream try to send invalid message");
            }
        }
    });
    task.into()
}

pub fn relay_down(
    mut recv: Receiver<EitherFrame>,
    send: Sender<PoolExtMessages<'static>>,
) -> AbortOnDrop {
    let task = tokio::spawn(async move {
        while let Some(msg) = recv.recv().await {
            let msg: Result<StdFrame, ()> = msg.try_into().map_err(|_| ());
            if let Ok(mut msg) = msg {
                if let Some(header) = msg.get_header() {
                    let message_type = header.msg_type();
                    let payload = msg.payload();
                    let extension = header.ext_type();
                    let msg: Result<PoolExtMessages<'_>, _> =
                        (extension, message_type, payload).try_into();
                    if let Ok(msg) = msg {
                        let msg = msg.into_static();
                        if send.send(msg).await.is_err() {
                            eprintln!("Internal Mining downstream not available");
                        }
                    } else {
                        eprintln!("Mining Upstream send non Mining message. Disconnecting");
                        break;
                    }
                } else {
                    eprintln!("Mining Upstream send invalid message no header. Disconnecting");
                    break;
                }
            } else {
                eprintln!("Mining Upstream down.");
                break;
            }
        }
        eprintln!("Failed to receive msg from Pool");
    });
    task.into()
}

pub fn open_channel() -> Mining<'static> {
    roles_logic_sv2::parsers::Mining::OpenExtendedMiningChannel(OpenExtendedMiningChannel {
        request_id: 0,
        max_target: binary_sv2::u256_from_int(u64::MAX),
        min_extranonce_size: 8,
        user_identity: "health-check"
            .to_string()
            .try_into()
            .expect("Failed to convert user identity"),
        nominal_hash_rate: 0.0,
    })
}
