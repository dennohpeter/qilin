// use crate::{prelude::Pool, utils};
use dashmap::DashMap;
use ethers::prelude::*;
use fork_database::forked_db::ForkedDatabase;
use futures::stream::FuturesUnordered;
use log;
use parking_lot::RwLock;
use qilin_cfmms::pool::Pool;
use revm::primitives::{AccountInfo, Bytecode};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};

/// Holds pools that have the potential to be sandwiched
#[derive(Clone, Copy, Debug)]
pub struct SandwichablePool {
    pub pool: Pool,
    // Is swap direction zero to one?
    pub is_weth_input: bool,
}
impl SandwichablePool {
    pub fn new(pool: Pool, is_weth_input: bool) -> Self {
        Self {
            pool,
            is_weth_input,
        }
    }
}

// ported directly from rusty sando
// https://github.com/mouseless-eth/rusty-sando
// Extract state diffs from a given tx
//
// Arguments:
// * `client`: Websocket provider used for making rpc calls
// * `meats`: Vec of transactions to extract state diffs from
// * `block_num`: Block number of the block the txs are in
// * `test`: Use Anvil only rpc method - trace_transaction - during testing
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
        Ok(x) => {
            log::info!("got block traces: {:?}", x);
            x
        }
        Err(_) => {
            log::error!("error getting block traces");
            return None;
        }
    };

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

    // Log the state diffs
    for (address, account_diff) in &merged_state_diffs {
        log::info!("Address: {}", address);
        log::info!("AccountDiff: {:?}", account_diff);
    }

    Some(merged_state_diffs)
}

/// Decode statediff to produce Vec of pools interacted with
///
/// Arguments:
/// * `state_diffs`: BTreeMap of Address and AccountDiff
/// * `all_pools`: HashMap of Address and Pool
///
/// Returns:
/// Some(Vec<SandwichablePool>): Vec of pools that have been interacted with
/// None: If state_diffs is empty
pub fn extract_pools(
    state_diffs: &BTreeMap<Address, AccountDiff>,
    all_pools: &DashMap<Address, Pool>,
) -> Option<Vec<SandwichablePool>> {
    // capture all addresses that have a state change and are also a pool
    let touched_pools: Vec<Pool> = state_diffs
        .keys()
        .filter_map(|e| all_pools.get(e).map(|p| (*p.value()).clone()))
        .collect();

    // find direction of swap based on state diff (does weth have state changes?)
    let weth_state_diff = &state_diffs
        .get(
            &"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                .parse::<Address>()
                .unwrap(),
        )?
        .storage;

    let mut sandwichable_pools: Vec<SandwichablePool> = vec![];

    // find storage mapping index for each pool
    for pool in touched_pools {
        // find mapping storage location
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
            _ => continue,
        };
        sandwichable_pools.push(SandwichablePool::new(pool, is_weth_input));
    }

    Some(sandwichable_pools)
}

// Turn state_diffs into a new cache_db
//
// Arguments:
// * `state`: Statediffs used as values for creation of cache_db
// * `block_num`: Block number to get state from
// * `provider`: Websocket provider used to make rpc calls
//
// Returns:
// Ok(CacheDB<EmptyDB>): cacheDB created from statediffs, if no errors
// Err(ProviderError): If encountered error during rpc calls
pub async fn to_cache_db<'a, M>(
    state: &'a BTreeMap<Address, AccountDiff>,
    block_num: Option<BlockId>,
    provider: &'a Arc<M>,
    db: &'a Arc<RwLock<ForkedDatabase>>,
) -> Result<&'a Arc<RwLock<ForkedDatabase>>, ProviderError>
where
    M: Middleware + 'static,
    ProviderError: From<<M as Middleware>::Error>,
{
    let mut write_cache_db = db.write();
    let cache_db = write_cache_db.database_mut();

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

            let code = match code_provider.get_code(addy, block_num).await {
                Ok(c) => c,
                Err(_) => {
                    log::warn!("error getting code for address: {}", addy);
                    Bytes::new()
                }
            };

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
        match result {
            Ok((acc_diff, address, nonce, balance, code)) => {
                let info =
                    AccountInfo::new(balance.into(), nonce.as_u64(), Bytecode::new_raw(code.0));
                cache_db.insert_account_info(address.0.into(), info);

                acc_diff.storage.iter().for_each(|(slot, storage_diff)| {
                    let slot_value: U256 = match storage_diff.to_owned() {
                        Diff::Changed(v) => v.from.0.into(),
                        Diff::Died(v) => v.0.into(),
                        _ => {
                            return;
                        }
                    };
                    let slot: U256 = slot.0.into();
                    cache_db
                        .insert_account_storage(address.0.into(), slot.into(), slot_value.into())
                        .unwrap();
                });
            }
            Err(e) => {
                log::error!("Error occurred while retrieving state: {:?}", e);
                return Err(e);
            }
        }
    }

    Ok(db)
}
