use std::collections::{HashSet, HashMap};

use io_arc::IoArc;
use mio::net::TcpStream;
use crypto_bigint::U256;

use crate::{p2p::networking::protocol::Address, server::Notifier};

#[derive(Debug)]
pub struct StratumClient {
    pub notifier: Notifier,
    pub extra_nonce: usize,
    pub authorized_workers: HashMap<String, Address>,
    pub submitted_shares: HashSet<u64>,
    pub difficulty: U256,
    pub subscription_key: Option<usize>
}

impl StratumClient {
    pub fn new(notifier: Notifier, id: usize) -> StratumClient {
        StratumClient {
            notifier,
            extra_nonce: id,
            difficulty: U256::ZERO,
            authorized_workers: HashMap::new(),
            submitted_shares: HashSet::new(),
            subscription_key: None
        }
    }
}
