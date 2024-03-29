use std::collections::HashMap;

use bitcoin::ScriptBuf;
use crypto_bigint::U256;
use log::info;

use crate::{p2p::networking::{block::Block, share::CoinbaseEncodedP2P}, stratum::job::Job};

use super::{header::BlockHeader, job::JobBtc, job_fetcher::BlockFetcher};

pub struct JobManager<JobT> {
    job_count: u32,
    jobs: HashMap<u32, JobT>,
}

// job manager is responsible for generating and updating jobs, the only one that can mutate jobs
impl<E> JobManager<JobBtc<bitcoin::Block, E>>
where
    JobBtc<bitcoin::Block, E>: Job<bitcoin::Block, E>,
{
    pub fn new<Fetcher: BlockFetcher<bitcoin::Block>>(
        header_fetcher: &Fetcher,
    ) -> JobManager<JobBtc<bitcoin::Block, E>> {
        let mut jobs = HashMap::with_capacity(16);

        // this is an invalid job, no outputs, a new one should be generated immediately
        match header_fetcher.fetch_blocktemplate(std::iter::empty(), CoinbaseEncodedP2P::default()) {
            Ok(res) => {
                let id = 0;
                let job = JobBtc::new(id, res);

                info!("First job: {:#?}", job);

                jobs.insert(id, job);
            }
            Err(e) => panic!("Failed to generate 1st job: {}", e),
        }

        JobManager { job_count: 1, jobs }
    }

    pub fn get_new_job<Fetcher: BlockFetcher<bitcoin::Block>>(
        &mut self,
        header_fetcher: &Fetcher,
        vout: impl Iterator<Item = (ScriptBuf, u64)>,
        cb_encoded: CoinbaseEncodedP2P,
    ) -> Result<Option<&JobBtc<bitcoin::Block, E>>, Fetcher::ErrorT> {
        let fetched = header_fetcher.fetch_blocktemplate(vout, cb_encoded)?;

        if fetched
            .block
            .get_header()
            .equal(&self.jobs[&(self.job_count - 1)].block.get_header())
        {
            return Ok(None);
        }

        let id = self.job_count;
        let job = JobBtc::new(id, fetched);

        self.job_count += 1;

        self.jobs.insert(id, job);

        Ok(Some(self.jobs.get(&id).unwrap()))
    }

    pub fn get_job_count(&self) -> u32 {
        self.job_count
    }

    pub fn last_job(&self) -> &JobBtc<bitcoin::Block, E> {
        &self.jobs[&(self.job_count - 1)]
    }

    pub fn get_jobs(&self) -> HashMap<u32, JobBtc<bitcoin::Block, E>> {
        self.jobs.clone()
    }
}
