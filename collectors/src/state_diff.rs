use qilin_cfmms::pool::Pool;
use anyhow::Result;
use dashmap::DashMap;
use ethers::types::H160;
use hashbrown::HashMap;
use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
};
use thiserror::Error;

use super::slot_finder;
use ethers::prelude::*;
use futures::stream::FuturesUnordered;
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{AccountInfo, Bytecode},
};
use serde::{Serialize, Serializer};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};
use tokio::sync::RwLock;

pub type ArbPools = Vec<HashMap<Pool, Vec<Pool>>>;
type RustyPool = rusty::cfmm::Pool;
struct SerializedBTreeMap<K, V>(BTreeMap<K, V>);

impl<K, V> Serialize for SerializedBTreeMap<K, V>
where
    K: Serialize + Ord,
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct TradablePool {
    pub pool: RustyPool,
    pub is_weth_input: bool,
}

impl TradablePool {
    pub fn new(pool: RustyPool, is_weth_input: bool) -> Self {
        Self {
            pool,
            is_weth_input,
        }
    }
}

#[derive(Error, Debug)]
pub enum StateDiffError<M>
where
    M: Middleware,
{
    #[error("Middleware error")]
    MiddlewareError(<M as Middleware>::Error),
    #[error("Could not get transaction trace")]
    GetTransactionTraceError,
}

// Extract state diffs from a given tx
//
// Arguments:
// * `client`: Websocket provider used for making rpc calls
// * `meats`: Vec of transactions to extract state diffs from
// * `block_num`: Block number of the block the txs are in
//
// Returns:
// Some(BTreeMap<Address, AccountDiff>): State diffs for each address)
// None: If encountered error or state diffs are non existant
pub async fn get_from_txs<M>(
    client: &Arc<M>,
    meats: &Vec<Transaction>,
    block_num: BlockNumber,
) -> Option<BTreeMap<Address, AccountDiff>>
where
    M: Middleware + 'static,
{
    // add statediff trace to each transaction
    let req = meats
        .iter()
        .map(|tx| (tx, vec![TraceType::StateDiff]))
        .collect();

    let block_traces = match client.trace_call_many(req, Some(block_num)).await {
        Ok(x) => x,
        Err(e) => {
            println!("Block Trace Error: {:?}", e);
            return None;
        }
    };
    println!("block_traces: {:?}", block_traces);

    let mut merged_state_diffs = BTreeMap::new();

    block_traces
        .into_iter()
        .flat_map(|bt| bt.state_diff.map(|sd| sd.0.into_iter()))
        .flatten()
        .for_each(|(address, account_diff)| {
            match merged_state_diffs.entry(address) {
                Entry::Vacant(entry) => {
                    entry.insert(account_diff);
                }
                Entry::Occupied(_) => {
                    // Do nothing if the key already exists
                    // we only care abt the starting state
                }
            }
        });

    Some(merged_state_diffs)
}

pub async fn extract_arb_pools(
    provider: Arc<Provider<Ws>>,
    state_diffs: &BTreeMap<Address, AccountDiff>,
    all_pools: &Arc<RwLock<DashMap<Address, Pool>>>,
    hash_pools: &Arc<DashMap<H160, Vec<Pool>>>,
) -> Option<ArbPools> {
    let read_lock = all_pools.read().await;
    let touched_pools: Vec<Pool> = state_diffs
        .keys()
        .filter_map(|e| read_lock.get(e).map(|p| (*p.value())))
        .collect();
    drop(read_lock);

    let mut arb_pools: ArbPools = vec![];

    let mut exclusion_map: HashSet<Pool> = HashSet::new();

    for pool in touched_pools {
        if exclusion_map.contains(&pool) {
            continue;
        };

        let token0 = pool.token_0;
        let token1 = pool.token_1;

        let token0_state_diff = &state_diffs.get(&token0)?.storage;

        // read the balanceOf mapping from the ERC20 contract
        let slot = if let Some(slot) =
            slot_finder::slot_finder(provider.clone(), token0.clone(), pool.address).await
        {
            slot
        } else {
            // if not found, skip
            // currently bot don't support Vyper contract balanceOf slot finding
            break;
        };

        // key in the balanceOf mapping with pool's address
        let storage_key = TxHash::from(ethers::utils::keccak256(abi::encode(&[
            abi::Token::Address(pool.address),
            abi::Token::Uint(slot),
        ])));

        // if storage_diff is true, then pool has more token0 than before
        let storage_diff = match token0_state_diff.get(&storage_key)? {
            Diff::Changed(c) => {
                let from = U256::from(c.from.to_fixed_bytes());
                let to = U256::from(c.to.to_fixed_bytes());
                to > from
            }
            _ => break,
        };
        // hash token0 & token1 addresses to key in all the relevant pools from
        // hash_pools
        let mut hasher = DefaultHasher::new();
        token0.hash(&mut hasher);
        token1.hash(&mut hasher);
        let hash = hasher.finish();

        let mut pool_map: HashMap<Pool, Vec<Pool>> = HashMap::new();
        let pools = hash_pools.get(&H160::from_low_u64_be(hash))?;

        if storage_diff {
            let mut vec_pool: Vec<Pool> = vec![];
            for pool in pools.iter().filter(|p| p.address != pool.address) {
                exclusion_map.insert(pool.clone());
                vec_pool.push(pool.clone());
            }

            pool_map.insert(pool, vec_pool);

            // if to > from, then pool has more token0 and less token1 than before*
            // to arb, buy token0 and sell token1 to other pools
            // *not always the case
            arb_pools.push(pool_map);
        } else {
            // need to add logic to handle when
            // to < from
            continue;
        }
    }
    Some(arb_pools)
}

