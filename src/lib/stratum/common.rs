use log::info;
use crypto_bigint::U256;

use crate::p2p::networking::{block::Block};

use super::{job::Job, header::BlockHeader, client::StratumClient};

pub enum ShareResult {
    Valid(U256),
    Block(U256),
    Stale(),
    Invalid(),
    Duplicate(),
}

#[inline]
pub fn process_share<T: Block>(
    job: &mut Option<&mut Job<T>>,
    params: <T::HeaderT as BlockHeader>::SubmitParams,
    client: &mut StratumClient,
) -> ShareResult {
    match job {
        Some(job) => {
            job.block.get_header_mut().update_fields(&params);
            let _share = job.block.clone();

            let hash = job.block.get_header().get_hash();
            let low = hash.as_words()[0];
            
            if client.submitted_shares.contains(&low) {
                return ShareResult::Duplicate();
            }

            client.submitted_shares.insert(low);

            info!("Hash {:x}", hash);

            if hash >= job.target {
                ShareResult::Block(hash)
            } else if hash >= client.difficulty {
                ShareResult::Valid(hash)
            } else {
                ShareResult::Invalid()
            }
        }
        None => ShareResult::Stale(),
    }
}