use std::collections::HashMap;
use ethers::types::U256;
use async_trait::async_trait;
use eth_market::{EthMarket, CrossedMarketDetails};
use utils::{ETHER, big_number_to_decimal};
use addresses::WETH_ADDRESS;
use std::cmp::Ordering;
use std::sync::Arc;
use futures::{stream, StreamExt};

type MarketsByToken = HashMap<String, Vec<Arc<dyn EthMarket>>>;

pub struct Arbitrage {
    // TODO: Define the fields for the Arbitrage struct
    // You need to implement the required structs and their methods for this.
}

impl Arbitrage {
    // TODO: Implement the constructor and other methods for the Arbitrage struct

    pub async fn evaluate_markets(markets_by_token: &MarketsByToken) -> Vec<CrossedMarketDetails> {
        let mut best_crossed_markets = Vec::new();

        for (token_address, markets) in markets_by_token {
            let priced_markets = markets
                .iter()
                .map(|eth_market| {
                    (
                        eth_market.clone(),
                        eth_market.get_tokens_in(token_address, &WETH_ADDRESS, ETHER / 100),
                        eth_market.get_tokens_out(&WETH_ADDRESS, token_address, ETHER / 100),
                    )
                })
                .collect::<Vec<_>>();

            let mut crossed_markets = Vec::new();
            for (eth_market, buy_token_price, _) in &priced_markets {
                for (_, _, sell_token_price) in &priced_markets {
                    if sell_token_price > buy_token_price {
                        crossed_markets.push((eth_market.clone(), sell_token_price));
                    }
                }
            }

            let best_crossed_market = get_best_crossed_market(crossed_markets, token_address).await;
            if let Some(best_crossed_market) = best_crossed_market {
                if best_crossed_market.profit > ETHER / 1000 {
                    best_crossed_markets.push(best_crossed_market);
                }
            }
        }

        best_crossed_markets.sort_unstable_by(|a, b| b.profit.cmp(&a.profit));

        best_crossed_markets
    }
}

pub async fn get_best_crossed_market(crossed_markets: Vec<(Arc<dyn EthMarket>, &U256)>, token_address: &str) -> Option<CrossedMarketDetails> {
    let test_volumes = vec![
        ETHER / 100,
        ETHER / 10,
        ETHER / 6,
        ETHER / 4,
        ETHER / 2,
        ETHER,
        ETHER * 2,
        ETHER * 5,
        ETHER * 10,
    ];

    let mut best_crossed_market: Option<CrossedMarketDetails> = None;

    for (sell_to_market, buy_from_market) in crossed_markets {
        for size in &test_volumes {
            let tokens_out_from_buying_size = buy_from_market.get_tokens_out(&WETH_ADDRESS, token_address, *size);
            let proceeds_from_selling_tokens = sell_to_market.get_tokens_out(token_address, &WETH_ADDRESS, tokens_out_from_buying_size);
            let profit = proceeds_from_selling_tokens.saturating_sub(*size);
            
            if let Some(current_best) = &best_crossed_market {
                if profit < current_best.profit {
                    let try_size = size.saturating_add(current_best.volume) / 2;
                    let try_tokens_out_from_buying_size = buy_from_market.get_tokens_out(&WETH_ADDRESS, token_address, try_size);
                    let try_proceeds_from_selling_tokens = sell_to_market.get_tokens_out(token_address, &WETH_ADDRESS, try_tokens_out_from_buying_size);
                    let try_profit = try_proceeds_from_selling_tokens.saturating_sub(try_size);
            
                    if try_profit > current_best.profit {
                        best_crossed_market = Some(CrossedMarketDetails {
                            volume: try_size,
                            profit: try_profit,
                            token_address: token_address.to_string(),
                            sell_to_market: sell_to_market.clone(),
                            buy_from_market: buy_from_market.clone(),
                        });
                    }
                    break;
                }
            }
            
            best_crossed_market = Some(CrossedMarketDetails {
                volume: *size,
                profit: profit,
                token_address: token_address.to_string(),
                sell_to_market: sell_to_market.clone(),
                buy_from_market: buy_from_market.clone(),
            });
        }
                
            best_crossed_market
    }
                
                // TODO: Implement the methods for the Arbitrage struct, such as `take_crossed_markets`
                