use qilin_cfmms::{
    dex,
    dex::PairSyncError,
    pool::{Pool, PoolVariant},
};
use crate::utils::constants::{UNISWAP_V2_FACTORY, UNISWAP_V3_FACTORY, WETH_ADDRESS};
use crate::utils::{
    helpers::{connect_to_network, generate_abigen},
    serialization::{read_pool_data, write_pool_data},
};
use anyhow::Result;
use clap::{arg, Command};
use dashmap::DashMap;
use dotenv;
use ethers::{
    prelude::*,
    providers::{Middleware, Provider, Ws},
    signers::LocalWallet,
    types::{H160, U256},
};
use ethers_flashbots::FlashbotsMiddleware;
use log;
use parking_lot::RwLock;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::{env, str::FromStr};
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum SetupError {
    #[error("Failed to load environment variable")]
    LoadingEnvironmentVariableError(#[from] std::env::VarError),
    #[error("Failed to connect to relay")]
    RelayConnectionError,
    #[error("Parsing error")]
    ParsingError(#[from] std::num::ParseIntError),
    #[error("Failed to sync pairs")]
    PairSyncError(#[from] PairSyncError),
}

/// Load the envitonment variables, sync pool states, and initate the backend database
pub async fn setup() -> Result<
    (
        SignerMiddleware<FlashbotsMiddleware<Arc<Provider<Ws>>, LocalWallet>, LocalWallet>,
        Arc<RwLock<DashMap<Address, Pool>>>,
        Arc<DashMap<H160, Vec<Pool>>>,
    ),
    SetupError,
> {
    dotenv::dotenv().ok();

    let arg: Vec<String> = env::args().collect();

    generate_abigen(arg).await.unwrap_or_else(|e| {
        log::error!("Error: {}", e);
        std::process::exit(1);
    });

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
            let _blast_key = env::var("BLAST_API_KEY").unwrap_or_else(|e| {
                log::error!("Error: {}", e);
                return e.to_string();
            });

            let mainnet_blast_url = format!("wss://eth-mainnet.blastapi.io/{}", _blast_key);

            let result =
                connect_to_network(&mainnet_blast_url, "wss://relay.flashbots.net", 1).await;

            match result {
                Ok((ws, mw, ci)) => {
                    _ws_provider = Some(ws);
                    _middleware_url = Some(mw);
                    _chain_id = Some(ci);
                }
                Err(e) => {
                    log::error!("Error: {}", e);
                }
            }
        }
        Some(network) if network == "goerli" => {
            log::info!("Running on goerli");
            let _blast_key_goerli = env::var("BLAST_API_GOERLI").unwrap_or_else(|e| {
                return e.to_string();
            });
            let goerli_blast_url = format!("wss://eth-goerli.blastapi.io/{}", _blast_key_goerli);

            let result =
                connect_to_network(&goerli_blast_url, "https://relay-goerli.flashbots.net", 5)
                    .await;

            match result {
                Ok((ws, mw, ci)) => {
                    _ws_provider = Some(ws);
                    _middleware_url = Some(mw);
                    _chain_id = Some(ci);
                }
                Err(e) => {
                    log::error!("Error: {}", e);
                }
            }
        }
        Some(_) => {
            log::error!("Invalid argument. Please use 'mainnet' or 'goerli'");
            std::process::exit(1);
        }
        None => {
            log::info!("Running on mainnet");
            let _blast_key = env::var("BLAST_API_KEY").unwrap_or_else(|e| {
                return e.to_string();
            });
            let mainnet_blast_url = format!("wss://eth-mainnet.blastapi.io/{}", _blast_key);

            let result =
                connect_to_network(&mainnet_blast_url, "https://relay.flashbots.net", 1).await;

            match result {
                Ok((ws, mw, ci)) => {
                    _ws_provider = Some(ws);
                    _middleware_url = Some(mw);
                    _chain_id = Some(ci);
                }
                Err(e) => {
                    log::error!("Error: {}", e);
                }
            }
        }
    }

    // Load environment variables
    let _etherscan_key = env::var("ETHERSCAN_API_KEY").unwrap_or_else(|e| {
        SetupError::LoadingEnvironmentVariableError(e);
        panic!("Please set the ETHERSCAN_API_KEY environment variable");
    });

    let _bundle_signer = env::var("FLASHBOTS_IDENTIFIER").unwrap_or_else(|e| {
        SetupError::LoadingEnvironmentVariableError(e);
        panic!("Please set the FLASHBOTS_IDENTIFIER environment variable");
    });

    let bundle_signer = _bundle_signer.parse::<LocalWallet>().unwrap_or_else(|e| {
        log::error!("Error: {}", e);
        panic!("Could not parse FLASHBOTS_IDENTIFIER");
    });

    let _wallet = env::var("FLASHBOTS_SIGNER").unwrap_or_else(|e| {
        SetupError::LoadingEnvironmentVariableError(e);
        panic!("Please set the FLASHBOTS_SIGNER environment variable");
    });

    // let addr = env::var("SANDWICH_CONTRACT").unwrap_or_else(|e| {
    // 	SetupError::LoadingEnvironmentVariableError(e);
    // 	panic!("Please set the SANDWICH_CONTRACT environment variable");
    // });
    // let sando_addr = H160::from_str(&addr).expect("Failed to parse \"SANDWICH_CONTRACT\"");

    // setup wallet, provider, and flashbot client
    let wallet = _wallet.parse::<LocalWallet>().unwrap();
    let ws_provider = _ws_provider.unwrap();
    let middleware_url = _middleware_url.unwrap();
    let _chain_id = _chain_id.unwrap();
    let mut flashbot_middleware = FlashbotsMiddleware::new(
        ws_provider.clone(),
        middleware_url.clone(),
        bundle_signer.clone(),
    );

    flashbot_middleware.set_simulation_relay(middleware_url.clone(), bundle_signer.clone());
    let flashbot_client = SignerMiddleware::new(flashbot_middleware, wallet);

    // let _weth_contract =
    // 	weth_contract::weth::new(WETH_ADDRESS.parse::<H160>()?, Arc::clone(&flashbot_client));
    // let weth_balance = _weth_contract.balance_of(wallet.address()).call().await?;
    // let _wallet_weth_balance = Arc::new(Mutex::new(weth_balance));

    // sync pools
    let (all_pools, hash_pools) = load_pools(flashbot_client.inner().inner().clone()).await?;

    // TODO: setup the global backedn

    Ok((flashbot_client, all_pools, hash_pools))
}

async fn load_pools(
    provider: Arc<Provider<Ws>>,
) -> Result<
    (
        Arc<RwLock<DashMap<Address, Pool>>>,
        Arc<DashMap<H160, Vec<Pool>>>,
    ),
    SetupError,
> {
    // load pool data from json file
    let all_pools = Arc::new(RwLock::new(DashMap::new()));
    // same as above but key is hash of token0 and token1 addresses for faster lookup
    let hash_addr_pools: Arc<DashMap<H160, Vec<Pool>>> = Arc::new(DashMap::new());

    match read_pool_data(provider.clone()).await {
        Ok((dmap, pdmap)) => {
            let write_lock = all_pools.write();
            for item in dmap.iter() {
                let (key, value) = item.pair();
                write_lock.insert(*key, *value);
            }

            // when read from json, hash_addr_pools' values are never updated
            for item in pdmap.iter() {
                let (key, value) = item.pair();
                let pool_vec = value.clone();
                hash_addr_pools.insert(*key, (*pool_vec).to_vec());
            }
        }
        Err(e) => {
            log::info!("Error reading pool data: {}", e);
            log::info!("Pulling pool data......");

            let dexes = vec![
                // UniswapV2
                dex::Dex::new(
                    UNISWAP_V2_FACTORY
                        .parse::<H160>()
                        .expect("Failed to parse UNISWAP_V2_FACTORY"),
                    PoolVariant::UniswapV2,
                    10000835,
                ),
                // UniswapV3
                dex::Dex::new(
                    UNISWAP_V3_FACTORY
                        .parse::<H160>()
                        .expect("Failed to parse UNISWAP_V3_FACTORY"),
                    PoolVariant::UniswapV3,
                    12369621,
                ),
            ];

            let current_block = provider
                .as_ref()
                .get_block_number()
                .await
                .expect("Failed to get block number");

            let synced_pools = dex::sync_dex(
                dexes.clone(),
                &Arc::clone(&provider),
                current_block,
                None,
                2, //throttled for 2 secs
            )
            .await
            .expect("Failed to sync dexes");

            let mut hasher = DefaultHasher::new();
            let mut token0;
            let mut token1;

            let write_lock = all_pools.write();
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
            let read_lock = all_pools.read();

            let _ = write_pool_data(&read_lock, false);
            let _ = write_pool_data(&hash_addr_pools, true);
        }
    }
    Ok((all_pools, hash_addr_pools))
}
