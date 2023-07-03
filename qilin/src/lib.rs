pub mod abigen;
pub mod init;
pub mod utils;

use anyhow::Result;

use env_logger::Env;
use ethers::{core::types::Block, prelude::*, providers::Middleware};

use collectors::mempool_collector::QilinMempoolCollector;

pub async fn runner() -> Result<()> {
    env_logger::Builder::from_env(Env::default()).init();

    let (flashbot_client, _all_pools, _hash_addr_pools) = init::setup().await?;
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

    let _mempool_collector = QilinMempoolCollector::new(ws_provider.clone(), initial_block.clone());
    // let block_collector: Box<dyn Collector<BlockPayload>> = QilinBlockCollector::new(
    //     ws_provider.clone(),
    //     all_pools.clone(),
    //     hash_addr_pools.clone(),
    // );

    // engine.add_collector(Box::new(mempool_collector));
    // engine.add_collector();

    Ok(())
}
