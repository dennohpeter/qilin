pub mod abigen;
pub mod bindings;
pub mod cfmm;
pub mod uni_math;
pub mod utils;

use crate::cfmm::{dex, pool::{
        PoolVariant, Pool
    }
};
use crate::utils::constants::{
    DAI_ADDRESS, NULL_ADDRESS, SELECTOR_UNI, SELECTOR_V2_R1, SELECTOR_V2_R2, SELECTOR_V3_R1,
    SELECTOR_V3_R2, UNISWAP_UNIVERSAL_ROUTER, UNISWAP_V2_ROUTER_1, UNISWAP_V2_ROUTER_2,
    UNISWAP_V3_ROUTER_1, UNISWAP_V3_ROUTER_2, USDC_ADDRESS, USDT_ADDRESS, WETH_ADDRESS,
};
use crate::utils::constants::{UNISWAP_V2_FACTORY, UNISWAP_V3_FACTORY};
use crate::utils::state_diff;
use dashmap::DashMap;
use dotenv::dotenv;
use ethers::core::types::{Block, Bytes, U256};
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
use crate::utils::{
    base_fee_helper
};
use cfmms::{
	dex::{
		Dex, DexVariant
	},
};
use ethers::types::NameOrAddress;
use ethers::types::transaction::{
    eip2930::AccessList,
    eip2718::TypedTransaction
};
use crate::utils::relayer;
use ethers_flashbots::PendingBundle;
use std::str::FromStr;
use ethers::types::{BlockId, U64};

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
    let searcher_wallet = env::var("FLASHBOTS_IDENTIFIER").clone().unwrap().parse::<LocalWallet>();
    //let bundle_signer = env::var("FLASHBOTS_SIGNER").clone().unwrap().parse::<LocalWallet>();
    //let bundle_signer = env::var("FLASHBOTS_SIGNER").clone().unwrap();
    //let private_key_bytes = hex::decode(bundle_signer.trim_start_matches("0x")).expect("Decoding failed");
    //let wallet = LocalWallet::new(secret_key_from_bytes(private_key_bytes).expect("Invalid private key"));
    //let signer2 = LocalWallet::from_private_key_str(bundle_signer);

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
    let _bundle_signer = env::var("FLASHBOTS_IDENTIFIER").clone().unwrap();
    let bundle_signer = _bundle_signer.parse::<LocalWallet>();
    //let addr = _bundle_signer.address();

    let _wallet = env::var("FLASHBOTS_SIGNER").clone().unwrap();
    let wallet = _wallet.parse::<LocalWallet>().unwrap();

    println!("Bundle Signer: {}", _bundle_signer);
    println!("Wallet: {}", _wallet);

    /*
    //https://boost-relay-sepolia.flashbots.net/ 
    let flashbot_client = Arc::new(
        SignerMiddleware::new(
            FlashbotsMiddleware::new(
                http_provider,
                Url::parse("https://relay-goerli.flashbots.net")?,
                //Url::parse("https://relay.flashbots.net")?,
                LocalWallet::from_bytes(bundle_signer)
            ),
            wallet,
        )
    );
     */

    let flashbot_client = Arc::new(
        SignerMiddleware::new(
            FlashbotsMiddleware::new(
                http_provider_sepolia,
                Url::parse("https://relay-sepolia.flashbots.net")?,
                bundle_signer.unwrap(),
            ),
            wallet,
        )
    );

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

    
    //let current_block = mainnet_ws_provider.get_block_number().await?;
    let current_block = sepolia_ws_provider.get_block_number().await?;
    // let synced_pools = dex::sync_dex(
    //     dexes.clone(),
    //     &Arc::clone(&mainnet_ws_provider),
    //     //&Arc::clone(&sepolia_ws_provider),
    //     current_block,
    //     None,
    //     2 //throttled for 2 secs
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
    
    // 0x864232e4b50F13f5599c3E0Ff6d21c2F42155a8d
    // let block_number = flashbot_client.get_block_number().await?;
    // let block_number = block_number;
    // println!("Block number: {:?}", block_number);
    // let tx = {
    //     let mut inner: TypedTransaction = TransactionRequest::pay(
    //         "0x864232e4b50F13f5599c3E0Ff6d21c2F42155a8d".parse::<H160>()?, 
    //         100).into();
    //     let inner_clone = inner.clone();
    //     //println!("inner: {:?}", inner_clone);
    //     flashbot_client.send_transaction(inner, Some(BlockId::from(block_number))).await?;
    //     inner_clone
    // };
    // let tx = {
    //     let mut inner: TypedTransaction = TransactionRequest::new()
    //         .from("0xd9Bea83c659a3D8317a8f1fecDc6fe5b3298AEcc".parse::<H160>()?)
    //         .to(NameOrAddress::from("0x9B749e19580934D14d955F993CB159D9747478DA"))
    //         .data(Bytes::from_static(b"0xe97ed6120000000000000000000000000000000000000000000000000000000000087e6f"))
    //         .chain_id(U64::from(11155111))
    //         .gas(U256::from(250000)).into();
    //     flashbot_client.fill_transaction(&mut inner, None).await?;
    //     inner
    // };
    // let signature = flashbot_client.signer().sign_transaction(&tx).await?;
    // let bundle = BundleRequest::new()
    //     .push_transaction(tx.rlp_signed(&signature))
    //     .set_block(block_number + 1)
    //     .set_simulation_block(block_number)
    //     .set_simulation_timestamp(0);
    // println!("bundle: {:?}", bundle);
    // let simulated_bundle = flashbot_client.inner().simulate_bundle(&bundle).await?;
    // println!("Simulated bundle: {:?}", simulated_bundle);


    let current_block = flashbot_client.get_block(BlockId::Number(BlockNumber::Latest)).await;
    let target = if let Some(b) = current_block.unwrap() {
        b.number.unwrap() + 1
    } else {
        U64::from(0)
    };
    let test_from = NameOrAddress::from("0xd9Bea83c659a3D8317a8f1fecDc6fe5b3298AEcc");
    let test_to = NameOrAddress::from("0x9B749e19580934D14d955F993CB159D9747478DA");
    let test_data = Bytes::from_static(b"0xe97ed6120000000000000000000000000000000000000000000000000000000000087e6f");
    let test_nonce = flashbot_client.get_transaction_count(test_from, None).await?;
    let test_transaction_request = Eip1559TransactionRequest {
        to: Some(test_to),
        from: Some("0xd9Bea83c659a3D8317a8f1fecDc6fe5b3298AEcc".parse::<H160>()?),
        data: Some(test_data),
        chain_id: Some(U64::from(11155111)),
        max_priority_fee_per_gas: Some(U256::from(0)),
        max_fee_per_gas: Some(U256::MAX),
        gas: Some(U256::from(250000)),
        nonce: Some(test_nonce),
        value: None,
        access_list: AccessList::default()
    };
    let frontrun_tx_typed = TypedTransaction::Eip1559(test_transaction_request);
    let signed_frontrun_tx_sig = searcher_wallet.unwrap().sign_transaction(&frontrun_tx_typed).await;
    println!("signed_frontrun_tx_sig: {:?}", signed_frontrun_tx_sig);
    let signed_frontrun_tx = frontrun_tx_typed.rlp_signed(&signed_frontrun_tx_sig.unwrap());
    let signed_transactions = vec![signed_frontrun_tx];
    println!("signed_transactions: {:?}", signed_transactions);
    let bundle2 = match relayer::construct_bundle(signed_transactions, target) {
        Ok(b) => b,
        Err(e) => { 
            println!("Construct Bundle: {:?}", e);
            BundleRequest::new()
         }
    };
    println!("bundle: {:?}", bundle2);
    // Simulate the flashbots bundle
    let simulated_bundle2 = match flashbot_client.inner().send_bundle(&bundle2).await {
        Ok(s) => {
            println!("Simulated Bundle Success");
            s
        },
        Err(e) => {
            println!("Simulated Bundle Error: {:?}", e);
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };
    // goreli cannot decode [tx] text
    /* This error is likely due to relay-sepolia.flashbots.net being a sucky rpc
    RelayError(
        RequestError(
            reqwest::Error { 
                kind: Status(500), 
                url: Url { 
                    scheme: "https", 
                    cannot_be_a_base: false, 
                    username: "", 
                    password: None, 
                    host: Some(Domain("relay-sepolia.flashbots.net")), 
                    port: None, 
                    path: "/", 
                    query: None, 
                    fragment: None } }))

     */





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

    //let mut mempool_stream = mainnet_ws_provider.subscribe_pending_txs().await?;
    let mut mempool_stream = sepolia_ws_provider.subscribe_pending_txs().await?;
    while let Some(tx_hash) = mempool_stream.next().await {

        //let msg = mainnet_ws_provider.get_transaction(tx_hash).await?;
        let msg = sepolia_ws_provider.get_transaction(tx_hash).await?;
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
            &Arc::clone(&sepolia_ws_provider),
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