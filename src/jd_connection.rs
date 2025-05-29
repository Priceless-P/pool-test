use binary_sv2::{Seq064K, Sv2DataType, B0255, B064K};
use codec_sv2::Frame;
use demand_share_accounting_ext::parser::PoolExtMessages;
use rand::distributions::{Alphanumeric, DistString};
use roles_logic_sv2::{
    common_messages_sv2::{Protocol, SetupConnection},
    handlers::{common::ParseUpstreamCommonMessages, SendTo_ as CSendTo},
    job_declaration_sv2::{AllocateMiningJobToken, DeclareMiningJob},
    parsers::JobDeclaration,
    routing_logic::NoRouting,
};
use std::{convert::TryInto, net::SocketAddr};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::timeout,
};

use crate::{errors::Error, MiningFrame, CONNECTION_TIMEOUT, TEST_TIMEOUT};

pub async fn setup_jd(
    sender: &Sender<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
    receiver: &mut Receiver<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
    pool: SocketAddr,
) -> Result<(), Error> {
    let setup_msg = SetupConnectionHandler::get_setup_connection_message(pool);
    let frame: MiningFrame = PoolExtMessages::Common(setup_msg.into()).try_into()?;
    sender
        .send(frame.into())
        .await
        .map_err(|e| Error::Send(e.to_string()))?;
    let response = timeout(CONNECTION_TIMEOUT, receiver.recv())
        .await
        .map_err(|_| Error::Timeout)?
        .ok_or(Error::Unrecoverable)?;

    let mut frame: MiningFrame = response.try_into()?;
    let header = frame.get_header().ok_or(Error::Unrecoverable)?;
    let payload = frame.payload();

    let msg: roles_logic_sv2::parsers::CommonMessages<'_> =
        (header.msg_type(), payload).try_into()?;

    match msg {
        roles_logic_sv2::parsers::CommonMessages::SetupConnectionSuccess(_) => Ok(()),
        _ => Err(Error::UnexpectedMessage),
    }
}

pub async fn test_unexpected_jd_message(
    sender: &Sender<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
    receiver: &mut Receiver<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
) -> Result<(), Error> {
    let msg = PoolExtMessages::JobDeclaration(JobDeclaration::DeclareMiningJob(DeclareMiningJob {
        request_id: 999,
        mining_job_token: B0255::from_vec_(vec![0; 32])?,
        coinbase_prefix: B064K::from_vec_(vec![0; 48])?,
        coinbase_suffix: B064K::from_vec_(vec![0; 128])?,
        version: 0,
        tx_list: Seq064K::new(vec![])?,
        excess_data: B064K::from_vec_(vec![0; 32])?,
    }));
    let frame: MiningFrame = msg.try_into()?;
    sender
        .send(frame.into())
        .await
        .map_err(|e| Error::Send(e.to_string()))?;

    match timeout(TEST_TIMEOUT, receiver.recv()).await {
        Ok(Some(res)) => println!("Response to unexpected JD message: {:?}", res),
        _ => println!("No response to unexpected JD message"),
    }
    Ok(())
}

#[allow(overflowing_literals)]
pub async fn test_invalid_jd_message(
    receiver: &mut Receiver<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
    sender: &Sender<Frame<PoolExtMessages<'static>, codec_sv2::buffer_sv2::Slice>>,
) -> Result<(), Error> {
    // Invalid request_id
    let msg = JobDeclaration::AllocateMiningJobToken(AllocateMiningJobToken {
        request_id: 4294967296,
        user_identifier: "".to_string().try_into()?,
    });
    let frame: MiningFrame = PoolExtMessages::JobDeclaration(msg).try_into()?;
    sender
        .send(frame.into())
        .await
        .map_err(|e| Error::Send(e.to_string()))?;
    match timeout(TEST_TIMEOUT, receiver.recv()).await {
        Ok(Some(res)) => println!("Response to Invaild JD message: {:?}", res),
        _ => println!("No response to Invaild JD message"),
    }
    Ok(())
}

pub struct SetupConnectionHandler {}

impl SetupConnectionHandler {
    pub fn get_setup_connection_message(proxy_address: SocketAddr) -> SetupConnection<'static> {
        let endpoint_host = proxy_address
            .ip()
            .to_string()
            .into_bytes()
            .try_into()
            .expect("Internal error: this operation can not fail because IP addr string can always be converted into Inner");
        let vendor = String::new().try_into().expect("Internal error: this operation can not fail because empty string can always be converted into Inner");
        let hardware_version = String::new().try_into().expect("Internal error: this operation can not fail because empty string can always be converted into Inner");
        let firmware = String::new().try_into().expect("Internal error: this operation can not fail because empty string can always be converted into Inner");
        let token = std::env::var("TOKEN").expect("Checked at initialization");
        let device_id = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        let device_id = format!("{}::POOLED::{}", device_id, token)
            .to_string()
            .try_into()
            .expect("Internal error: this operation can not fail because device_id string can always be converted into Inner");
        let mut setup_connection = SetupConnection {
            protocol: Protocol::JobDeclarationProtocol,
            min_version: 2,
            max_version: 2,
            flags: 0b0000_0000_0000_0000_0000_0000_0000_0000,
            endpoint_host,
            endpoint_port: proxy_address.port(),
            vendor,
            hardware_version,
            firmware,
            device_id,
        };
        setup_connection.set_async_job_nogotiation();
        setup_connection
    }
}

impl ParseUpstreamCommonMessages<NoRouting> for SetupConnectionHandler {
    fn handle_setup_connection_success(
        &mut self,
        _: roles_logic_sv2::common_messages_sv2::SetupConnectionSuccess,
    ) -> Result<roles_logic_sv2::handlers::common::SendTo, roles_logic_sv2::errors::Error> {
        Ok(CSendTo::None(None))
    }

    fn handle_setup_connection_error(
        &mut self,
        _: roles_logic_sv2::common_messages_sv2::SetupConnectionError,
    ) -> Result<roles_logic_sv2::handlers::common::SendTo, roles_logic_sv2::errors::Error> {
        todo!()
    }

    fn handle_channel_endpoint_changed(
        &mut self,
        _: roles_logic_sv2::common_messages_sv2::ChannelEndpointChanged,
    ) -> Result<roles_logic_sv2::handlers::common::SendTo, roles_logic_sv2::errors::Error> {
        todo!()
    }
}
