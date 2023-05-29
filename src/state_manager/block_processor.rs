use crate::cfmm::pool::Pool;
use crate::utils::constants::WETH_ADDRESS;
use crate::utils::state_diff::get_from_txs;
use dashmap::DashMap;
use ethers::prelude::*;
use ethers::types::BlockId;
use rusty::prelude::fork_factory::ForkFactory;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;

pub fn process_block_update(
    fork_factory: Arc<ForkFactory>,
    block: BlockId,
) -> Result<Vec<Transaction>, Box<dyn Error>> {
    let raw_block = fork_factory.get_full_block(block.clone())?;

    // turn payload into Vec<Transaction>
    let block_tx = raw_block.transactions;
    println!("block_tx: {:?}", block_tx);
    Ok(block_tx)

    // create initailDB
}

pub async fn update_pools(
    client: &Arc<Provider<Ws>>,
    block_tx: &Vec<Transaction>,
    block_num: BlockNumber,
    all_pools: Arc<RwLock<DashMap<Address, Pool>>>,
) -> Option<Arc<RwLock<DashMap<Address, Pool>>>> {
    let state_diffs =
        if let Some(state_diffs) = get_from_txs(&client.clone(), block_tx, block_num).await {
            state_diffs
        } else {
            return None;
        };

    let weth_state_diff = &state_diffs
        .get(&WETH_ADDRESS.parse::<H160>().unwrap())?
        .storage;

    let read_pool = all_pools.read().await;

    let touched_pools: Vec<Pool> = state_diffs
        .keys()
        .filter_map(|e| read_pool.get(e).map(|p| (*p.value())))
        .collect();

    drop(read_pool);

    let write_pool = all_pools.write().await;

    for pool in touched_pools.iter() {
        let pool_address = pool.address;
        if let Some(mut pool_entry) = write_pool.get_mut(&pool_address) {
            let pool = pool_entry.value_mut();
            pool.update_pool_state(client.clone()).await;
        }
    }
    drop(write_pool);

    Some(all_pools)
}

#[cfg(test)]
mod test {
    use crate::state_manager::block_processor::process_block_update;
    use crate::utils::helpers::connect_to_network;
    use ethers::providers::{Middleware, Provider, Ws};
    use ethers::types::{BlockId, BlockNumber};
    use futures_util::StreamExt;
    use revm::db::{CacheDB, EmptyDB};
    use rusty::prelude::fork_factory::ForkFactory;
    use std::env;
    use std::error::Error;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_process_block_update() {
        // dotenv();
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
        let fork_block = fork_block
            .ok()
            .map(|number| BlockId::Number(BlockNumber::Number(number)));
        let _fork_factory = Arc::new(ForkFactory::new_sandbox_factory(
            ws_provider.clone(),
            cache_db,
            fork_block,
        ));

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
