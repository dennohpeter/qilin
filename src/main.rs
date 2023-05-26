pub mod abigen;
pub mod bindings;
pub mod cfmm;
pub mod uni_math;
pub mod utils;

use crate::bindings::weth::weth_contract;
use crate::cfmm::{
    dex,
    pool::{Pool, PoolVariant},
};
use crate::utils::constants::{UNISWAP_V2_FACTORY, UNISWAP_V3_FACTORY, USDC_ADDRESS, WETH_ADDRESS};
use crate::utils::{
    base_fee_helper,
    helpers::{connect_to_network, generate_abigen},
    relayer,
    serialization::{read_pool_data, write_pool_data},
    state_diff,
};
use cfmms::pool::{UniswapV2Pool, UniswapV3Pool};
use clap::{arg, Command};
use dashmap::DashMap;
use dotenv::dotenv;
use ethers::core::types::{Block, U256};
use ethers::prelude::*;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::signers::LocalWallet;
use ethers_flashbots::FlashbotsMiddleware;
use eyre::Result;
use log;
use rusty::prelude::{fork_factory::ForkFactory, make_sandwich, sandwich_types::RawIngredients};
use std::env;
use std::error::Error;
use std::sync::{Arc, Mutex};
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv()?;
    let arg: Vec<String> = env::args().collect();
    generate_abigen(arg).await?;

    // data collection
    let _etherscan_key = env::var("ETHERSCAN_API_KEY").unwrap();

    let llama_url = format!("wss://eth.llamarpc.com");

    let block_provider = Provider::<Ws>::connect(llama_url).await?;

    // bundle signing
    let _bundle_signer = env::var("FLASHBOTS_IDENTIFIER")?;
    let bundle_signer = _bundle_signer.parse::<LocalWallet>()?;

    let _wallet = env::var("FLASHBOTS_SIGNER").clone().unwrap();
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

    let ws_provider = _ws_provider.unwrap();
    let middleware_url = _middleware_url.unwrap();
    let chain_id = _chain_id.unwrap();

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

    let wallet_weth_balance = Arc::new(Mutex::new(weth_balance));

    let block: Arc<Mutex<Option<Block<H256>>>> = Arc::new(Mutex::new(None));
    let block_clone = Arc::clone(&block);

    tokio::spawn(async move {
        loop {
            let mut block_stream = if let Ok(stream) = block_provider.subscribe_blocks().await {
                stream
            } else {
                panic!("Failed to connect");
            };

            while let Some(new_block) = block_stream.next().await {
                let mut locked_block = (*block_clone).lock().unwrap();
                *locked_block = Some(new_block);
                println!(
                    "Block Number: {:?}",
                    locked_block
                        .as_ref()
                        .map(|blk| blk.number)
                        .unwrap()
                        .unwrap()
                );
                println!(
                    "Block TimeStamp: {:?}",
                    locked_block.as_ref().map(|blk| blk.timestamp).unwrap()
                );
            }
        }
    });

    let all_pools: Arc<DashMap<Address, Pool>> = Arc::new(DashMap::new());

    match read_pool_data() {
        Ok(dmap) => {
            for item in dmap.iter() {
                let (key, value) = item.pair();
                // if let Ok(address) = key.try_into() {
                all_pools.insert(*key, *value);
                // }
            }
        }
        Err(e) => {
            println!("Error reading pool data: {}", e);
            print!("Pulling pool data......");
            let dexes = vec![
                //UniswapV2
                dex::Dex::new(
                    UNISWAP_V2_FACTORY.parse::<H160>()?,
                    PoolVariant::UniswapV2,
                    17330000,
                ),
                // Add UniswapV3
                dex::Dex::new(
                    UNISWAP_V3_FACTORY.parse::<H160>()?,
                    PoolVariant::UniswapV3,
                    17330000,
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

            for pool in synced_pools {
                all_pools.insert(pool.address, pool);
            }

            let _ = write_pool_data(&all_pools);
        }
    }

    let mut mempool_stream = ws_provider.subscribe_pending_txs().await?;
    println!("Subscribed to pending txs");

    while let Some(tx_hash) = mempool_stream.next().await {
        let msg = ws_provider.get_transaction(tx_hash).await?;

        let mut data = msg.clone().unwrap_or(Transaction::default());
        let mut next_block_base_fee: Option<U256> = None;

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

        // if tx has statediff on pool addr then record it in `mev_pools`
        let mev_pools =
            if let Some(mevP) = utils::state_diff::extract_pools(&state_diffs, &all_pools) {
                mevP
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
        let fork_factory =
            ForkFactory::new_sandbox_factory(temp_provider.clone(), initial_db, fork_block);

        // create evm simulation handler by setting up `fork_factory`
        let initial_db = utils::state_diff::to_cache_db(&state_diffs, fork_block, &ws_provider)
            .await
            .unwrap();
        let fork_factory =
            ForkFactory::new_sandbox_factory(ws_provider.clone(), initial_db, fork_block);

        // search for opportunities in all pools that the tx touches (concurrently)
        for mev_pool in mev_pools {
            // if !mev_pool.is_weth_input {
            //     // enhancement: increase opportunities by handling swaps in pools with stables
            //     log::info!("{:?} [weth_is_output]", victim_tx.hash);
            //     continue;
            // } else {
            //     log::info!(
            //         "{}",
            //         format!("{:?} [weth_is_input]", victim_tx.hash).green()
            //     );
            // }

            // prepare variables for new thread
            let data = data.clone();
            let mev_pool = mev_pool.clone();
            let mut fork_factory = fork_factory.clone();
            // let block_oracle = block_oracle.clone();
            // let sandwich_maker = self.sandwich_maker.clone();
            let state_diffs = state_diffs.clone();

            tokio::spawn(async move {
                // enhancement: increase opportunities by handling swaps in pools with stables
                let input_token = WETH_ADDRESS.parse::<H160>().unwrap();
                let victim_hash = data.hash;

                // variables used when searching for opportunity
                // let raw_ingredients = if let Ok(data) = RawIngredients::new(
                //     &mev_pool.pool,
                //     vec![data],
                //     input_token,
                //     state_diffs,
                // )
                // .await
                // {
                //     data
                // } else {
                //     log::error!("Failed to create raw ingredients for: {:?}", &victim_hash);
                //     return;
                // };

                // find optimal input to sandwich tx
                // let mut optimal_sandwich = match make_sandwich::create_optimal_sandwich(
                //     &raw_ingredients,
                //     sandwich_balance,
                //     &block_oracle.next_block,
                //     &mut fork_factory,
                //     &sandwich_maker,
                // )
                // .await
                // {
                //     Ok(optimal) => optimal,
                //     Err(e) => {
                //         log::info!(
                //             "{}",
                //             format!("{:?} sim failed due to {:?}", &victim_hash, e)
                //         );
                //         return;
                //     }
                // };

                // let optimal_sandwich = optimal_sandwich.clone();
                // let optimal_sandwich_two = optimal_sandwich.clone();
                // let sandwich_maker = sandwich_maker.clone();
            });
        }
    }

    Ok(())
}
