use std::collections::HashMap;
use ethers::types::U256;
use ethers::prelude::*;
use crate::eth_market::{EthMarket, CrossedMarketDetails};
use crate::utils::{ETHER, big_number_to_decimal};

use crate::markets_by_token::MarketsByToken;
use crate::eth_market::EthMarket;
use crate::utils::{BigNumber, WETH_ADDRESS, ETHER};
use std::collections::BTreeMap;

pub type MarketsByToken = HashMap<String, Vec<Box<dyn EthMarket>>>;

const TEST_VOLUMES: [U256; 9] = [
    ETHER / 100.into(),
    ETHER / 10.into(),
    ETHER / 6.into(),
    ETHER / 4.into(),
    ETHER / 2.into(),
    ETHER,
    ETHER * 2.into(),
    ETHER * 5.into(),
    ETHER * 10.into(),
];

pub fn get_best_crossed_market(crossed_markets: &Vec<Vec<Box<dyn EthMarket>>>, token_address: &str) -> Option<CrossedMarketDetails> {
    // ...
}

pub struct Arbitrage {
    flashbots_provider: FlashbotsBundleProvider,
    bundle_executor_contract: Contract,
    executor_wallet: Wallet,
}

impl Arbitrage {
    fn new(executor_wallet: Wallet, flashbots_provider: FlashbotsBundleProvider, bundle_executor_contract: Contract) -> Self {
        Arbitrage {
            executor_wallet,
            flashbots_provider,
            bundle_executor_contract,
        }
    }

    fn print_crossed_market(crossed_market: &CrossedMarketDetails) {
        let buy_tokens = &crossed_market.buy_from_market.tokens;
        let sell_tokens = &crossed_market.sell_to_market.tokens;

        println!(
            "Profit: {} Volume: {}\n{} ({})\n  {} => {}\n{} ({})\n  {} => {}\n\n",
            big_number_to_decimal(&crossed_market.profit),
            big_number_to_decimal(&crossed_market.volume),
            crossed_market.buy_from_market.protocol,
            crossed_market.buy_from_market.marketAddress,
            buy_tokens[0],
            buy_tokens[1],
            crossed_market.sell_to_market.protocol,
            crossed_market.sell_to_market.marketAddress,
            sell_tokens[0],
            sell_tokens[1]
        );
    }

    // ...

    async fn evaluate_markets(&self, markets_by_token: MarketsByToken) -> Vec<CrossedMarketDetails> {
        let mut best_crossed_markets = Vec::new();

        for (token_address, markets) in markets_by_token {
            let priced_markets = markets.into_iter().map(|eth_market: EthMarket| {
                (
                    eth_market.clone(),
                    eth_market.get_tokens_in(&token_address, &WETH_ADDRESS, ETHER.div(100)),
                    eth_market.get_tokens_out(&WETH_ADDRESS, &token_address, ETHER.div(100)),
                )
            }).collect::<Vec<_>>();

            let mut crossed_markets = Vec::new();
            for (priced_market, buy_token_price, _) in &priced_markets {
                for (_, _, sell_token_price) in &priced_markets {
                    if sell_token_price > buy_token_price {
                        crossed_markets.push((priced_market.clone(), sell_token_price.clone()));
                    }
                }
            }

            if let Some(best_crossed_market) = get_best_crossed_market(&crossed_markets, &token_address) {
                if best_crossed_market.profit > ETHER.div(1000) {
                    best_crossed_markets.push(best_crossed_market);
                }
            }
        }

        best_crossed_markets.sort_by(|a, b| b.profit.cmp(&a.profit));
        best_crossed_markets
    }

    async fn take_crossed_markets(&mut self, best_crossed_markets: &[CrossedMarketDetails], block_number: u64, miner_reward_percentage: u64) -> Result<(), Box<dyn std::error::Error>> {
        // ...
    }
}
//TODO....stuff