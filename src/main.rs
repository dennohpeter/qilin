pub mod abigen;
pub mod batch_requests;
pub mod bindings;
pub mod cfmm;
pub mod errors;
pub mod state_manager;
pub mod uni_math;
pub mod utils;

use crate::bindings::weth::weth_contract;
use crate::cfmm::{
    dex,
    pool::{Pool, PoolVariant},
};
use crate::utils::constants::{UNISWAP_V2_FACTORY, UNISWAP_V3_FACTORY, WETH_ADDRESS};
use crate::utils::{
    base_fee_helper,
    helpers::{connect_to_network, generate_abigen},
    serialization::{read_pool_data, write_pool_data},
};
use revm::db::{CacheDB, EmptyDB};
use tokio::sync::RwLock;

use crate::state_manager::block_processor::{process_block_update, update_pools};
use clap::{arg, Command};
use dashmap::DashMap;
use dotenv::dotenv;
use ethers::core::types::{Block, U256};
use ethers::prelude::*;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::signers::LocalWallet;
use ethers_flashbots::FlashbotsMiddleware;
use eyre::Result;

use rusty::prelude::fork_factory::ForkFactory;

use std::collections::hash_map::DefaultHasher;
use std::env;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv()?;
    let arg: Vec<String> = env::args().collect();
    generate_abigen(arg).await?;

    // data collection
    let _etherscan_key = env::var("ETHERSCAN_API_KEY").unwrap();

    // bundle signing
    let _bundle_signer = env::var("FLASHBOTS_IDENTIFIER")?;
    let bundle_signer = _bundle_signer.parse::<LocalWallet>()?;

    let _wallet = env::var("FLASHBOTS_SIGNER").unwrap();
    let wallet = _wallet.parse::<LocalWallet>().unwrap();

    let matches = Command::new("Qi(氣) Bot")
        .version("1.0")
        .about("A general purpose MEV bot")
        .arg(arg!([NETWORK_NAME]).required(false))
        .get_matches();

    let mut _ws_provider: Option<Arc<Provider<Ws>>> = None;
    let mut _middleware_url: Option<Url> = None;
    let mut _chain_id: Option<i32> = None;

    match matches.get_one::<String>("NETWORK_NAME") {
        Some(network) if network == "mainnet" => {
            println!("Running on mainnet");
            let _blast_key = env::var("BLAST_API_KEY").unwrap();
            let mainnet_blast_url = format!("wss://eth-mainnet.blastapi.io/{}", _blast_key);

            let result: Result<_, Box<dyn Error>> =
                connect_to_network(&mainnet_blast_url, "https://relay.flashbots.net", 1).await;

            match result {
                Ok((ws, mw, ci)) => {
                    _ws_provider = Some(ws);
                    _middleware_url = Some(mw);
                    _chain_id = Some(ci);
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
        Some(network) if network == "goerli" => {
            println!("Running on goerli");
            let _blast_key_goerli = env::var("BLAST_API_GOERLI").unwrap();
            let goerli_blast_url = format!("wss://eth-goerli.blastapi.io/{}", _blast_key_goerli);

            let result: Result<_, Box<dyn Error>> =
                connect_to_network(&goerli_blast_url, "https://relay-goerli.flashbots.net", 5)
                    .await;

            match result {
                Ok((ws, mw, ci)) => {
                    _ws_provider = Some(ws);
                    _middleware_url = Some(mw);
                    _chain_id = Some(ci);
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
        Some(_) => {
            println!("Invalid argument. Please use 'mainnet' or 'goerli'");
        }
        None => {
            println!("Running on mainnet");
            let _blast_key = env::var("BLAST_API_KEY").unwrap();
            let mainnet_blast_url = format!("wss://eth-mainnet.blastapi.io/{}", _blast_key);

            let result: Result<_, Box<dyn Error>> =
                connect_to_network(&mainnet_blast_url, "https://relay.flashbots.net", 1).await;

            match result {
                Ok((ws, mw, ci)) => {
                    _ws_provider = Some(ws);
                    _middleware_url = Some(mw);
                    _chain_id = Some(ci);
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
    }

    pub type AllPool = Arc<RwLock<DashMap<Address, Pool>>>;

    let ws_provider = _ws_provider.unwrap();
    let middleware_url = _middleware_url.unwrap();
    let _chain_id = _chain_id.unwrap();

    let mut flashbot_middleware = FlashbotsMiddleware::new(
        ws_provider.clone(),
        middleware_url.clone(),
        bundle_signer.clone(),
    );

    flashbot_middleware.set_simulation_relay(middleware_url.clone(), bundle_signer.clone());

    let flashbot_client = Arc::new(SignerMiddleware::new(flashbot_middleware, wallet.clone()));

    let _weth_contract =
        weth_contract::weth::new(WETH_ADDRESS.parse::<H160>()?, Arc::clone(&flashbot_client));
    let weth_balance = _weth_contract.balance_of(wallet.address()).call().await?;

    let _wallet_weth_balance = Arc::new(Mutex::new(weth_balance));

    // TODO: delete the option
    let block: Arc<Mutex<Option<Block<H256>>>> = Arc::new(Mutex::new(None));
    let block_clone = Arc::clone(&block);

    // let sandwich_maker = Arc::new(SandwichMaker::new().await);
    // let _block_oracle = BlockOracle::new(&ws_provider.clone()).await.unwrap();
    // let mut block_oracle = Arc::new(RwLock::new(_block_oracle));

    let all_pools: AllPool = Arc::new(RwLock::new(DashMap::new()));
    // same as above but key is hash of token0 and token1 addresses for faster lookup
    let hash_addr_pools: Arc<DashMap<H160, Vec<Pool>>> = Arc::new(DashMap::new());

    let cache_db = CacheDB::new(EmptyDB::default());
    let fork_block = ws_provider.as_ref().get_block_number().await;
    let fork_block = fork_block
        .ok()
        .map(|number| BlockId::Number(BlockNumber::Number(number)));

    let _arb_fork_factory = Arc::new(ForkFactory::new_sandbox_factory(
        ws_provider.clone(),
        cache_db,
        fork_block,
    ));

    let block_provider = ws_provider.clone();
    let clone_all_pool = all_pools.clone();

    // TODO: move this to block processor
    tokio::spawn(async move {
        let clone_arb_fork_factory = _arb_fork_factory.clone();
        let clone_all_pool = clone_all_pool.clone();

        loop {
            let mut block_stream = if let Ok(stream) = block_provider.subscribe_blocks().await {
                stream
            } else {
                panic!("Failed to connect");
            };
            while let Some(new_block) = block_stream.next().await {
                let mut locked_block = (*block_clone).lock().unwrap();
                *locked_block = Some(new_block.clone());
                if let Some(number) = new_block.number {
                    println!("New block number: {:?}", number);
                    let arb_fork_factory = clone_arb_fork_factory.clone();
                    let all_pools = clone_all_pool.clone();
                    let block_provider = block_provider.clone();

                    let block_tx_shared: Arc<Mutex<Vec<Transaction>>> =
                        Arc::new(Mutex::new(vec![]));
                    let block_tx_shared_clone = block_tx_shared.clone();

                    tokio::task::spawn_blocking(move || {
                        let block_num = number.into();
                        let block_tx =
                            process_block_update(arb_fork_factory.clone(), block_num).unwrap();

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
    });
    // let bot_state = BotState::new(inception_block, &ws_provider.clone()).await?;
    // let bot_state = Arc::new(bot_state);

    match read_pool_data(ws_provider.clone()).await {
        Ok((dmap, pdmap)) => {
            let write_lock = all_pools.write().await;
            for item in dmap.iter() {
                let (key, value) = item.pair();
                write_lock.insert(*key, *value);
            }
            for item in pdmap.iter() {
                let (key, value) = item.pair();
                let pool_vec = value.clone();
                hash_addr_pools.insert(*key, (*pool_vec).to_vec());
            }
        }
        Err(e) => {
            println!("Error reading pool data: {}", e);
            print!("Pulling pool data......");
            let dexes = vec![
                // UniswapV2
                dex::Dex::new(
                    UNISWAP_V2_FACTORY.parse::<H160>()?,
                    PoolVariant::UniswapV2,
                    10000835,
                ),
                // UniswapV3
                dex::Dex::new(
                    UNISWAP_V3_FACTORY.parse::<H160>()?,
                    PoolVariant::UniswapV3,
                    12369621,
                ),
            ];

            let current_block = ws_provider.as_ref().get_block_number().await?;

            println!("Current Block: {:?}", current_block);
            let synced_pools = dex::sync_dex(
                dexes.clone(),
                &Arc::clone(&ws_provider),
                current_block,
                None,
                2, //throttled for 2 secs
            )
            .await?;

            let mut hasher = DefaultHasher::new();
            let mut token0;
            let mut token1;

            let write_lock = all_pools.write().await;
            for pool in synced_pools {
                write_lock.insert(pool.address, pool);

                token0 = pool.token_0;
                token1 = pool.token_1;
                token0.hash(&mut hasher);
                token1.hash(&mut hasher);
                let hash = hasher.finish();

                hash_addr_pools
                    .entry(H160::from_low_u64_be(hash))
                    .and_modify(|pools| pools.push(pool))
                    .or_insert_with(|| vec![pool]);
            }
            let read_lock = all_pools.read().await;

            let _ = write_pool_data(&read_lock, false);
            let _ = write_pool_data(&hash_addr_pools, true);
        }
    }

    // oracles::start_block_oracle(&mut block_oracle);

    let mut mempool_stream = ws_provider.subscribe_pending_txs().await?;
    println!("Subscribed to pending txs");

    while let Some(tx_hash) = mempool_stream.next().await {
        let msg = ws_provider.get_transaction(tx_hash).await?;

        let mut data = msg.clone().unwrap_or(Transaction::default());
        let mut next_block_base_fee: Option<U256> = None;

        // let bo = {
        //     let read_lock = block_oracle.read().await;
        //     (*read_lock).clone()
        // };

        // let sandwich_balance = {
        //     let read_lock = bot_state.weth_balance.read().await;
        //     (*read_lock).clone()
        // };

        match (*block).lock() {
            Ok(blk) => match blk.as_ref() {
                Some(blk_ref) => {
                    next_block_base_fee = Some(base_fee_helper::calculate_next_block_base_fee(
                        blk_ref.clone(),
                    ));
                }
                None => {
                    println!("No block available");
                }
            },
            Err(_) => {
                println!("Mutex currently taken");
            }
        }

        if data.max_fee_per_gas.unwrap_or(U256::zero()) < next_block_base_fee.unwrap() {
            format!("{:?} max fee per gas < next base fee", data.hash);
            continue;
        }

        if let Ok(from) = data.recover_from() {
            data.from = from;
        } else {
            format!("{:?} ecdsa recovery failed", data.hash);
            continue;
        };

        let state_diffs = if let Some(state_diff) = utils::state_diff::get_from_txs(
            &Arc::clone(&ws_provider),
            &vec![data.clone()],
            if let Some(blk) = (*block).lock().unwrap().as_ref() {
                BlockNumber::Number(blk.number.unwrap())
            } else {
                BlockNumber::Latest
            },
        )
        .await
        {
            state_diff
        } else {
            format!("{:?}", data.hash);
            continue;
        };
        // arb
        // let arb_read_lock = all_pools.read().await;
        // let _mev_pools = if let Some(mev_p) =
        //     utils::state_diff::extract_sandwich_pools(&state_diffs, &arb_read_lock)
        // {
        //     mev_p
        // } else {
        //     continue;
        // };

        // sandwich
        let sand_read_lock = all_pools.read().await;
        let _mev_pools = if let Some(mev_p) =
            utils::state_diff::extract_sandwich_pools(&state_diffs, &sand_read_lock)
        {
            mev_p
        } else {
            continue;
        };
        let fork_block = Some(BlockId::Number(BlockNumber::Number(
            ws_provider.get_block_number().await? + 1,
        )));
        let temp_provider = Arc::clone(&ws_provider);
        let initial_db = utils::state_diff::to_cache_db(&state_diffs, fork_block, &temp_provider)
            .await
            .unwrap();
        let _fork_factory =
            ForkFactory::new_sandbox_factory(temp_provider.clone(), initial_db, fork_block);

        // search for opportunities in all pools that the tx touches (concurrently)
        // for mev_pool in mev_pools {
        //     if !mev_pool.is_weth_input {
        //         // enhancement: increase opportunities by handling swaps in pools with stables
        //         log::info!("{:?} [weth_is_output]", data.hash);
        //         continue;
        //     } else {
        //         log::info!(
        //             "{}",
        //             format!("{:?} [weth_is_input]", data.hash)
        //         );
        //     }

        //     // prepare variables for new thread
        //     let data = data.clone();
        //     let mev_pool = mev_pool.clone();
        //     let mut fork_factory = fork_factory.clone();
        //     let bo = bo.clone();
        //     let sandwich_maker = sandwich_maker.clone();
        //     let state_diffs = state_diffs.clone();

        //     tokio::spawn(async move {
        //         // enhancement: increase opportunities by handling swaps in pools with stables
        //         let input_token = WETH_ADDRESS.parse::<H160>().unwrap();
        //         let _hash = data.hash;

        //         // variables used when searching for opportunity
        //         let raw_ingredients = if let Ok(data) = RawIngredients::new(
        //             &mev_pool.pool,
        //             vec![data],
        //             input_token,
        //             state_diffs,
        //         )
        //         .await
        //         {
        //             data
        //         } else {
        //             log::error!("Failed to create raw ingredients for: {:?}", &_hash);
        //             return;
        //         };

        //         // find optimal input to sandwich tx
        //         let mut optimal_sandwich = match make_sandwich::create_optimal_sandwich(
        //             &raw_ingredients,
        //             sandwich_balance,
        //             &bo.next_block,
        //             &mut fork_factory,
        //             &sandwich_maker,
        //         )
        //         .await
        //         {
        //             Ok(optimal) => optimal,
        //             Err(e) => {
        //                 log::info!(
        //                     "{}",
        //                     format!("{:?} sim failed due to {:?}", &_hash, e)
        //                 );
        //                 return;
        //             }
        //         };

        //         // let optimal_sandwich_two = optimal_sandwich.clone();
        //         // let sandwich_maker = sandwich_maker.clone();
        //     });
        // }
    }

    Ok(())
}
