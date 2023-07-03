use super::state_diff::{get_from_txs, StateDiffError};
use crate::types::NewTx;
use anyhow::Result;
use artemis::types::{Collector, CollectorStream};
use async_trait::async_trait;
use ethers::{
    prelude::Middleware,
    providers::PubsubClient,
    types::{AccountDiff, Block, BlockNumber, Transaction, H160, H256, U256, U64},
};
use log::error;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::runtime::Runtime;
use tokio_stream::StreamExt;

pub struct QilinMempoolCollector<M> {
    provider: Arc<M>,
    block: RwLock<Block<H256>>,
}

impl NewTx {
    pub fn new(tx: Transaction, state_diff: BTreeMap<H160, AccountDiff>) -> Self {
        Self { tx, state_diff }
    }
}

#[derive(Error, Debug)]
pub enum MempoolCollectorError<M>
where
    M: Middleware,
{
    #[error("Middleware error")]
    MiddlewareError(<M as Middleware>::Error),
    #[error("Error calculating max fee")]
    MaxFeeCalcError,
    #[error("Error Recovering ecdsa from tx")]
    EcdsaRecoveryError,
    #[error("Error getting block")]
    BlockError,
    #[error("Error getting block number")]
    BlockNumberError,
    #[error("Error getting block base fee")]
    BlockBaseFeeError,
    #[error("Error getting transaction trace from the tx")]
    GetTransactionTraceError,
}

impl<M> From<StateDiffError<M>> for MempoolCollectorError<M>
where
    M: Middleware,
{
    fn from(err: StateDiffError<M>) -> Self {
        match err {
            StateDiffError::MiddlewareError(e) => MempoolCollectorError::MiddlewareError(e),
            StateDiffError::GetTransactionTraceError => {
                MempoolCollectorError::GetTransactionTraceError
            }
        }
    }
}

impl<M> QilinMempoolCollector<M>
where
    M: Middleware + 'static,
    M::Provider: PubsubClient,
{
    pub fn new(provider: Arc<M>, block: Block<H256>) -> Self {
        Self {
            provider,
            block: RwLock::new(block),
        }
    }

    async fn update_block(&self, new_block: Block<H256>) {
        let mut block_writer = self.block.write();
        *block_writer = new_block;
    }

    async fn drop_max_fee_per_gas_tx<'a>(
        &self,
        tx: &'a mut Transaction,
    ) -> Result<&'a mut Transaction, MempoolCollectorError<M>> {
        let block_num = if let Ok(block_num) = self.provider.get_block_number().await {
            block_num
        } else {
            return Err(MempoolCollectorError::BlockNumberError);
        };

        let block = if let Ok(block) = self.provider.get_block(block_num).await {
            block.unwrap()
        } else {
            return Err(MempoolCollectorError::BlockError);
        };

        self.update_block(block.clone()).await;

        let next_block_base_fee = if let Some(basefee) = block.next_block_base_fee() {
            basefee
        } else {
            return Err(MempoolCollectorError::BlockBaseFeeError);
        };

        if tx.max_fee_per_gas.unwrap_or(U256::zero()) < next_block_base_fee {
            return Err(MempoolCollectorError::MaxFeeCalcError);
        }

        if let Ok(from) = tx.recover_from() {
            tx.from = from;
        } else {
            return Err(MempoolCollectorError::EcdsaRecoveryError);
        };

        Ok(tx)
    }

    pub async fn get_account_diffs(
        &self,
        tx: &mut Transaction,
    ) -> Result<BTreeMap<H160, AccountDiff>, MempoolCollectorError<M>> {
        let state_diffs = if let Some(state_diff) = get_from_txs(
            &self.provider,
            &vec![tx.clone()],
            BlockNumber::Number(self.block.read().number.unwrap_or(U64::zero())).into(),
        )
        .await
        {
            state_diff
        } else {
            return Err(MempoolCollectorError::GetTransactionTraceError);
        };

        Ok(state_diffs)
    }
}

#[async_trait]
impl<M> Collector<NewTx> for QilinMempoolCollector<M>
where
    M: Middleware + 'static,
    M::Provider: PubsubClient,
    M::Error: 'static,
{
    async fn get_event_stream(&self) -> Result<CollectorStream<'_, NewTx>> {
        let stream = self.provider.subscribe_pending_txs().await?;
        let stream = stream.transactions_unordered(256);
        let stream = stream.filter_map(|res| {
            let rt = Runtime::new().unwrap();

            let mut incoming_tx = if let Some(res) = res.ok() {
                res
            } else {
                return None;
            };

            let tx = if let Ok(tx) = rt.block_on(self.drop_max_fee_per_gas_tx(&mut incoming_tx)) {
                tx
            } else {
                return None;
            };

            let state_diff = rt.block_on(self.get_account_diffs(tx));

            if let Some(state_diff) = state_diff.ok() {
                let res = NewTx::new(tx.clone(), state_diff);
                return Some(res);
            } else {
                return None;
            };
        });
        Ok(Box::pin(stream))
    }
}
