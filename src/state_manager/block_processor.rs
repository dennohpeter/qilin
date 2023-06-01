use super::state_diff::get_from_txs;
use crate::batch_requests;
use crate::cfmm::pool::Pool;
use dashmap::DashMap;
use ethers::{
    core::types::{Block, U256},
    prelude::*,
    providers::{Middleware, Provider, Ws},
    types::{BlockId, U64},
};
use rusty::prelude::fork_factory::ForkFactory;
use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

pub fn process_block_update(
    fork_factory: Arc<ForkFactory>,
    block: BlockId,
) -> Result<Vec<Transaction>, Box<dyn Error>> {
    // call the backend to get the full block
    let raw_block = fork_factory.get_full_block(block.clone())?;
    // turn payload into Vec<Transaction>
    let block_tx = raw_block.transactions;
    Ok(block_tx)
}

type PoolVariant = cfmms::dex::DexVariant;

pub async fn update_pools(
    client: &Arc<Provider<Ws>>,
    block_tx: &Vec<Transaction>,
    block_num: BlockNumber,
    all_pools: Arc<RwLock<DashMap<Address, Pool>>>,
) -> Option<Arc<RwLock<DashMap<Address, Pool>>>> {
    // get last block number to do the tracing
    let last_block_num = block_num.as_number()? - U64::from(1);

    // extract the state_diffs
    let state_diffs =
        // take the state of last block and trace diffs
        if let Some(state_diffs) = get_from_txs(
            &client.clone(),
            block_tx,
            ethers::types::BlockNumber::Number(last_block_num)
        ).await {
            state_diffs
        } else {
            return None;
        };

    let read_pool = all_pools.read().await;

    // get v2 and v3 pools that were touched
    let (mut touched_v3_pools, mut touched_v2_pools): (Vec<Pool>, Vec<Pool>) = state_diffs
        .keys()
        .filter_map(|e| read_pool.get(e).map(|p| (*p.value())))
        .partition(|pool| matches!(pool.pool_variant, PoolVariant::UniswapV3));
    drop(read_pool);

    // batch update v3 pools
    let v3_pool_slice = touched_v3_pools.as_mut_slice();
    batch_requests::uniswap_v3::get_pool_data_batch_request(v3_pool_slice, client.clone())
        .await
        .unwrap_or_else(|e| {
            println!("Error: {}", e);
        });
    let write_pool = all_pools.write().await;
    v3_pool_slice.to_vec().into_iter().for_each(|pool| {
        write_pool.insert(pool.address, pool);
    });
    drop(write_pool);
    println!("write_pool: {:?}", all_pools);

    // batch update v2 pools
    let v2_pool_slice = touched_v2_pools.as_mut_slice();
    batch_requests::uniswap_v2::get_pool_data_batch_request(v2_pool_slice, client.clone())
        .await
        .unwrap_or_else(|e| {
            println!("Error: {}", e);
        });
    let write_pool = all_pools.write().await;
    v2_pool_slice.to_vec().into_iter().for_each(|pool| {
        write_pool.insert(pool.address, pool);
    });
    drop(write_pool);

    Some(all_pools)
}

pub async fn block_update_thread(
    block_provider: Arc<Provider<Ws>>,
    block_clone: Arc<Mutex<Block<H256>>>,
    _arb_fork_factory: Arc<ForkFactory>,
    clone_all_pool: Arc<RwLock<DashMap<H160, Pool>>>,
) {
    loop {
        let mut block_stream = if let Ok(stream) = block_provider.subscribe_blocks().await {
            stream
        } else {
            panic!("Failed to connect");
        };

        while let Some(new_block) = block_stream.next().await {
            let mut locked_block = (*block_clone).lock().unwrap();
            *locked_block = new_block.clone();

            if let Some(number) = new_block.number {
                println!("New block: {:?}", number);

                let arb_fork_factory = _arb_fork_factory.clone();
                let all_pools = clone_all_pool.clone();
                let block_provider = block_provider.clone();

                let block_tx_shared: Arc<Mutex<Vec<Transaction>>> = Arc::new(Mutex::new(vec![]));
                let block_tx_shared_clone = block_tx_shared.clone();

                tokio::task::spawn_blocking(move || {
                    let block_num = number.into();
                    let block_tx =
                        process_block_update(arb_fork_factory.clone(), block_num).unwrap();
                    println!("block_tx: {:?}", block_tx);

                    *block_tx_shared_clone.lock().unwrap() = block_tx;
                });

                tokio::spawn(async move {
                    let block_tx = block_tx_shared.lock().unwrap().clone();
                    let _ = update_pools(
                        &block_provider.clone(),
                        &block_tx,
                        number.into(),
                        all_pools.clone(),
                    )
                    .await;
                });
            }
        }
    }
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
        let _blast_key =
            env::var("BLAST_API_KEY").expect("BLAST_API_KEY environment variable not set");
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
