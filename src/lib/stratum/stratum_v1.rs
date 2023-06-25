use bitcoincore_rpc::bitcoin::{self, Address};

use crypto_bigint::Encoding;
use log::{error, info, warn};

use slab::Slab;
use std::{
    collections::HashMap,
    net::SocketAddr,
    str::FromStr,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex, RwLock,
    },
    time::Instant,
};

use serde_json::{json, Value};

use crate::{
    p2p::networking::{
        block::Block, difficulty::get_target_from_diff_units, hard_config::PPLNS_SHARE_UNITS,
        pplns::MyBtcAddr, protocol::ProtocolP2P, stratum_handler::CompleteStrartumHandler,
    },
    protocol::{JsonRpcProtocol, Protocol},
    server::Notifier,
    sickrpc::RpcReqBody,
};

use super::{
    client::StratumClient,
    common::{process_share, ShareResult},
    config::StratumConfig,
    handler::StratumHandler,
    header::BlockHeader,
    job::JobBtc,
    job_fetcher::BlockFetcher,
    job_manager::JobManager,
    protocol::{StratumRequestsBtc, StratumV1ErrorCodes, SubmitReqParams},
};

// original slush bitcoin stratum protocol
pub struct StratumV1<T: BlockFetcher<BlockT = bitcoin::Block>> {
    job_manager: RwLock<JobManager<JobBtc<bitcoin::Block, RpcReqBody>>>,
    client_count: AtomicU32,
    config: StratumConfig,
    pub handler: CompleteStrartumHandler<T::BlockT>,
    pub subscribed_clients: Mutex<Slab<Notifier>>,
    pub daemon_cli: T,
}

impl<T> StratumV1<T>
where
    T: BlockFetcher<BlockT = bitcoin::Block>,
{
    pub fn process_stratum_request(
        &self,
        req: StratumRequestsBtc,
        ctx: Arc<Mutex<StratumClient>>,
        ptx: &mut StratumProcessingContext<T::BlockT, RpcReqBody>,
    ) -> Result<(Value, Vec<RpcReqBody>), StratumV1ErrorCodes> {
        let now = Instant::now();
        info!("Received stratum request: {:?}", req);

        let res = match req {
            StratumRequestsBtc::Submit(req) => self.process_submit(req, ctx, ptx),
            StratumRequestsBtc::Subscribe => {
                let mut lock = ctx.lock().unwrap();
                let key = self
                    .subscribed_clients
                    .lock()
                    .unwrap()
                    .insert(lock.notifier.clone());

                lock.subscription_key = Some(key);
                Ok((
                    json!([
                        [
                            ["mining.set_difficulty", Value::Null],
                            ["mining.notify", Value::Null]
                        ],
                        hex::encode(lock.extra_nonce.to_be_bytes()),
                        std::mem::size_of_val(&lock.extra_nonce),
                        // extranonce 2
                    ]),
                    Vec::new(),
                ))
            }
            StratumRequestsBtc::Authorize(params) => {
                // TODO: get address
                let pk = match Address::from_str(&params.username) {
                    Ok(k) => {
                        // let atype = k.assume_checked().address_type();
                        // info!("Type: {:?}", atype);
                        k.require_network(bitcoin::Network::Bitcoin).unwrap()
                    }
                    Err(_) => {
                        return Err(StratumV1ErrorCodes::Unknown(String::from(
                            "Invalid address provided",
                        )));
                    }
                };
                ctx.lock()
                    .unwrap()
                    .authorized_workers
                    .insert(params.username, MyBtcAddr(pk));

                let jobs = self.job_manager.read().unwrap();
                let job = jobs.last_job();

                let diff = self.config.default_diff_units;
                let notifs = Vec::from([
                    (
                        "mining.set_difficulty".into(),
                        json!([diff as f64 / PPLNS_SHARE_UNITS as f64]),
                    ),
                    job.broadcast_message.clone(),
                ]);
                ctx.lock().unwrap().target = get_target_from_diff_units(diff);

                Ok((Value::Bool(true), notifs))
            }
        };

        let elapsed = now.elapsed().as_micros();
        info!("Processed stratum response in {}us: {:?}", elapsed, &res);
        res
    }

    fn process_submit(
        &self,
        params: SubmitReqParams,
        ctx: Arc<Mutex<StratumClient>>,
        ptx: &mut StratumProcessingContext<T::BlockT, RpcReqBody>,
    ) -> Result<(Value, Vec<RpcReqBody>), StratumV1ErrorCodes> {
        if !ptx
            .jobs
            .contains_key(&(self.job_manager.read().unwrap().get_job_count() - 1))
        {
            ptx.jobs = self.job_manager.read().unwrap().get_jobs()
        }

        let mut job = ptx.jobs.get_mut(&params.job_id);
        let mut lock = ctx.lock().unwrap();
        let address = match lock.authorized_workers.get(&params.worker_name) {
            Some(s) => s.clone(),
            None => return Err(StratumV1ErrorCodes::UnauthorizedWorker),
        };

        let res = process_share(
            &mut job,
            (params, lock.extra_nonce),
            &mut *lock,
        );

        match res {
            ShareResult::Block(diff) => {
                info!("Found block!");
                let job = job.unwrap();
                if let Err(e) = self.daemon_cli.submit_block(&job.block) {
                    error!("Failed to submit block: {}", e);
                }

                self.handler.on_valid_share(address, &job.block, diff)
            }
            ShareResult::Valid(diff) => {
                self.handler
                    .on_valid_share(address, &job.unwrap().block, diff)
            }
            _ => {}
        };

        let res: Result<Value, StratumV1ErrorCodes> = res.into();
        match res {
            Ok(k) => Ok((k, Vec::new())),
            Err(e) => Err(e),
        }
    }

    pub fn parse_stratum_req(
        method: String,
        params: Value,
    ) -> Result<StratumRequestsBtc, serde_json::Error> {
        match method.as_ref() {
            "mining.submit" => Ok(StratumRequestsBtc::Submit(serde_json::from_value(params)?)),
            "mining.authorize" => Ok(StratumRequestsBtc::Authorize(serde_json::from_value(
                params,
            )?)),
            "mining.subscribe" => Ok(StratumRequestsBtc::Subscribe),
            unknown => Err(serde::de::Error::custom(format!(
                "Unknown method: {}",
                unknown
            ))),
        }
    }

    pub fn fetch_new_job(&self, header_fetcher: &T) {
        let mut lock = self.job_manager.write().unwrap();
        let res = lock.get_new_job(
            header_fetcher,
            &self.handler.p2p.pplns_window.lock().unwrap().address_scores,
            self.handler
                .p2p
                .block_manager
                .p2p_tip()
                .block
                .get_header()
                .get_hash()
                .to_le_bytes(),
        );

        if let Ok(job) = res {
            if let Some(job) = job {
                let lock = self.subscribed_clients.lock().unwrap();
                info!(
                    "New job! broadcasting to {} clients: {:?}",
                    lock.len(),
                    lock
                );

                for (_token, notifier) in &*lock {
                    JsonRpcProtocol::<Self>::notify(job.broadcast_message.clone(), notifier);
                }
                self.handler.on_new_block(job.height, &job.block);
            }
        }
    }
}

