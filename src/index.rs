use dotenv::dotenv;
use std::env;
use ethers::core::{rand::thread_rng, types::transaction::eip2718::TypedTransaction};
use ethers::prelude::*;
use ethers::core::k256::SecretKey;
use ethers_flashbots::*;
use eyre::Result;
use std::convert::TryFrom;
use url::Url;
use ethers_signers::{LocalWallet, Signer};

#[tokio::main]
pub async fn first_fn() -> Result<()> {
    dotenv().ok(); 
    let provider = Provider::<Http>::try_from("https://mainnet.eth.aragon.network")?;

    let _bundle_signer = env::var("FLASHBOTS_IDENTIFIER").clone().unwrap();
    let bundle_signer = _bundle_signer.parse::<LocalWallet>();

    let _wallet = env::var("FLASHBOTS_SIGNER").clone().unwrap();
    let wallet = _wallet.parse::<LocalWallet>();

    println!("Hello, world!");
    println!("Bundle Signer: {}", _bundle_signer);
    println!("Wallet: {}", _wallet);

    let client = SignerMiddleware::new(
        FlashbotsMiddleware::new(
            provider,
            Url::parse("https://relay.flashbots.net")?,
            bundle_signer.unwrap(),
        ),
        wallet.unwrap(),
    );

    //println!("{}", client);

    // get last block number
    let block_number = client.get_block_number().await?;
    println!("Block Number: {}", block_number);

    // Build a custom bundle that pays 0x0000000000000000000000000000000000000000
    let tx = {
        let mut inner: TypedTransaction = TransactionRequest::pay(Address::zero(), 100).into();
        client.fill_transaction(&mut inner, None).await?;
        inner
    };
    

    let signature = client.signer().sign_transaction(&tx).await?;
    //println!("signature: {}", signature);
    let bundle = BundleRequest::new()
        .push_transaction(tx.rlp_signed(&signature))
        .set_block(block_number + 1)
        .set_simulation_block(block_number)
        .set_simulation_timestamp(0);

    // Simulate it
    let simulated_bundle = client.inner().simulate_bundle(&bundle).await?;
    //println!("Simulated bundle: {:?}", simulated_bundle);

    Ok(())
}