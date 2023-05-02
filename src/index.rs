use flashbots_ethers_provider_bundle::FlashbotsBundleProvider;
use ethers::prelude::{ Contract, Provider, Wallet };
use std::env;
use std::error::Error;
use reqwest::Client;
use tokio::task;

use crate::abi::BUNDLE_EXECUTOR_ABI;
use crate::uniswappy_v2_eth_pair::UniswappyV2EthPair;
use crate::addresses::FACTORY_ADDRESSES;
use crate::arbitrage::Arbitrage;
use crate::utils::get_default_relay_signing_key;

const ETHEREUM_RPC_URL: &str = "http://127.0.0.1:8545";
const MINER_REWARD_PERCENTAGE: i32 = 80;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let private_key = env::var("PRIVATE_KEY").unwrap_or("".to_string());
    let bundle_executor_address = env::var("BUNDLE_EXECUTOR_ADDRESS").unwrap_or("".to_string());
    let flashbots_relay_signing_key = env
        ::var("FLASHBOTS_RELAY_SIGNING_KEY")
        .unwrap_or(get_default_relay_signing_key());

    let healthcheck_url = env::var("HEALTHCHECK_URL").unwrap_or("".to_string());

    if private_key.is_empty() {
        eprintln!("Must provide PRIVATE_KEY environment variable");
        std::process::exit(1);
    }

    if bundle_executor_address.is_empty() {
        eprintln!(
            "Must provide BUNDLE_EXECUTOR_ADDRESS environment variable. Please see README.md"
        );
        std::process::exit(1);
    }

    if flashbots_relay_signing_key.is_empty() {
        eprintln!(
            "Must provide FLASHBOTS_RELAY_SIGNING_KEY. Please see https://github.com/flashbots/pm/blob/main/guides/searcher-onboarding.md"
        );
        std::process::exit(1);
    }

    let provider = Provider::connect(ETHEREUM_RPC_URL).await?;
    let arbitrage_signing_wallet = Wallet::from_private_key(&private_key)?;
    let flashbots_relay_signing_wallet = Wallet::from_private_key(&flashbots_relay_signing_key)?;

    println!("Searcher Wallet Address: {}", arbitrage_signing_wallet.address());
    println!(
        "Flashbots Relay Signing Wallet Address: {}",
        flashbots_relay_signing_wallet.address()
    );

    let flashbots_provider = FlashbotsBundleProvider::new(
        provider.clone(),
        flashbots_relay_signing_wallet
    ).await?;
    let arbitrage = Arbitrage::new(
        arbitrage_signing_wallet,
        flashbots_provider,
        Contract::new(
            bundle_executor_address.parse()?,
            BUNDLE_EXECUTOR_ABI.clone(),
            provider.clone()
        )
    );

    let markets = UniswappyV2EthPair::get_uniswap_markets_by_token(
        provider.clone(),
        &FACTORY_ADDRESSES
    ).await?;

    let _ = task::spawn(async move {
        while let Some(event) = provider.watch_blocks().await.unwrap().next().await {
            let block_number = event.unwrap().number.unwrap().as_u64();
            let _ = UniswappyV2EthPair::update_reserves(&provider, &markets.all_market_pairs).await;

            match arbitrage.evaluate_markets(&markets.markets_by_token).await {
                Ok(best_crossed_markets) => {
                    if best_crossed_markets.is_empty() {
                        println!("No crossed markets");
                        continue;
                    }

                    for crossed_market in &best_crossed_markets {
                        Arbitrage::print_crossed_market(crossed_market);
                    }

                    if
                        let Err(e) = arbitrage.take_crossed_markets(
                            &best_crossed_markets,
                            block_number,
                            MINER_REWARD_PERCENTAGE
                        ).await
                    {
                        eprintln!("Error taking crossed markets: {}", e);
                    }

                    if !healthcheck_url.is_empty() {
                        if let Err(e) = healthcheck(&healthcheck_url).await {
                            eprintln!("Healthcheck error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error evaluating markets: {}", e);
                }
            }
        }
    }).await;

    Ok(())
}

async fn healthcheck(healthcheck_url: &str) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let _ = client.get(healthcheck_url).send().await?;
    Ok(())
}