use dotenv::dotenv;
use std::env;
use ethers::core::{rand::thread_rng, types::TransactionRequest, types::transaction::eip2718::TypedTransaction};
use ethers::prelude::*;
use ethers::providers::{Provider, Ws};
use ethers::core::k256::SecretKey;
use ethers_flashbots::{BundleRequest, FlashbotsMiddleware};
use eyre::Result;
use std::convert::TryFrom;
use url::Url;
use ethers_signers::{LocalWallet, Signer};

#[tokio::main]
pub async fn first_fn() -> Result<()> {
    // data collection
    dotenv().ok(); 
    let _infura_key = env::var("INFURA_SEPOLIA_API_KEY").clone().unwrap();
    //let http_provider = Provider::try_from("https://mainnet.eth.aragon.network")?;
    let http_provider = Provider::try_from(format!("https://sepolia.infura.io/v3/{}", _infura_key))?;

    // see: https://www.gakonst.com/ethers-rs/providers/ws.html
    let ws_url = format!("wss://sepolia.infura.io/ws/v3/{}", _infura_key);
    let ws_provider = Provider::<Ws>::connect(ws_url).await?;

    // bundle signing
    let _bundle_signer = env::var("FLASHBOTS_IDENTIFIER").clone().unwrap();
    let bundle_signer = _bundle_signer.parse::<LocalWallet>();
    //let addr = _bundle_signer.address();

    let _wallet = env::var("FLASHBOTS_SIGNER").clone().unwrap();
    let wallet = _wallet.parse::<LocalWallet>().unwrap();

    println!("Bundle Signer: {}", _bundle_signer);
    println!("Wallet: {}", _wallet);

    let client = SignerMiddleware::new(
        FlashbotsMiddleware::new(
            http_provider,
            // use testnet builder
            Url::parse("https://relay-sepolia.flashbots.net")?,
            // Url::parse("https://relay.flashbots.net")?,
            bundle_signer.unwrap(),
        ),
        wallet,
    );

    // let seploia_endpoint = "https://api-sepolia.etherscan.io/";
//     https://api-sepolia.etherscan.io/api
//    ?module=transaction
//    &action=getstatus
//    &txhash=0x5d329954fae7d19b2fb9abf0e6862735243b1079c58e0ea307d7e933657ac083
//    &apikey=YourApiKeyToken
    let mut stream = ws_provider.subscribe_pending_txs().await?;

    
    //let mut stream = ws_provider.pending_bundle(msg).await?;
    while let Some(msg) = stream.next().await {
        let mut msg2 = ws_provider.get_transaction(msg).await?;
        println!(
            // "Timestamp: {:?}, block number: {} -> {:?}", 
            // block.timestamp,
            // block.number.unwrap(),
            // block.hash.unwrap()
            // msg
            "{:?}", msg2
        );

        // let tx: TypedTransaction = TransactionRequest::pay("vitalik.eth", 1).into();
        // let signature = client.signer().sign_transaction(&tx).await?;
        // let mut bundle = BundleRequest::new();
        // bundle.add_transaction(
        //     tx.rlp_signed(&signature)
        // );
    
        //let bundle = bundle.set_block(block.number.unwrap()+1).set_simulation_block(block.number.unwrap()).set_simulation_timestamp(0);

        //sending bundle
        // let pending_bundle = client.inner().send_bundle(&bundle).await?;
    }



    Ok(())
}