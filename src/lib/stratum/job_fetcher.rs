use std::{fmt::Debug, path::PathBuf};

use bitcoin::{hashes::Hash, BlockHash, ScriptBuf};
use bitcoincore_rpc::{
    self,
    bitcoin::{self},
    bitcoincore_rpc_json::GetBlockTemplateResult,
    Auth, RpcApi,
};
use crypto_bigint::{Encoding, U256};

use crate::p2p::networking::{block::Block, share::CoinbaseEncodedP2P};

pub struct BlockFetch<BlockT> {
    pub block: BlockT,
    pub tx_hashes: Vec<[u8; 32]>,
    pub height: u32,
    pub reward: u64,
}

pub trait BlockFetcher<BlockT: Block>: Send + Sync + Debug + Sized {
    type ErrorT: std::fmt::Display + std::fmt::Debug + Sized;
    fn new(url: &str) -> Result<Self, Self::ErrorT>;
    fn fetch_blocktemplate(
        &self,
        vout: impl Iterator<Item = (BlockT::Script, u64)>,
        cb_encoded: CoinbaseEncodedP2P,
    ) -> Result<BlockFetch<BlockT>, Self::ErrorT>;
    fn submit_block(&self, block: &BlockT) -> Result<(), bitcoincore_rpc::Error>;

    fn fetch_block(&self, hash: &U256) -> Result<BlockT, bitcoincore_rpc::Error>;
    fn get_best_blockhash(&self) -> Result<U256, bitcoincore_rpc::Error>;
}

impl BlockFetcher<bitcoin::Block> for bitcoincore_rpc::Client
where
    bitcoin::Block: Block<BlockTemplateT = GetBlockTemplateResult>,
{
    type ErrorT = bitcoincore_rpc::Error;

    fn new(url: &str) -> Result<Self, Self::ErrorT> {
        Self::new(
            url,
            Auth::CookieFile("/home/sickguy/.bitcoin/regtest/regtest/.cookie".into()),
        )
    }

    fn fetch_blocktemplate(
        &self,
        vout: impl Iterator<Item = (ScriptBuf, u64)>,
        cb_encoded: CoinbaseEncodedP2P,
    ) -> Result<BlockFetch<bitcoin::Block>, bitcoincore_rpc::Error> {
        use bitcoincore_rpc::json::*;

        let header = self.get_block_template(
            GetBlockTemplateModes::Template,
            &[GetBlockTemplateRules::SegWit],
            &[],
        )?;
        let height = header.height as u32;

        let (block, tx_hashes) = bitcoin::Block::from_block_template(&header, vout, cb_encoded);

        Ok(BlockFetch {
            block,
            height,
            tx_hashes,
            reward: header.coinbase_value.to_sat(),
        })
    }

    fn fetch_block(&self, hash: &U256) -> Result<bitcoin::Block, bitcoincore_rpc::Error> {
        self.get_block(&BlockHash::from_byte_array(hash.clone().to_be_bytes()))
    }

    fn submit_block(&self, block: &bitcoin::Block) -> Result<(), bitcoincore_rpc::Error> {
        RpcApi::submit_block(self, &block)
    }

    fn get_best_blockhash(&self) -> Result<U256, bitcoincore_rpc::Error> {
        Ok(U256::from_be_bytes(
            RpcApi::get_best_block_hash(self)?.to_byte_array(),
        ))
    }
}
