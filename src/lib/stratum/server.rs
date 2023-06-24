use std::sync::Arc;
use std::thread;

use bitcoincore_rpc::bitcoin;
use log::info;

use crate::config::ProtocolServerConfig;
use crate::p2p::networking::protocol::ProtocolP2P;
use crate::protocol::Protocol;

use crate::{protocol::JsonRpcProtocol, server::Server};

use super::{config::StratumConfig, job_fetcher::BlockFetcher, stratum_v1::StratumV1};

type SProtocol<T> = JsonRpcProtocol<StratumV1<T>>;

pub struct StratumServer<T: BlockFetcher<BlockT = bitcoin::Block>> {
    server: Server<SProtocol<T>>,
}

impl<T> StratumServer<T>
where
    T: BlockFetcher<BlockT = bitcoin::Block> + Send + Sync + 'static,
{
    pub fn new(
        conf: ProtocolServerConfig<StratumConfig>,
        p2p: Arc<ProtocolP2P<T::BlockT>>,
    ) -> Self {
        let job_poll_interval = conf.protocol_config.job_poll_interval;
        let protocol = Arc::new(SProtocol::<T>::new((conf.protocol_config, p2p)));

        let protocol_poll_cp = protocol.clone();
        thread::spawn(move || {
            let protocol = protocol_poll_cp;
            loop {
                thread::sleep(job_poll_interval);
                info!("Polling job...");

                protocol.up.fetch_new_job(&protocol.up.daemon_cli);
            }
        });

        Self {
            server: Server::new(conf.server_config, protocol),
        }
    }

    pub fn process_stratum(&mut self) {
        self.server.process_requests();
    }
}

// TODO: make control server
