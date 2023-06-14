use super::state_diff::{get_from_txs, StateDiffError};
use anyhow::Result;
use artemis::types::{Collector, CollectorStream};
use async_trait::async_trait;
use dashmap::DashMap;
use ethers::{
    prelude::*,
    providers::{Middleware, PubsubClient},
    types::{Block, BlockId, Transaction, H160, H256, U64},
};
use log::{error, info};
use parking_lot::RwLock;
use qilin_cfmms::batch_requests;
use qilin_cfmms::pool::Pool;
use rusty::prelude::fork_factory::ForkFactory;
use std::sync::Arc;
use thiserror::Error;
use tokio::runtime::Runtime;
use tokio_stream::StreamExt;

type PoolVariant = cfmms::dex::DexVariant;
type RwLockMap = RwLock<DashMap<H160, Pool>>;

pub struct QilinBlockCollector<M> {
    provider: Arc<M>,
    block_hash: Block<H256>,
    block: RwLock<Block<Transaction>>,
    fork_factory: Arc<ForkFactory>,
    all_pools: Arc<RwLockMap>,
}

#[derive(Error, Debug)]
pub enum BlockCollectorError<M>
where
    M: Middleware,
{
    #[error("Middleware error")]
    MiddlewareError(<M as Middleware>::Error),
    #[error("Process block update error")]
    ProcessBlockUpdateError,
    #[error("Error getting transaction trace from the previous block")]
    GetStateDiffError(#[from] StateDiffError<M>),
}

impl<M> QilinBlockCollector<M>
where
    M: Middleware + 'static,
    M::Provider: PubsubClient,
{
    /// update the block hash and block transactions
    async fn process_block_update(
        &self,
        block_hash: &H256,
    ) -> Result<Vec<Transaction>, BlockCollectorError<M>> {
        // call the ForkFactory backend to get the full block
        let raw_block =
            if let Ok(raw_block) = self.fork_factory.get_full_block(BlockId::from(*block_hash)) {
                raw_block
            } else {
                return Err(BlockCollectorError::ProcessBlockUpdateError);
            };

        // create a deep copy of old block transactions before update and return
        let old_block_txs = self.block.read().transactions.clone();

        // update the block
        let mut block_writer = self.block.write();
        *block_writer = raw_block;

        Ok(old_block_txs)
    }

    /// update the the local pool state
    async fn update_pools(&self, meat: &Vec<Transaction>) -> Result<(), BlockCollectorError<M>> {
        // get last block number to do the tracing
        let last_block_num = self.block.read().number.unwrap() - U64::from(1);

        // extract the state_diffs
        let state_diffs =
            // take the state of last block and trace diffs
            if let Some(state_diffs) = get_from_txs(
                &self.provider.clone(),
                &meat,
                ethers::types::BlockNumber::Number(last_block_num)
            ).await {
                state_diffs
            } else {
                return Err(BlockCollectorError::GetStateDiffError(
                    StateDiffError::GetTransactionTraceError
                ));
            };

        let read_pool = self.all_pools.read();

        // get v2 and v3 pools that were touched
        let (mut touched_v3_pools, mut touched_v2_pools): (Vec<Pool>, Vec<Pool>) = state_diffs
            .keys()
            .filter_map(|e| read_pool.get(e).map(|p| (*p.value())))
            .partition(|pool| matches!(pool.pool_variant, PoolVariant::UniswapV3));
        drop(read_pool);

        // batch update v3 pools
        let v3_pool_slice = touched_v3_pools.as_mut_slice();
        batch_requests::uniswap_v3::get_pool_data_batch_request(
            v3_pool_slice,
            self.provider.clone(),
        )
        .await
        .unwrap_or_else(|e| {
            error!("Error: {}", e);
        });
        let write_pool = self.all_pools.write();
        v3_pool_slice.to_vec().into_iter().for_each(|pool| {
            write_pool.insert(pool.address, pool);
        });
        drop(write_pool);
        info!("write_pool: {:?}", self.all_pools);

        // batch update v2 pools
        let v2_pool_slice = touched_v2_pools.as_mut_slice();
        batch_requests::uniswap_v2::get_pool_data_batch_request(
            v2_pool_slice,
            self.provider.clone(),
        )
        .await
        .unwrap_or_else(|e| {
            error!("Error: {}", e);
        });
        let write_pool = self.all_pools.write();
        v2_pool_slice.to_vec().into_iter().for_each(|pool| {
            write_pool.insert(pool.address, pool);
        });
        drop(write_pool);

        Ok(())
    }

    async fn run_processore_n_update(
        &self,
        block_hash: &H256,
    ) -> Result<(), BlockCollectorError<M>> {
        let block_transactions = self.process_block_update(block_hash).await?;
        self.update_pools(&block_transactions).await?;
        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct BlockPayload {
    block_hash: Block<H256>,
    all_pools: Arc<RwLockMap>,
}

#[async_trait]
impl<M> Collector<BlockPayload> for QilinBlockCollector<M>
where
    M: Middleware + 'static,
    M::Provider: PubsubClient,
{
    async fn get_event_stream(&self) -> Result<CollectorStream<'_, BlockPayload>> {
        let block_stream = if let Ok(stream) = self.provider.subscribe_blocks().await {
            stream
        } else {
            panic!("Failed to connect");
        };

        let rt = Runtime::new().unwrap();

        let block_stream = block_stream.filter_map(move |block| {
            if let Some(hash) = block.hash {
                rt.block_on(self.run_processore_n_update(&hash)).unwrap();

                return Some(BlockPayload {
                    block_hash: self.block_hash.clone(),
                    all_pools: self.all_pools.clone(),
                });
            } else {
                return None;
            }
        });

        Ok(Box::pin(block_stream))
    }
}

// #[cfg(test)]
// mod test {
// use crate::state_manager::block_processor::process_block_update;
// use crate::utils::helpers::connect_to_network;
// use ethers::providers::{Middleware, Provider, Ws};
// use ethers::types::{BlockId, BlockNumber};
// use futures_util::StreamExt;
// use revm::db::{CacheDB, EmptyDB};
// use rusty::prelude::fork_factory::ForkFactory;
// use std::env;
// use std::error::Error;
// use std::sync::Arc;

// #[tokio::test]
// async fn test_process_block_update() {
//     // dotenv();
//     let _blast_key =
//         env::var("BLAST_API_KEY").expect("BLAST_API_KEY environment variable not set");
//     let mainnet_blast_url = format!("wss://eth-mainnet.blastapi.io/{}", _blast_key);

//     let result: Result<_, Box<dyn Error>> =
//         connect_to_network(&mainnet_blast_url, "https://relay.flashbots.net", 1).await;

//     let mut _ws_provider: Option<Arc<Provider<Ws>>> = None;
//     match result {
//         Ok((ws, _, _)) => {
//             _ws_provider = Some(ws);
//         }
//         Err(e) => {
//             println!("Error: {}", e);
//         }
//     }

//     let ws_provider = _ws_provider.unwrap();
//     let cache_db = CacheDB::new(EmptyDB::default());
//     let fork_block = ws_provider.as_ref().get_block_number().await;
//     let fork_block = fork_block
//         .ok()
//         .map(|number| BlockId::Number(BlockNumber::Number(number)));
//     let _fork_factory = Arc::new(ForkFactory::new_sandbox_factory(
//         ws_provider.clone(),
//         cache_db,
//         fork_block,
//     ));

//     tokio::spawn(async move {
//         let fork_factory = _fork_factory.clone();
//         println!("fork_factory: {:?}", ws_provider.clone());

//         loop {
//             let ws_provider = ws_provider.clone();
//             let mut block_stream = if let Ok(stream) = ws_provider.subscribe_blocks().await {
//                 stream
//             } else {
//                 panic!("Failed to connect");
//             };
//             while let Some(new_block) = block_stream.next().await {
//                 println!("New block: {:?}", new_block);
//                 if let Some(number) = new_block.number {
//                     let fork_factory = fork_factory.clone();
//                     tokio::task::spawn_blocking(move || {
//                         println!("New block: {:?}", number);
//                         let block_num = number.into();
//                         process_block_update(fork_factory.clone(), block_num).unwrap();
//                     });
//                 }
//             }
//         }
//     });
// }
// }
