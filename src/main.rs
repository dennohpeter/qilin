pub mod abigen;
pub mod bindings;
pub mod uni_math;
pub mod utils;

use crate::utils::base_fee_helper;
use crate::utils::constants::{
    DAI_ADDRESS, NULL_ADDRESS, SELECTOR_UNI, SELECTOR_V2_R1, SELECTOR_V2_R2, SELECTOR_V3_R1,
    SELECTOR_V3_R2, UNISWAP_UNIVERSAL_ROUTER, UNISWAP_V2_ROUTER_1, UNISWAP_V2_ROUTER_2,
    UNISWAP_V3_ROUTER_1, UNISWAP_V3_ROUTER_2, USDC_ADDRESS, USDT_ADDRESS, WETH_ADDRESS,
};
use crate::utils::state_diff;
use dotenv::dotenv;
use ethers::core::{
    types::{Block, Bytes, U256},
    utils::hex,
};
use ethers::prelude::*;
use ethers::providers::{Provider, Ws};
use ethers::signers::{LocalWallet, Signer};
use ethers_flashbots::{BundleRequest, FlashbotsMiddleware};
use eyre::Result;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::error::Error;
use std::sync::{Arc, Mutex};
use url::Url;

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
    let _infura_key = env::var("INFURA_API_KEY").clone().unwrap();
    let _etherscan_key = env::var("ETHERSCAN_API_KEY").clone().unwrap();

    // for WETH address need to check current request and pool via weth9 function
    // from router contract
    let http_provider_sepolia =
        Provider::try_from(format!("https://sepolia.infura.io/v3/{}", _infura_key))?;
    let http_provider =
        Provider::try_from(format!("https://mainnet.infura.io/v3/{}", _infura_key))?;

    // see: https://www.gakonst.com/ethers-rs/providers/ws.html
    // let ws_url_sepolia = format!("wss://sepolia.infura.io/ws/v3/{}", _infura_key);
    let infura_url = format!("wss://mainnet.infura.io/ws/v3/{}", _infura_key);
    let llama_url = format!("wss://eth.llamarpc.com");
    // let ws_provider_sepolia = Provider::<Ws>::connect(ws_url_sepolia).await?;
    let ws_provider = Arc::new(Provider::<Ws>::connect(infura_url).await?);
    let block_provider = Provider::<Ws>::connect(llama_url).await?;

    // bundle signing
    let _bundle_signer = env::var("FLASHBOTS_IDENTIFIER").clone().unwrap();
    let bundle_signer = _bundle_signer.parse::<LocalWallet>();
    //let addr = _bundle_signer.address();

    let _wallet = env::var("FLASHBOTS_SIGNER").clone().unwrap();
    let wallet = _wallet.parse::<LocalWallet>().unwrap();

    println!("Bundle Signer: {}", _bundle_signer);
    println!("Wallet: {}", _wallet);

    let flashbot_client = Arc::new(SignerMiddleware::new(
        FlashbotsMiddleware::new(
            http_provider,
            //Url::parse("https://relay-sepolia.flashbots.net")?,
            Url::parse("https://relay.flashbots.net")?,
            bundle_signer.unwrap(),
        ),
        wallet,
    ));

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

    let routers = [
        (UNISWAP_UNIVERSAL_ROUTER, "Uniswap Univeral Router"),
        (UNISWAP_V3_ROUTER_1, "Uniswap V3 Router 1"),
        (UNISWAP_V3_ROUTER_2, "Uniswap V3 Router 2"),
        (UNISWAP_V2_ROUTER_1, "Uniswap V2 Router 1"),
        (UNISWAP_V2_ROUTER_2, "Uniswap V2 Router 2"),
    ];

    let mut router_selectors = HashMap::new();
    router_selectors.insert(UNISWAP_UNIVERSAL_ROUTER, &SELECTOR_UNI[..]);
    router_selectors.insert(UNISWAP_V3_ROUTER_1, &SELECTOR_V3_R1[..]);
    router_selectors.insert(UNISWAP_V3_ROUTER_2, &SELECTOR_V3_R2[..]);
    router_selectors.insert(UNISWAP_V2_ROUTER_1, &SELECTOR_V2_R1[..]);
    router_selectors.insert(UNISWAP_V2_ROUTER_2, &SELECTOR_V2_R2[..]);

    let mut mempool_stream = ws_provider.subscribe_pending_txs().await?;

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

        //     // if tx has statediff on pool addr then record it in `sandwichable_pools`
        //     let sandwichable_pools =
        //     if let Some(sp) = utils::state_diff::extract_pools(&state_diffs, &all_pools) {
        //         sp
        //     } else {
        //         log::info!("{:?}", victim_tx.hash);
        //         continue;
        //     };

        // let fork_block = Some(BlockId::Number(BlockNumber::Number(
        //     block_oracle.next_block.number,
        // )));

        /////////////////////////////////////////////////////////////////////////////////////////////////////////////
        // while let Some(mut victim_tx) = mempool_stream.next().await {
        // let client = utils::create_websocket_client().await?;
        // let block_oracle = {
        //     let read_lock = self.latest_block_oracle.read().await;
        //     (*read_lock).clone()
        // };
        // let all_pools = &self.all_pools;
        // let sandwich_balance = {
        //     let read_lock = self.sandwich_state.weth_balance.read().await;
        //     (*read_lock).clone()
        // };
        // // ignore txs that we can't include in next block
        // // enhancement: simulate all txs, store result, and use result when tx can included
        // if victim_tx.max_fee_per_gas.unwrap_or(U256::zero()) < block_oracle.next_block.base_fee
        // {
        //     log::info!("{}", format!("{:?} mf<nbf", victim_tx.hash).cyan());
        //     continue;
        // }

        // // recover from field from vrs (ECDSA)
        // // enhancement: expensive operation, can avoid by modding rpc to share `from` field
        // if let Ok(from) = victim_tx.recover_from() {
        //     victim_tx.from = from;
        // } else {
        //     log::error!(
        //         "{}",
        //         format!("{:?} ecdsa recovery failed", victim_tx.hash).red()
        //     );
        //     continue;
        // };

        // // get all state diffs that this tx produces
        // let state_diffs = if let Some(sd) = utils::state_diff::get_from_txs(
        //     &self.client,
        //     &vec![victim_tx.clone()],
        //     BlockNumber::Number(block_oracle.latest_block.number),
        // )
        // .await
        // {
        //     sd
        // } else {
        //     log::info!("{:?}", victim_tx.hash);
        //     continue;
        // };
    }
    Ok(())
}
