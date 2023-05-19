use crate::cfmm::pool::Pool;
use crate::utils::constants::WETH_ADDRESS;
use dashmap::DashMap;
use ethers::types::H160;

use ethers::prelude::*;
// use futures::stream::FuturesUnordered;
// use revm::{
//     db::{CacheDB, EmptyDB},
//     primitives::{AccountInfo, Bytecode},
// };
use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};

#[derive(Debug, Clone, Copy)]
pub struct TradablePool {
    pub pool: Pool,
    pub is_weth_input: bool,
}

impl TradablePool {
    pub fn new(pool: Pool, is_weth_input: bool) -> Self {
        Self {
            pool,
            is_weth_input,
        }
    }
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
pub async fn get_from_txs(
    client: &Arc<Provider<Ws>>,
    meats: &Vec<Transaction>,
    block_num: BlockNumber,
) -> Option<BTreeMap<Address, AccountDiff>> {
    // add statediff trace to each transaction
    let req = meats
        .iter()
        .map(|tx| (tx, vec![TraceType::StateDiff]))
        .collect();

    let block_traces = match client.trace_call_many(req, Some(block_num)).await {
        Ok(x) => x,
        Err(e) => {
            println!("Error: {:?}", e);
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

pub fn extract_pools(
    state_diffs: &BTreeMap<Address, AccountDiff>,
    all_pools: &DashMap<Address, Pool>,
) -> Option<Vec<TradablePool>> {
    // capture all addresses that have a state change and are also a pool
    let touched_pools: Vec<Pool> = state_diffs
        .keys()
        .filter_map(|e| all_pools.get(e).map(|p| (*p.value()).clone()))
        .collect();

    // find direction of swap based on state diff (does weth have state changes?)
    let weth_state_diff = &state_diffs
        .get(&WETH_ADDRESS.parse::<H160>().unwrap())?
        .storage;

    let mut tradable_pools: Vec<TradablePool> = vec![];

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
        // tradable_pools.push(Tradable::new(pool, is_weth_input));
    }

    Some(tradable_pools)
}
