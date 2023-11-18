use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use bitcoin::{address::NetworkUnchecked, Network};
use crypto_bigint::U256;
use serde::{Deserialize, Serialize};

use crate::{
    config::{ProtocolServerConfig, ServerConfig},
    p2p::networking::{
        block::Block,
        config::{ConfigP2P, ConsensusConfigP2P},
    },
    stratum::header::BlockHeader,
};

use super::coin::Coin;

#[derive(Serialize, Clone, Debug, Eq, Hash, PartialEq)]
pub struct MyBtcAddr(pub bitcoin::Address);

#[derive(Clone, Debug, PartialEq)]
pub struct Btc;

impl Coin for Btc {
    type Address = MyBtcAddr;
    type BlockT = bitcoin::Block;
    type Fetcher = bitcoincore_rpc::Client;

    const NAME: &'static str = "Bitcoin";
    const DONATION_ADDRESS: &'static str = "bcrt1q9ude4m7uetjdwv5ud5h6qn7740ret7sznanxch";

    const ATOMIC_UNITS: u64 = 8;
    const DIFF1: U256 =
        U256::from_be_hex("00000000FFFF0000000000000000000000000000000000000000000000000000");

    const DEFAULT_DAEMON_PORT: u16 = 8332;

    fn main_pool_consensus_config() -> ConsensusConfigP2P<Self::BlockT> {
        ConsensusConfigP2P {
            parent_pool_id: U256::ZERO,
            block_time_ms: Duration::from_secs(10).as_millis() as u64,
            diff_adjust_blocks: 16,
            genesis_share: bitcoin::blockdata::constants::genesis_block(Network::Bitcoin),
            password: None,
            target_1: Self::DIFF1,
            name: String::from("main"),
            default_port_p2p: Self::DEFAULT_P2P_PORT,
            default_port_stratum: Self::DEFAULT_STRATUM_PORT,
        }
    }
}

impl Btc {
    pub const NETWORK: Network = Network::Regtest;
}
impl<'de> Deserialize<'de> for MyBtcAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(MyBtcAddr(
            bitcoin::Address::<NetworkUnchecked>::deserialize(deserializer)?
                .require_network(Btc::NETWORK)
                .unwrap(),
        ))
    }
}

fn mine_genesis_share(mut genesis_block: bitcoin::Block, target_1: U256) -> bitcoin::Block {
    loop {
        genesis_block.header.nonce += 1;
        let hash = genesis_block.get_header().get_hash();

        if hash < target_1 {
            return genesis_block;
        }
    }
    // ConsensusConfigP2P {
    //     parent_pool_id: U256::ZERO,
    //     block_time: Duration::from_secs(10),
    //     diff_adjust_blocks: 16,
    //     genesis_share: U256::from_be_hex(
    //         "3a052ae3c5e1684d6648b2674ccf26f65d27f489f536a015146e986ee45ffbf6",
    //     ),
    //     password: None,
    //     target_1: Self::DIFF1,
    // };
}
