use codec_sv2::Frame;
use demand_share_accounting_ext::parser::PoolExtMessages;
use rand::RngCore;
use roles_logic_sv2::mining_sv2::SubmitSharesExtended;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Receiver;
use tokio::task::AbortHandle;
use tokio::task::JoinHandle;

use crate::Message;

#[derive(Debug)]
pub struct AbortOnDrop {
    abort_handle: AbortHandle,
}

impl AbortOnDrop {
    pub fn new<T: Send + 'static>(handle: JoinHandle<T>) -> Self {
        let abort_handle = handle.abort_handle();
        Self { abort_handle }
    }

    pub fn _is_finished(&self) -> bool {
        self.abort_handle.is_finished()
    }
}

impl core::ops::Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.abort_handle.abort()
    }
}

impl<T: Send + 'static> From<JoinHandle<T>> for AbortOnDrop {
    fn from(value: JoinHandle<T>) -> Self {
        Self::new(value)
    }
}

pub async fn receive_response(stream: &mut TcpStream) -> Option<Vec<u8>> {
    let mut buffer = [0u8; 1024];
    match stream.readable().await {
        Ok(_) => match stream.try_read(&mut buffer) {
            Ok(n) if n > 0 => Some(buffer[..n].to_vec()),
            _ => None,
        },
        Err(_) => None,
    }
}

pub async fn log_incoming_messages(
    mut receiver: Receiver<Message>,
    protocol: &str,
    relay_up: AbortOnDrop,
    relay_down: AbortOnDrop,
) {
    while let Some(msg) = receiver.recv().await {
        println!("[{}] Received message: {:?}", protocol, msg);
    }
    drop(relay_down);
    drop(relay_up);
    println!("[{}] Message channel closed", protocol);
}

pub async fn log_incoming_jd_messages(
    mut receiver: Receiver<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
    protocol: &str,
) {
    while let Some(msg) = receiver.recv().await {
        println!("[{}] Received message: {:?}", protocol, msg);
    }
    println!("[{}] Message channel closed", protocol);
}

pub async fn send_random_bytes(
    stream: &mut TcpStream,
    size: usize,
) -> Result<(), crate::errors::Error> {
    let mut random_bytes = vec![0u8; size];
    rand::thread_rng().fill_bytes(&mut random_bytes);
    stream.write_all(&random_bytes).await?;
    Ok(())
}

pub fn submit_share_msg() -> SubmitSharesExtended<'static> {
    SubmitSharesExtended {
        channel_id: 1,
        sequence_number: 0,
        job_id: 1,
        nonce: 3131238739,
        ntime: 1748169317,
        version: 781836288,
        extranonce: vec![8].try_into().expect("failed to convert"),
    }
}
