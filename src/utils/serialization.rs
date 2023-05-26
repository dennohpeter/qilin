use crate::cfmm::pool::Pool;
use dashmap::DashMap;
use ethers::prelude::*;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::prelude::*;

#[derive(Debug)]
pub enum ReadError {
    FileNotFound,
    JsonParsingError(serde_json::Error),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::FileNotFound => write!(f, "all_pools.json file not found in src/assets/"),
            ReadError::JsonParsingError(err) => write!(f, "Failed to parse JSON: {}", err),
        }
    }
}

impl Error for ReadError {}

pub fn write_pool_data(dash: &DashMap<Address, Pool>) -> BTreeMap<Address, Pool> {
    let btree_map: BTreeMap<_, _> = dash
        .iter()
        .map(|entry| (*entry.key(), entry.value().clone()))
        .collect();

    let json_data = serde_json::to_string(&btree_map).unwrap();

    let mut file = File::create("src/assets/all_pools.json").unwrap();
    file.write_all(json_data.as_bytes()).unwrap();

    btree_map
}

pub fn read_pool_data() -> Result<DashMap<Address, Pool>, ReadError> {
    let json_data = match fs::read_to_string("src/assets/all_pools.json") {
        Ok(data) => data,
        Err(_) => return Err(ReadError::FileNotFound),
    };

    let btree_map: BTreeMap<Address, Pool> =
        serde_json::from_str(&json_data).map_err(ReadError::JsonParsingError)?;

    let dash_map: DashMap<Address, Pool> = DashMap::new();
    for (key, value) in btree_map {
        dash_map.insert(key, value);
    }

    Ok(dash_map)
}
