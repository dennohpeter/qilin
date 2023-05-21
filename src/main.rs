pub mod abigen;
pub mod bindings;
pub mod cfmm;
pub mod uni_math;
pub mod utils;

use crate::utils::base_fee_helper;
use crate::utils::constants::{UNISWAP_V2_FACTORY, UNISWAP_V3_FACTORY};
use crate::utils::relayer;
use crate::utils::state_diff;
use dashmap::DashMap;
use dotenv::dotenv;
use ethers::core::types::{Block, Bytes, U256};
use ethers::prelude::*;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::signers::{LocalWallet};
use ethers::types::NameOrAddress;
use ethers_flashbots::PendingBundle;
use ethers_flashbots::{FlashbotsMiddleware};
use eyre::Result;
use std::env;
use std::error::Error;
use std::sync::{Arc, Mutex};
use url::Url;
use cfmms::dex::{Dex, DexVariant};
use crate::cfmm::{
    dex,
    pool::{Pool, PoolVariant},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv()?;
    let arg: Vec<String> = env::args().collect();

    let first_arg = if arg.len() > 1 {
        arg[1].clone()
    } else {
        String::from("")
    };

    match first_arg.get(0..1) {
        Some(_) => {
            if first_arg.contains("abigen") {
                abigen::generate_abigen_for_addresses().await?;
                return Ok(());
            } else {
                println!("command not recognized");
            }
        }
        None => {
            println!("");
        }
    }

    // data collection
    let _blast_key = env::var("BLAST_API_KEY").unwrap();
    let _blast_key_goerli = env::var("BLAST_API_GOERLI").unwrap();
    let _etherscan_key = env::var("ETHERSCAN_API_KEY").unwrap();

    let mainnet_blast_url = format!("wss://eth-mainnet.blastapi.io/{}", _blast_key);
    let goerli_blast_url = format!("wss://eth-goerli.blastapi.io/{}", _blast_key_goerli);
    let llama_url = format!("wss://eth.llamarpc.com");

    let mainnet_ws_provider = Arc::new(Provider::<Ws>::connect(mainnet_blast_url).await?);
    let goerli_ws_provider = Arc::new(Provider::<Ws>::connect(goerli_blast_url).await?);
    let block_provider = Provider::<Ws>::connect(llama_url).await?;

    // bundle signing
    let _bundle_signer = env::var("FLASHBOTS_IDENTIFIER")?;
    let bundle_signer = _bundle_signer.parse::<LocalWallet>()?;

    let _wallet = env::var("FLASHBOTS_SIGNER").clone().unwrap();
    let wallet = _wallet.parse::<LocalWallet>().unwrap();

    println!("Bundle Signer: {}", _bundle_signer);
    println!("Wallet: {}", _wallet);

    let mut flashbot_middleware = FlashbotsMiddleware::new(
        // mainnet_ws_provider.clone(),
        goerli_ws_provider.clone(),
        Url::parse("https://relay-goerli.flashbots.net")?,
        // Url::parse("https://relay.flashbots.net")?,
        bundle_signer.clone(),
    );

    flashbot_middleware.set_simulation_relay(
        Url::parse("https://relay-goerli.flashbots.net")?,
        // Url::parse("https://relay.flashbots.net")?,
        bundle_signer.clone(),
    );

    let flashbot_client = Arc::new(SignerMiddleware::new(flashbot_middleware, wallet.clone()));

    let dexes = vec![
        //UniswapV2
        dex::Dex::new(
            UNISWAP_V2_FACTORY.parse::<H160>()?,
            PoolVariant::UniswapV2,
            10000835,
        ),
        //Add UniswapV3
        dex::Dex::new(
            UNISWAP_V3_FACTORY.parse::<H160>()?,
            PoolVariant::UniswapV3,
            12369621,
        ),
    ];

    // let synced_pools = dex::sync_dex(
    //     dexes.clone(),
    //     &Arc::clone(&mainnet_ws_provider),
    //     //&Arc::clone(&goerli_ws_provider),
    //     current_block,
    //     None,
    //     2, //throttled for 2 secs
    // )
    // .await?;

    // let all_pools: DashMap<Address, Pool> = DashMap::new();
    // for pool in synced_pools {
    //     all_pools.insert(pool.address, pool);
    // }

    // let all_pools = Arc::new(all_pools);

    let block: Arc<Mutex<Option<Block<H256>>>> = Arc::new(Mutex::new(None));
    let block_clone = Arc::clone(&block);


    let _to = NameOrAddress::from("0xd9Bea83c659a3D8317a8f1fecDc6fe5b3298AEcc");
    let _data = Bytes::from_static(
        b"0xe97ed6120000000000000000000000000000000000000000000000000000000000087e6f",
    );

    let simulated_bundle = relayer::simulate_bundle(
        _to,
        _data,
        &flashbot_client.clone(),
        &goerli_ws_provider.clone(),
        wallet.clone(),
    )
    .await?;
    println!("simulated_bundle: {:?}", simulated_bundle);

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

    let mut mempool_stream = goerli_ws_provider.subscribe_pending_txs().await?;
    println!("Subscribed to pending txs");

    while let Some(tx_hash) = mempool_stream.next().await {
        println!("New TxHash: {:?}", tx_hash);
        let msg = goerli_ws_provider.get_transaction(tx_hash).await?;
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
            //&Arc::clone(&mainnet_ws_provider),
            &Arc::clone(&goerli_ws_provider),
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
        // let mev_pools =
        // if let Some(mevP) = utils::state_diff::extract_pools(&state_diffs, &all_pools) {
        //     mevP
        // } else {
        //     continue;
        // };
        // let fork_block = Some(BlockId::Number(BlockNumber::Number(
        //     block_oracle.next_block.number,
        // )));
        // let fork_block = Some(BlockId::Number(BlockNumber::Number(
        // )));
    }

    Ok(())
}