pub fn extract_sandwich_pools(
    state_diffs: &BTreeMap<Address, AccountDiff>,
    all_pools: &DashMap<Address, Pool>,
) -> Option<Vec<TradablePool>> {
    // capture all addresses that have a state change and are also a pool
    let touched_pools: Vec<Pool> = state_diffs
        .keys()
        .filter_map(|e| all_pools.get(e).map(|p| (*p.value())))
        .collect();

    // find direction of swap based on state diff (does weth have state changes?)
    let weth_state_diff = &state_diffs
        .get(&"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse::<H160>().unwrap())?
        .storage;

    let mut tradable_pools: Vec<TradablePool> = vec![];

    // find storage mapping index for each pool
    for pool in touched_pools {
        // find mapping storage location
        // reading balanceOf mapping given the address of the pool's address
        let storage_key = TxHash::from(ethers::utils::keccak256(abi::encode(&[
            abi::Token::Address(pool.address),
            abi::Token::Uint(U256::from(3)),
        ])));

        let is_weth_input = match weth_state_diff.get(&storage_key)? {
            Diff::Changed(c) => {
                let from = U256::from(c.from.to_fixed_bytes());
                let to = U256::from(c.to.to_fixed_bytes());
                to > from
            }
            // TODO: handle reverse direction
            _ => continue,
        };
        let rp = pool.to_rp();
        tradable_pools.push(TradablePool::new(rp, is_weth_input));
    }

    Some(tradable_pools)
}

//  Turn state_diffs into a new cache_db
//
// Arguments:
// * `state`: Statediffs used as values for creation of cache_db
// * `block_num`: Block number to get state from
// * `provider`: Websocket provider used to make rpc calls
//
// Returns:
// Ok(CacheDB<EmptyDB>): cacheDB created from statediffs, if no errors
// Err(ProviderError): If encountered error during rpc calls
pub async fn to_cache_db(
    state: &BTreeMap<Address, AccountDiff>,
    block_num: Option<BlockId>,
    provider: &Arc<Provider<Ws>>,
) -> Result<CacheDB<EmptyDB>, ProviderError> {
    let mut cache_db = CacheDB::new(EmptyDB::default());

    let mut futures = FuturesUnordered::new();

    for (address, acc_diff) in state.iter() {
        let nonce_provider = provider.clone();
        let balance_provider = provider.clone();
        let code_provider = provider.clone();

        let addy = *address;

        let future = async move {
            let nonce = nonce_provider
                .get_transaction_count(addy, block_num)
                .await?;

            let balance = balance_provider.get_balance(addy, block_num).await?;

            let code = code_provider.get_code(addy, block_num).await?;

            Ok::<(AccountDiff, Address, U256, U256, Bytes), ProviderError>((
                acc_diff.clone(),
                *address,
                nonce,
                balance,
                code,
            ))
        };

        futures.push(future);
    }

    while let Some(result) = futures.next().await {
        let (acc_diff, address, nonce, balance, code) = result?;
        let info = AccountInfo::new(balance.into(), nonce.as_u64(), Bytecode::new_raw(code.0));
        cache_db.insert_account_info(address.0.into(), info);

        acc_diff.storage.iter().for_each(|(slot, storage_diff)| {
            let slot_value: U256 = match storage_diff.to_owned() {
                Diff::Changed(v) => v.from.0.into(),
                Diff::Died(v) => v.0.into(),
                _ => {
                    // for cases Born and Same no need to touch
                    return;
                }
            };
            let slot: U256 = slot.0.into();
            cache_db
                .insert_account_storage(address.0.into(), slot.into(), slot_value.into())
                .unwrap();
        });
    }

    Ok(cache_db)
}
