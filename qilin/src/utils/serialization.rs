use dashmap::DashMap;
use ethers::prelude::*;
use ethers::providers::{Provider, Ws};
use ethers::types::U256;
use qilin_cfmms::pool::{Pool, PoolVariant};
use serde::Serialize;
use serde_json;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Debug;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::sync::Arc;

#[derive(Debug)]
pub enum ReadError {
    FileNotFound,
    JsonParsingError(serde_json::Error),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::FileNotFound => {
                write!(f, "all_pools.json file not found in qilin/src/assets/")
            }
            ReadError::JsonParsingError(err) => write!(f, "Failed to parse JSON: {}", err),
        }
    }
}

impl Error for ReadError {}

pub fn write_pool_data<T>(dash: &DashMap<Address, T>, hash_addr: bool) -> BTreeMap<Address, T>
where
    T: Clone + Debug + Serialize,
{
    let btree_map: BTreeMap<_, _> = dash
        .iter()
        .map(|entry| (*entry.key(), entry.value().clone()))
        .collect();

    let json_data = serde_json::to_string(&btree_map).unwrap();

    println!("{:?}", json_data);

    if hash_addr {
        let mut file = File::create("qilin/src/assets/all_pools_hashed.json").unwrap();
        file.write_all(json_data.as_bytes()).unwrap();
    } else {
        let mut file = File::create("qilin/src/assets/all_pools.json").unwrap();
        file.write_all(json_data.as_bytes()).unwrap();
    }

    btree_map
}

pub async fn read_pool_data(
    provider: Arc<Provider<Ws>>,
) -> Result<(DashMap<Address, Pool>, DashMap<Address, Vec<Pool>>), ReadError> {
    let pool_json_data = match fs::read_to_string("qilin/src/assets/all_pools.json") {
        Ok(data) => data,
        Err(_) => return Err(ReadError::FileNotFound),
    };
    let hash_json_data = match fs::read_to_string("qilin/src/assets/all_pools_hashed.json") {
        Ok(data) => data,
        Err(_) => return Err(ReadError::FileNotFound),
    };

    let pool_btree_map: BTreeMap<Address, Pool> =
        serde_json::from_str(&pool_json_data).map_err(ReadError::JsonParsingError)?;
    let hash_pool_btree_map: BTreeMap<Address, Vec<Pool>> =
        serde_json::from_str(&hash_json_data).map_err(ReadError::JsonParsingError)?;

    let pool_dash_map: DashMap<Address, Pool> = DashMap::new();
    for (addr, _pool) in pool_btree_map {
        let pool = pool_initializer(&_pool, provider.clone()).await.unwrap();

        pool_dash_map.insert(addr, pool);
    }

    let hash_pool_dash_map: DashMap<Address, Vec<Pool>> = DashMap::new();
    for (_hash, _pool) in hash_pool_btree_map {
        hash_pool_dash_map.insert(_hash, _pool);
    }

    Ok((pool_dash_map, hash_pool_dash_map))
}

pub async fn pool_initializer(_pool: &Pool, provider: Arc<Provider<Ws>>) -> Option<Pool> {
    match _pool.pool_variant {
        PoolVariant::UniswapV2 => {
            let address = _pool.address;
            let token_0 = _pool.token_0;
            let token_1 = _pool.token_1;

            let _pool = Pool::new(
                provider.clone(),
                address,
                token_0,
                token_1,
                U256::from(3000),
                PoolVariant::UniswapV2,
            )
            .await;
            _pool
        }
        PoolVariant::UniswapV3 => {
            let address = _pool.address;
            let token_0 = _pool.token_0;
            let token_1 = _pool.token_1;
            let fee = _pool.swap_fee;

            let _pool = Pool::new(
                provider.clone(),
                address,
                token_0,
                token_1,
                fee,
                PoolVariant::UniswapV3,
            )
            .await;

            _pool
        }
    }
}

/// for testing purposes
pub fn pool_initializer_test(_pool: &Pool) -> Pool {
    match _pool.pool_variant {
        PoolVariant::UniswapV2 => {
            let address = _pool.address;
            let token_0 = _pool.token_0;
            let token_1 = _pool.token_1;

            let _pool = Pool::new_empty_pool(
                address,
                token_0,
                token_1,
                U256::from(3000),
                PoolVariant::UniswapV2,
            );
            _pool
        }
        PoolVariant::UniswapV3 => {
            let address = _pool.address;
            let token_0 = _pool.token_0;
            let token_1 = _pool.token_1;
            let fee = _pool.swap_fee;

            let _pool =
                Pool::new_empty_pool(address, token_0, token_1, fee, PoolVariant::UniswapV3);

            _pool
        }
    }
}
