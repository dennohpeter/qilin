use ethers::prelude::*;
use ethers::prelude::{AbiError, ContractError};
use ethers::providers::{Provider, ProviderError, Ws};
use eyre::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::{Arc, Mutex};
use std::{
    thread::sleep,
    time::{Duration, SystemTime},
};
use thiserror::Error;
use tokio::task::JoinError;

use super::pool::{Pool, PoolVariant};
use super::bindings::{
    uniswap_v2_factory::uniswap_v2_factory_contract,
    uniswap_v3_factory::uniswap_v3_factory_contract,
};

#[derive(Clone, Copy)]
pub struct RequestThrottle {
    enabled: bool,
    last_request_timestamp: SystemTime,
    requests_per_second_limit: usize,
    requests_per_second: usize,
}

impl RequestThrottle {
    pub fn new(requests_per_second_limit: usize) -> RequestThrottle {
        if requests_per_second_limit > 0 {
            RequestThrottle {
                enabled: true,
                last_request_timestamp: SystemTime::now(),
                requests_per_second_limit,
                requests_per_second: 0,
            }
        } else {
            RequestThrottle {
                enabled: false,
                last_request_timestamp: SystemTime::now(),
                requests_per_second_limit,
                requests_per_second: 0,
            }
        }
    }

    pub fn increment_or_sleep(&mut self, inc: usize, interval: u128) {
        let time_elapsed = self
            .last_request_timestamp
            .elapsed()
            .expect("Could not get time elapsed from last request timestamp")
            .as_millis();

        if self.enabled && time_elapsed < interval {
            if self.requests_per_second >= self.requests_per_second_limit {
                sleep(Duration::from_secs(2));
                self.requests_per_second = 0;
                self.last_request_timestamp = SystemTime::now();
            } else {
                self.requests_per_second += inc;
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct Dex {
    pub factory_address: Address,
    pub pool_variant: PoolVariant,
    pub creation_block: BlockNumber,
}

impl Dex {
    // Creates a new dex instance
    pub fn new(factory_address: H160, pool_variant: PoolVariant, creation_block: u64) -> Dex {
        Dex {
            factory_address,
            pool_variant,
            creation_block: BlockNumber::Number(creation_block.into()),
        }
    }

    // Parse logs and extract pools
    pub async fn new_pool_from_event(
        &self,
        log: Log,
        provider: Arc<Provider<Ws>>,
        req_throttle: Arc<Mutex<RequestThrottle>>,
    ) -> Option<Pool> {
        match self.pool_variant {
            PoolVariant::UniswapV2 => {
                let uniswap_v2_factory = uniswap_v2_factory_contract::uniswap_v2_factory::new(
                    self.factory_address,
                    provider.clone(),
                );
                let (token_0, token_1, address, _) = if let Ok(pair) = uniswap_v2_factory
                    .decode_event::<(Address, Address, Address, U256)>(
                        "PairCreated",
                        log.topics,
                        log.data,
                    ) {
                    pair
                } else {
                    return None;
                };

                // if ![token_0, token_1].contains(&WETH_ADDRESS.parse::<H160>().ok()?) {
                //     return None;
                // }

                req_throttle
                    .lock()
                    .expect("Could not acquire Mutex")
                    .increment_or_sleep(1, 8000);

                let _pool = Pool::new(
                    provider.clone(),
                    address,
                    token_0,
                    token_1,
                    U256::from(3000),
                    PoolVariant::UniswapV2,
                )
                .await?;
                Some(_pool)
            }
            PoolVariant::UniswapV3 => {
                let uniswap_v3_factory = uniswap_v3_factory_contract::uniswap_v3_factory::new(
                    self.factory_address,
                    provider.clone(),
                );

                let (token_0, token_1, fee, _, address) = if let Ok(pool) = uniswap_v3_factory
                    .decode_event::<(Address, Address, u32, u128, Address)>(
                        "PoolCreated",
                        log.topics,
                        log.data,
                    ) {
                    pool
                } else {
                    return None;
                };

                // if ![token_0, token_1].contains(&WETH_ADDRESS.parse::<H160>().ok()?) {
                //     return None;
                // }

                req_throttle
                    .lock()
                    .expect("Could not acquire Mutex")
                    .increment_or_sleep(1, 8000);

                let _pool = Pool::new(
                    provider.clone(),
                    address,
                    token_0,
                    token_1,
                    U256::from(fee),
                    PoolVariant::UniswapV3,
                )
                .await?;
                Some(_pool)
            }
        }
    }
}

// get all pairs for a given dex between `start_block` and `current_block`
pub async fn sync_dex(
    dexes: Vec<Dex>,
    client: &Arc<Provider<Ws>>,
    current_block: U64,
    start_block: Option<BlockNumber>,
    req_per_sec: usize,
) -> Result<Vec<Pool>, PairSyncError> {
    // initialize multi progress bar
    let multi_progress_bar = MultiProgress::new();

    let mut handles = vec![];

    let req_throttle = Arc::new(Mutex::new(RequestThrottle::new(req_per_sec)));

    // for each dex supplied, get all pair created events
    for dex in dexes {
        let req_throttle = req_throttle.clone();

        let async_provider = client.clone();
        let progress_bar = multi_progress_bar.add(ProgressBar::new(0));

        handles.push(tokio::spawn(async move {
            progress_bar.set_style(
                ProgressStyle::with_template("{msg} {bar:40.green/grey} {pos:>7}/{len:7} Blocks")
                    .unwrap()
                    .progress_chars("##-"),
            );

            let pools = get_all_pools(
                dex,
                async_provider.clone(),
                BlockNumber::Number(current_block),
                start_block,
                progress_bar.clone(),
                1999,
                req_throttle.clone(),
            )
            .await?;

            println!("Pulled {} Pairs", pools.len());

            progress_bar.reset();
            progress_bar.set_style(
                ProgressStyle::with_template("{msg} {bar:40.green/grey} {pos:>7}/{len:7} Pairs")
                    .unwrap()
                    .progress_chars("##-"),
            );

            Ok::<Vec<Pool>, PairSyncError>(pools)
        }));
    }

    // aggregate the populated pools from each thread
    let mut aggregated_pools: Vec<Pool> = vec![];

    for handle in handles {
        match handle.await {
            Ok(sync_result) => aggregated_pools.extend(sync_result?),
            Err(join_error) => return Err(PairSyncError::JoinError(join_error)),
        }
    }
    println!("Synced {} Pairs", aggregated_pools.len());
    println!("Pools: {:?}", aggregated_pools);

    // return the populated aggregated pools vec
    Ok(aggregated_pools)
}

/// function to get all pair created events for a given Dex factory address
async fn get_all_pools(
    dex: Dex,
    provider: Arc<Provider<Ws>>,
    current_block: BlockNumber,
    start_block: Option<BlockNumber>,
    progress_bar: ProgressBar,
    step: usize,
    req_throttle: Arc<Mutex<RequestThrottle>>,
) -> Result<Vec<Pool>, PairSyncError> {
    // get start block
    let creation_block = if let Some(block) = start_block {
        block.as_number().unwrap().as_u64()
    } else {
        dex.creation_block.as_number().unwrap().as_u64()
    };

    let current_block = current_block.as_number().unwrap().as_u64();

    // initialize the progress bar message
    progress_bar.set_length(current_block - creation_block);
    progress_bar.set_message(format!("Getting all pools from: {}", dex.factory_address));

    // init a new vec to keep track of tasks
    let mut handles = vec![];

    // for each block within the range, get all pairs asynchronously
    for from_block in (creation_block..=current_block).step_by(step) {
        req_throttle
            .lock()
            .expect("Could noet acquire Mutex")
            .increment_or_sleep(2, 1000);

        let provider = provider.clone();
        let progress_bar = progress_bar.clone();

        //Spawn a new task to get pair created events from the block range
        handles.push(tokio::spawn(async move {
            let mut pools = vec![];

            //Get pair created event logs within the block range
            let to_block = from_block + step as u64;

            let logs = provider
                .get_logs(
                    &Filter::new()
                        .topic0(ValueOrArray::Value(
                            dex.pool_variant.pool_created_event_signature(),
                        ))
                        .address(dex.factory_address)
                        .from_block(BlockNumber::Number(U64([from_block])))
                        .to_block(BlockNumber::Number(U64([to_block]))),
                )
                .await?;

            let inner_req_throttle = Arc::new(Mutex::new(RequestThrottle::new(1)));
            // increment the progres bar by the step
            progress_bar.inc(step as u64);

            // for each pair created log, create a new Pair type and add it to the pairs vec
            for log in logs {
                match dex
                    .new_pool_from_event(log, provider.clone(), inner_req_throttle.clone())
                    .await
                {
                    Some(pool) => pools.push(pool),
                    None => continue,
                }
            }
            Ok::<Vec<Pool>, ProviderError>(pools)
        }));
    }

    let mut aggregated_pairs: Vec<Pool> = vec![];
    let mut handled = 0;
    for handle in handles {
        println!("Handled {:?} Pools", handled);

        handled += 1;
        match handle.await {
            Ok(sync_result) => aggregated_pairs.extend(sync_result?),
            Err(join_error) => return Err(PairSyncError::JoinError(join_error)),
        }
    }
    println!("{:?}", aggregated_pairs);
    Ok(aggregated_pairs)
}

#[derive(Error, Debug)]
pub enum PairSyncError {
    #[error("Provider error")]
    ProviderError(#[from] ProviderError),
    #[error("Contract error")]
    ContractError(#[from] ContractError<Provider<Ws>>),
    #[error("ABI error")]
    ABIError(#[from] AbiError),
    #[error("Join error")]
    JoinError(#[from] JoinError),
    #[error("Pair for token_a/token_b does not exist in provided dexes")]
    PairDoesNotExistInDexes(H160, H160),
}
