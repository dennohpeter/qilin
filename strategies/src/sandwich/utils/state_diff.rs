// use crate::{prelude::Pool, utils};
use dashmap::DashMap;
use ethers::prelude::*;
use futures::stream::FuturesUnordered;
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{AccountInfo, Bytecode},
};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};


// ported directly from rusty sando
// https://github.com/mouseless-eth/rusty-sando
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
        Err(_) => {
            // should throw error here but guess None also works :<
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

    Some(merged_state_diffs)
}