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
    let _infura_key = env::var("INFURA_API_KEY").clone().unwrap();
    let _uniswap_v3_address = env::var("V3FACTORY_ADDRESS").clone().unwrap();
    let _dai_address = env::var("DAI_ADDRESS").clone().unwrap();
    let _usdc_address = env::var("USDC_ADDRESS").clone().unwrap();
    let _usdt_address = env::var("USDT_ADDRESS").clone().unwrap();

    // for ETH address need to check current request and pool via weth9 function
    // from router contract
    let http_provider_sepolia = Provider::try_from(format!("https://sepolia.infura.io/v3/{}", _infura_key))?;
    let http_provider = Provider::try_from(format!("https://mainnet.infura.io/v3/{}", _infura_key))?;

    // see: https://www.gakonst.com/ethers-rs/providers/ws.html
    let ws_url_sepolia = format!("wss://sepolia.infura.io/ws/v3/{}", _infura_key);
    let ws_url = format!("wss://mainnet.infura.io/ws/v3/{}", _infura_key);
    let ws_provider_sepolia = Provider::<Ws>::connect(ws_url_sepolia).await?;
    let ws_provider = Provider::<Ws>::connect(ws_url).await?;

    // bundle signing
    let _bundle_signer = env::var("FLASHBOTS_IDENTIFIER").clone().unwrap();
    let bundle_signer = _bundle_signer.parse::<LocalWallet>();
    //let addr = _bundle_signer.address();

    let _wallet = env::var("FLASHBOTS_SIGNER").clone().unwrap();
    let wallet = _wallet.parse::<LocalWallet>().unwrap();

    println!("Bundle Signer: {}", _bundle_signer);
    println!("Wallet: {}", _wallet);

    let client_sepolia = SignerMiddleware::new(
        FlashbotsMiddleware::new(
            http_provider,
            Url::parse("https://relay-sepolia.flashbots.net")?,
            bundle_signer.unwrap(),
        ),
        wallet,
    );

    let client = SignerMiddleware::new(
        FlashbotsMiddleware::new(
            http_provider,
            Url::parse("https://relay.flashbots.net")?,
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
    while let Some(tx_hash) = stream.next().await {
        let mut msg = ws_provider.get_transaction(tx_hash).await?;
        println!(
            // "Timestamp: {:?}, block number: {} -> {:?}", 
            // block.timestamp,
            // block.number.unwrap(),
            // block.hash.unwrap()
            // msg
            "{:?}", msg
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
    /*
Some(Transaction { 
    hash: 0x69be97c0cda4d872ee5868bfcd7671a8acb915a4f7a7edba7b76f4d08dad7c08, 
    nonce: 8848, 
    block_hash: None, 
    block_number: None, 
    transaction_index: None, 
    from: 0x007ab5199b6c57f7aa51bc3d0604a43505501a0c, 
    to: Some(0x0328adcc26d7ee6a71843bdbf716b7b2b0b4ffa3), 
    value: 1000000000000000, 
    gas_price: Some(1500000014), 
    gas: 21000, 
    input: Bytes(0x), 
    v: 0, 
    r: 108853379393575403114648895570501267920446440064797523802631923820109603901592, 
    s: 54432401851499039607746644666317390761051500462096442063488962792652443980717, 
    transaction_type: Some(2), 
    access_list: Some(AccessList([])), 
    max_priority_fee_per_gas: Some(1500000000), 
    max_fee_per_gas: Some(1500000014), 
    chain_id: Some(11155111), 
    other: OtherFields { inner: {} } })

Some(Transaction { 
    hash: 0x736a6a6abea12465de1cc4d73e9d0633203d30170aadf826a77c3f6d154d7732, 
    nonce: 21000, 
    block_hash: None, block_number: None, transaction_index: None, 
    from: 0xaabb8c0deb1270151b9b0776bbf9c890cd877e67, 
    to: Some(0x53844f9577c2334e541aec7df7174ece5df1fcf0), 
    value: 0, 
    gas_price: Some(1000000000), 
    gas: 3000000, 
    input: Bytes(0x9ed93318000000000000000000000000aabb8c0deb1270151b9b0776bbf9c890cd877e67), 
    v: 22310258, 
    r: 52120736319054986821632375614197023773675339739065370707613673340985280157230, 
    s: 2345234057493645648343460763188633139081893918229592397379631637369603903814, transaction_type: Some(0), access_list: None, max_priority_fee_per_gas: None, max_fee_per_gas: None, chain_id: Some(11155111), other: OtherFields { inner: {} } })

    // monitor works
    // monitor specific address
    // parse the data to only the functions we want ... bytes4(keccak256(function_name(paramenters)))
    */



    Ok(())
}