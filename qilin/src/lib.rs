pub mod abigen;
pub mod init;
pub mod utils;

use qilin_cfmms::{
    dex,
    pool::{Pool, PoolVariant},
};
use anyhow::Result;
use dashmap::DashMap;
use env_logger::Env;
use ethers::{
    core::types::{Block, U256},
    prelude::*,
    providers::{Middleware, Provider, Ws},
};
use parking_lot::RwLock;
use std::env;

use std::sync::{Arc, Mutex};

use collectors::{
    block_collector::{BlockPayload, QilinBlockCollector},
    mempool_collector::{NewTx, QilinMempoolCollector},
};
use artemis::{engine::Engine, types::Collector};

pub async fn runner() -> Result<()> {

    env_logger::Builder::from_env(Env::default()).init();

    let (flashbot_client, all_pools, hash_addr_pools) = init::setup().await?;
    let ws_provider = flashbot_client.inner().inner().clone();
    let initial_block_num = ws_provider
        .get_block_number()
        .await
        .expect("Error getting block number");
    let initial_block = ws_provider
        .get_block(initial_block_num)
        .await
        .unwrap()
        .unwrap_or(Block::default());

    // let engine = Engine::<Event, Action>::default();

    let mempool_collector = QilinMempoolCollector::new(ws_provider.clone(), initial_block.clone());
    // let block_collector: Box<dyn Collector<BlockPayload>> = QilinBlockCollector::new(
    //     ws_provider.clone(),
    //     all_pools.clone(),
    //     hash_addr_pools.clone(),
    // );

    // engine.add_collector(Box::new(mempool_collector));
    // engine.add_collector();

    Ok(())
}