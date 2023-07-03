use super::state_diff::{get_from_txs, StateDiffError};
use crate::types::{BlockPayload, RwLockMap};
use anyhow::Result;
use artemis::types::{Collector, CollectorStream};
use async_trait::async_trait;
use ethers::{
    providers::{Middleware, PubsubClient},
    types::{Block, BlockId, Transaction, H256, U64},
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
    /// Update the block hash and block transactions
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

    // TODO: switch the trace_call_many to trace_replay_block_transactions
    // See https://docs.rs/ethers/latest/ethers/providers/trait.Middleware.html#method.trace_replay_block_transactions
    /// Update the the local pool state
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
