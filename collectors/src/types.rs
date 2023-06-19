/// Artemis Collectors types implementation
use ethers::{
    prelude::*,
    types::{AccountDiff, Block, BlockId, Transaction, H160, H256, U64},
};
use qilin_cfmms::pool::Pool;

use parking_lot::RwLock;
use dashmap::DashMap;
use std::sync::Arc;
use std::collections::BTreeMap;

pub(crate) type RwLockMap = RwLock<DashMap<H160, Pool>>;

/// A block payload, containing the all pool's stats and block hash.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct BlockPayload {
    pub block_hash: Block<H256>,
    pub all_pools: Arc<RwLockMap>,
}

/// A new block event, containing the block number and hash.
#[derive(Debug, Clone)]
pub struct NewTx {
    pub tx: Transaction,
    pub state_diff: BTreeMap<H160, AccountDiff>,
}