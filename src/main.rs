pub mod abigen;
pub mod bindings;
pub mod cfmm;
pub mod uni_math;
pub mod utils;

use crate::cfmm::{
    dex,
    pool::{Pool, PoolVariant},
};
use crate::utils::base_fee_helper;
use crate::utils::constants::{UNISWAP_V2_FACTORY, UNISWAP_V3_FACTORY};
use crate::utils::relayer;
use crate::utils::state_diff;
use cfmms::dex::{Dex, DexVariant};
use dashmap::DashMap;
use dotenv::dotenv;
use ethers::core::types::{Block, Bytes, U256};
use ethers::prelude::*;
use ethers::providers::{Provider, Ws, Middleware};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::transaction::{eip2718::TypedTransaction, eip2930::AccessList};
use ethers::types::NameOrAddress;
use ethers_flashbots::PendingBundle;
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
    let _blast_key = env::var("BLAST_API_KEY").clone().unwrap();
    let _blast_key_sepolia = env::var("BLAST_API_SEPOLIA").clone().unwrap();
    let _etherscan_key = env::var("ETHERSCAN_API_KEY").clone().unwrap();

    // for WETH address need to check current request and pool via weth9 function
    // from router contract
    let http_provider_sepolia =
        Provider::try_from(format!("https://sepolia.infura.io/v3/{}", _infura_key))?;
    let http_provider =
        Provider::try_from(format!("https://mainnet.infura.io/v3/{}", _infura_key))?;
    // see: https://www.gakonst.com/ethers-rs/providers/ws.html
    // let ws_url_sepolia = format!("wss://sepolia.infura.io/ws/v3/{}", _infura_key);

    let mainnet_blast_url = format!("wss://eth-mainnet.blastapi.io/{}", _blast_key);
    let sepolia_blast_url = format!("wss://eth-sepolia.blastapi.io/{}", _blast_key_sepolia);
    let llama_url = format!("wss://eth.llamarpc.com");
    // let ws_provider_sepolia = Provider::<Ws>::connect(ws_url_sepolia).await?;
    let mainnet_ws_provider = Arc::new(Provider::<Ws>::connect(mainnet_blast_url).await?);
    let sepolia_ws_provider = Arc::new(Provider::<Ws>::connect(sepolia_blast_url).await?);
    let block_provider = Provider::<Ws>::connect(llama_url).await?;

    // bundle signing
    let _bundle_signer = env::var("FLASHBOTS_IDENTIFIER")?;
    let bundle_signer = _bundle_signer.parse::<LocalWallet>()?;

    let _wallet = env::var("FLASHBOTS_SIGNER").clone().unwrap();
    let wallet = _wallet.parse::<LocalWallet>().unwrap();

    println!("Bundle Signer: {}", _bundle_signer);
    println!("Wallet: {}", _wallet);

    let mut flashbot_middleware = FlashbotsMiddleware::new(
        mainnet_ws_provider.clone(),
        // http_provider_sepolia,
        // Url::parse("https://relay-sepolia.flashbots.net")?,
        Url::parse("https://relay.flashbots.net")?,
        bundle_signer.clone(),
    );

    flashbot_middleware.set_simulation_relay(
        // Url::parse("https://relay-sepolia.flashbots.net")?,
        Url::parse("https://relay.flashbots.net")?,
        bundle_signer.clone(),
    );

    let flashbot_client = Arc::new(SignerMiddleware::new(
        flashbot_middleware,
        wallet.clone(),
    ));


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

    let current_block = mainnet_ws_provider.get_block_number().await?;
    // let synced_pools = dex::sync_dex(
    //     dexes.clone(),
    //     &Arc::clone(&mainnet_ws_provider),
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

    // send bundle via https://relay-sepolia.flashbots.net
    // swap v2 usdc - eth
    // to: 0xd9Bea83c659a3D8317a8f1fecDc6fe5b3298AEcc
    // from: 0x9B749e19580934D14d955F993CB159D9747478DA
    // data: 0xe97ed6120000000000000000000000000000000000000000000000000000000000087e6f
    // chain_id: 11155111
    // max_priority_fee_per_gas: Some(U256::from(0))
    // max_fee_per_gas: Some(next_base_fee)
    //  gas: Some(U256::from(250000))
    // nonce: Some(nonce)
    // access_list: AccessList::default()

    // let current_block = flashbot_client
    //     .get_block(BlockId::Number(BlockNumber::Latest))
    //     .await;
    // let target = if let Some(b) = current_block.unwrap() {
    //     b.number.unwrap()// + 1
    // } else {
    //     U64::from(0)
    // };
    let target = current_block;

    let wallet_address = wallet.address();
    let test_to = NameOrAddress::from("0x9B749e19580934D14d955F993CB159D9747478DA");
    let test_data = Bytes::from_static(b"0xe97ed6120000000000000000000000000000000000000000000000000000000000087e6f");
    let test_nonce = flashbot_client.get_transaction_count(wallet_address.clone(), None).await?;
    println!("Nonce: {}", test_nonce);
    let test_transaction_request = Eip1559TransactionRequest {
        to: Some(test_to),
        from: Some(wallet_address),
        data: Some(test_data),
        chain_id: Some(U64::from(1)),
        max_priority_fee_per_gas: Some(U256::from(0)),
        max_fee_per_gas: Some(U256::MAX),
        gas: Some(U256::from(250000)),
        nonce: Some(test_nonce),
        value: None,
        access_list: AccessList::default(),
    };

    let frontrun_tx_typed = TypedTransaction::Eip1559(test_transaction_request);
    
    let tx = {
        let mut inner: TypedTransaction = frontrun_tx_typed;
        flashbot_client.fill_transaction(&mut inner, None).await?;
        inner
    };
    println!("Tx: {:?}", tx);

    let signature = flashbot_client.signer().sign_transaction(&tx).await?;
    let signed_frontrun_tx = tx.rlp_signed(&signature);
    let signed_transactions = vec![signed_frontrun_tx];
    println!("signed_transactions: {:?}", signed_transactions);

    let bundle = relayer::construct_bundle(signed_transactions, target).map_err(|e| {
        println!("Bundle Construction Error{:?}", e);
        e
    })?;

    // Simulate the flashbots bundle
    let simulated_bundle = flashbot_client.inner().simulate_bundle(&bundle).await;
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

    let mut mempool_stream = mainnet_ws_provider.subscribe_pending_txs().await?;
    println!("Subscribed to pending txs");

    while let Some(tx_hash) = mempool_stream.next().await {

        println!("New TxHash: {:?}", tx_hash);
        let msg = mainnet_ws_provider.get_transaction(tx_hash).await?;
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
            &Arc::clone(&mainnet_ws_provider),
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
