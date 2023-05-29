// infra
// 1. stream tx update from new blocks from rusty's global backend
// 2. get state diff using trace_call_many
// 3. update pools
// 4. unify fork factory

//     let all_pools: Arc<DashMap<Address, Pool>> = Arc::new(DashMap::new());
use dashmap::DashMap;
use ethers::prelude::*;
use std::sync::Arc;
use crate::cfmm::{
    pool::Pool,
};
use std::error::Error;
use ethers::{
    types::BlockId,
};
use rusty::prelude::fork_factory::ForkFactory;

pub fn process_block_update(fork_factory: Arc<ForkFactory>, block: BlockId) -> Result<(), Box<dyn Error>> 
{

	let block_tx = fork_factory.get_full_block(block.clone())?;
	println!("block_tx: {:?}", block_tx);
	Ok(())
	// load all_pools
	// need to know how to get block update?
	// get state diff from meat of the tx in the updated block
	// - call get_from_txs
	// - call extract pool
	// create initailDB
	// get all Transaction from block
}

#[cfg(test)]
mod test {
	use futures_util::StreamExt;
	use revm::{
		db::{CacheDB, EmptyDB},
	};
	use rusty::prelude::fork_factory::ForkFactory;
	use crate::utils::{
		helpers::connect_to_network,
	};
	use std::env;
	use std::error::Error;
	use ethers::providers::{Middleware, Provider, Ws};
	use std::sync::Arc;
	use dotenv::dotenv;
	use ethers::{
		types::{
			BlockId,
			BlockNumber,
		},
	};
	use crate::state_manager::block_processor::process_block_update;

	#[tokio::test]
	async fn test_process_block_update() {
		dotenv();
		let _blast_key = env::var("BLAST_API_KEY").unwrap();
		let mainnet_blast_url = format!("wss://eth-mainnet.blastapi.io/{}", _blast_key);


		let result: Result<_, Box<dyn Error>> =
			connect_to_network(&mainnet_blast_url, "https://relay.flashbots.net", 1).await;

		let mut _ws_provider: Option<Arc<Provider<Ws>>> = None;
		match result {
			Ok((ws, _, _)) => {
				_ws_provider = Some(ws);
			}
			Err(e) => {
				println!("Error: {}", e);
			}
		}

		let ws_provider = _ws_provider.unwrap();
		let cache_db = CacheDB::new(EmptyDB::default());
		let fork_block = ws_provider.as_ref().get_block_number().await;
		let fork_block = fork_block.ok().map(|number| BlockId::Number(BlockNumber::Number(number)));
		let _fork_factory =
			Arc::new(ForkFactory::new_sandbox_factory(ws_provider.clone(), cache_db, fork_block));

		tokio::spawn(async move {
			let fork_factory = _fork_factory.clone();
			println!("fork_factory: {:?}", ws_provider.clone());

			loop {
				let ws_provider = ws_provider.clone();
				let mut block_stream = if let Ok(stream) = ws_provider.subscribe_blocks().await {
					stream
				} else {
					panic!("Failed to connect");
				};
				while let Some(new_block) = block_stream.next().await {
					println!("New block: {:?}", new_block);
					if let Some(number) = new_block.number {
						let fork_factory = fork_factory.clone();
						tokio::task::spawn_blocking(move || {
							println!("New block: {:?}", number);
							let block_num = number.into();
							process_block_update(fork_factory.clone(), block_num).unwrap();	
						});
					}
				}

			}
		});

	}

}


// pub async fn get_from_txs(
//     client: &Arc<Provider<Ws>>,
//     meats: &Vec<Transaction>,
//     block_num: BlockNumber,
// ) -> Option<BTreeMap<Address, AccountDiff>> {
//     // add statediff trace to each transaction
//     let req = meats
//         .iter()
//         .map(|tx| (tx, vec![TraceType::StateDiff]))
//         .collect();

//     let block_traces = match client.trace_call_many(req, Some(block_num)).await {
//         Ok(x) => x,
//         Err(e) => {
//             println!("Block Trace Error: {:?}", e);
//             return None;
//         }
//     };
//     println!("block_traces: {:?}", block_traces);

//     let mut merged_state_diffs = BTreeMap::new();

//     block_traces
//         .into_iter()
//         .flat_map(|bt| bt.state_diff.map(|sd| sd.0.into_iter()))
//         .flatten()
//         .for_each(|(address, account_diff)| {
//             match merged_state_diffs.entry(address) {
//                 Entry::Vacant(entry) => {
//                     entry.insert(account_diff);
//                 }
//                 Entry::Occupied(_) => {
//                     // Do nothing if the key already exists
//                     // we only care abt the starting state
//                 }
//             }
//         });

//     Some(merged_state_diffs)
// }