impl Into<Result<Value, StratumV1ErrorCodes>> for ShareResult {
    fn into(self) -> Result<Value, StratumV1ErrorCodes> {
        match self {
            ShareResult::Valid(_) | ShareResult::Block(_) => Ok(Value::Bool(true)),
            ShareResult::Stale() => Err(StratumV1ErrorCodes::JobNotFound),
            ShareResult::Invalid() => Err(StratumV1ErrorCodes::LowDifficultyShare),
            ShareResult::Duplicate() => Err(StratumV1ErrorCodes::DuplicateShare),
        }
    }
}

pub struct StratumProcessingContext<T, E> {
    pub jobs: HashMap<u32, JobBtc<T, E>>,
}

impl<T, E> Default for StratumProcessingContext<T, E>
// where
// T: BlockFetcher<BlockT = bitcoin::Block>,
{
    fn default() -> Self {
        StratumProcessingContext {
            jobs: HashMap::new(),
        }
    }
}

// any client that can generate the compatible header can be suited to this stratum protocol
impl<T> Protocol for StratumV1<T>
where
    T: BlockFetcher<BlockT = bitcoin::Block>,
{
    // method, params
    type Request = RpcReqBody;
    type Response = Result<(Value, Vec<RpcReqBody>), StratumV1ErrorCodes>;
    type Config = (StratumConfig, Arc<ProtocolP2P<T::BlockT>>);
    type ClientContext = StratumClient;
    type ProcessingContext = StratumProcessingContext<T::BlockT, RpcReqBody>;

    fn new(conf: Self::Config) -> Self {
        // let p = .clone();

        let daemon_cli = T::new(conf.0.rpc_url.as_ref());

        StratumV1 {
            job_manager: RwLock::new(JobManager::new(&daemon_cli)),
            client_count: AtomicU32::new(1),
            subscribed_clients: Mutex::new(Slab::new()),
            daemon_cli,
            handler: CompleteStrartumHandler { p2p: conf.1 },
            config: conf.0,
        }
    }

    fn process_request(
        &self,
        req: Self::Request,
        ctx: Arc<Mutex<Self::ClientContext>>,
        ptx: &mut Self::ProcessingContext,
    ) -> Self::Response {
        match Self::parse_stratum_req(req.0, req.1) {
            Ok(stratum_req) => self.process_stratum_request(stratum_req, ctx, ptx),
            Err(e) => {
                warn!("Failed to parse stratum request: {}", e);
                return Err(StratumV1ErrorCodes::Unknown(format!(
                    "Failed to parse stratum request: {}",
                    e
                )));
            }
        }
    }

    fn create_client(&self, _addr: SocketAddr, notifier: Notifier) -> Option<Self::ClientContext> {
        let id = self.client_count.load(Ordering::Relaxed);
        self.client_count.store(id + 1, Ordering::Relaxed);
        Some(StratumClient::new(notifier, id))
    }

    fn delete_client(&self, ctx: Arc<Mutex<Self::ClientContext>>) {
        let lock = ctx.lock().unwrap();
        if let Some(subkey) = lock.subscription_key {
            self.subscribed_clients.lock().unwrap().remove(subkey);
        }

        // info!("Deleted client with token: {}", _token.0);
    }

    fn create_ptx(&self) -> Self::ProcessingContext {
        Self::ProcessingContext {
            jobs: self.job_manager.read().unwrap().get_jobs(),
        }
    }
}

// demo \n
/*
{"id": 1, "method": "mining.subscribe", "params": []}
{"params": ["slush.miner1", "password"], "id": 2, "method": "mining.authorize"}
{"params": ["slush.miner1", "00000000", "00000001", "504e86ed", "b2957c02"], "id": 4, "method": "mining.submit"}
 */
