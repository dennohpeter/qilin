/// Artemis Collectors types implementations
use ethers::{
    types::{AccountDiff, Block, Transaction, H160, H256},
};
use qilin_cfmms::pool::Pool;

use parking_lot::RwLock;
use dashmap::DashMap;
use std::sync::Arc;
use std::collections::BTreeMap;

pub(crate) type RwLockMap = RwLock<DashMap<H160, Pool>>;

/// A block payload, containing the all pool's states and block hash.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct BlockPayload {
    pub block_hash: Block<H256>,
    pub all_pools: Arc<RwLockMap>,
}

/// A new block event, containing the [Transaction] type and the `state_diff` BTreeMap.
#[derive(Debug, Clone)]
pub struct NewTx {
    pub tx: Transaction,
    pub state_diff: BTreeMap<H160, AccountDiff>,
}

impl Default for NewTx {
    fn default() -> Self {
        Self {
            tx: Transaction::default(),
            state_diff: BTreeMap::new(),
        }
    }
}