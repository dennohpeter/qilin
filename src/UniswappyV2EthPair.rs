// UniswappyV2EthPair.rs
use std::collections::HashMap;
use ethers::prelude::*;
use futures::future::join_all;
use ethers::types::{ BigNumber, Address };
use ethers::abi::Contract;
use serde::{ Deserialize, Serialize };
use std::error::Error;
use std::iter::repeat_with;
use std::sync::Arc;
use crate::abi::{ UNISWAP_PAIR_ABI, UNISWAP_QUERY_ABI };
use crate::addresses::{ UNISWAP_LOOKUP_CONTRACT_ADDRESS, WETH_ADDRESS };
use crate::eth_market::{ EthMarket, CallDetails, MultipleCallData };
use crate::token_balances::TokenBalances;
use crate::utils::{ ETHER, estimate_gas };
use crate::arbitrage::MarketsByToken;

// Constants
const BATCH_COUNT_LIMIT: u32 = 100;
const UNISWAP_BATCH_SIZE: u32 = 1000;
const BLACKLIST_TOKENS: &[&str] = &["0xD75EA151a61d06868E31F8988D28DFE5E9df57B4"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupedMarkets {
    markets_by_token: MarketsByToken,
    all_market_pairs: Vec<UniswappyV2EthPair>,
}

pub struct UniswappyV2EthPair {
    eth_market: EthMarket,
    token_balances: TokenBalances,
}

impl UniswappyV2EthPair {
    pub fn new(marketAddress: Address, tokens: Vec<Address>, protocol: String) -> Self {
        Self {
            eth_market: EthMarket::new(marketAddress, tokens.clone(), protocol),
            token_balances: tokens
                .into_iter()
                .zip(vec![BigNumber::from(0), BigNumber::from(0)])
                .collect(),
        }
    }

    pub fn receive_directly(&self, token_address: &Address) -> bool {
        self.token_balances.contains_key(token_address)
    }

    pub async fn prepare_receive(
        &self,
        token_address: &Address,
        amount_in: BigNumber
    ) -> Result<Vec<CallDetails>, Box<dyn std::error::Error>> {
        if !self.token_balances.contains_key(token_address) {
            return Err(format!("Market does not operate on token {}", token_address).into());
        }
        if amount_in <= BigNumber::from(0) {
            return Err(format!("Invalid amount: {}", amount_in).into());
        }

        // No preparation necessary
        Ok(vec![])
    }

    pub async fn get_uniswappy_markets(
        provider: Arc<Provider>,
        factory_address: Address
    ) -> Result<Vec<UniswappyV2EthPair>, Box<dyn std::error::Error>> {
        let uniswap_query = Contract::from_json(
            provider.clone(),
            UNISWAP_LOOKUP_CONTRACT_ADDRESS.parse()?,
            UNISWAP_QUERY_ABI
        )?;

        let mut market_pairs = Vec::new();

        for i in (0..BATCH_COUNT_LIMIT * UNISWAP_BATCH_SIZE).step_by(UNISWAP_BATCH_SIZE as usize) {
            let (pairs,): (Vec<(Address, Address, Address)>,) = uniswap_query.call(
                "getPairsByIndexRange",
                (factory_address, i, i + (UNISWAP_BATCH_SIZE as usize)),
                None,
                None
            ).await?;

            for pair in pairs {
                let market_address = pair.2;
                let token_address;

                if pair.0 == WETH_ADDRESS.parse()? {
                    token_address = pair.1;
                } else if pair.1 == WETH_ADDRESS.parse()? {
                    token_address = pair.0;
                } else {
                    continue;
                }

                if
                    !BLACKLIST_TOKENS.iter().any(
                        |&blacklist_token| blacklist_token == token_address.to_string().as_str()
                    )
                {
                    let eth_market = EthMarket::new(market_address);
                    let token_balances = TokenBalances::new(vec![pair.0, pair.1]);
                    let uniswappy_v2_eth_pair = UniswappyV2EthPair { eth_market, token_balances };
                    market_pairs.push(uniswappy_v2_eth_pair);
                }
            }

            if pairs.len() < (UNISWAP_BATCH_SIZE as usize) {
                break;
            }
        }

        Ok(market_pairs)
    }

    pub async fn get_uniswap_markets_by_token(
        provider: &Provider<Http>,
        factory_addresses: &[String]
    ) -> Result<GroupedMarkets, Box<dyn std::error::Error>> {
        let all_pairs_futures: Vec<_> = factory_addresses
            .iter()
            .map(|factory_address|
                UniswappyV2EthPair::get_uniswappy_markets(
                    provider.clone(),
                    factory_address.parse()?
                )
            )
            .collect::<Result<Vec<_>, _>>()?;

        let all_pairs = futures::future::join_all(all_pairs_futures).await;

        let markets_by_token_all = all_pairs
            .into_iter()
            .flatten()
            .fold(HashMap::new(), |mut acc, pair| {
                let key = if pair.tokens[0] == WETH_ADDRESS {
                    pair.tokens[1]
                } else {
                    pair.tokens[0]
                };
                acc.entry(key).or_insert_with(Vec::new).push(pair);
                acc
            });

        let all_market_pairs: Vec<UniswappyV2EthPair> = markets_by_token_all
            .values()
            .filter(|pairs| pairs.len() > 1)
            .flat_map(|pairs| pairs.clone())
            .collect();

        UniswappyV2EthPair::update_reserves(provider, &all_market_pairs).await?;

        let markets_by_token = all_market_pairs
            .iter()
            .filter(|pair| pair.get_balance(&WETH_ADDRESS).unwrap() > *ETHER)
            .fold(HashMap::new(), |mut acc, pair| {
                let key = if pair.tokens[0] == WETH_ADDRESS {
                    pair.tokens[1]
                } else {
                    pair.tokens[0]
                };
                acc.entry(key).or_insert_with(Vec::new).push(pair.clone());
                acc
            });

        Ok(GroupedMarkets {
            markets_by_token,
            all_market_pairs,
        })
    }

    pub async fn update_reserves(
        provider: &Provider<Http>,
        all_market_pairs: &[UniswappyV2EthPair]
    ) -> Result<(), Box<dyn std::error::Error>> {
        let uniswap_query = Contract::from_json(
            provider,
            UNISWAP_LOOKUP_CONTRACT_ADDRESS,
            UNISWAP_QUERY_ABI
        )?;
        let pair_addresses: Vec<Address> = all_market_pairs
            .iter()
            .map(|market_pair| market_pair.marketAddress)
            .collect();
        println!("Updating markets, count: {}", pair_addresses.len());
        let reserves: Vec<Vec<BigNumber>> = uniswap_query.function(
            "getReservesByPairs",
            &pair_addresses,
            provider
        ).await?;
        for (market_pair, reserve) in all_market_pairs.iter_mut().zip(reserves) {
            market_pair.set_reserves_via_ordered_balances(
                vec![reserve[0].clone(), reserve[1].clone()]
            );
        }
        Ok(())
    }

    pub fn get_balance(&self, token_address: &Address) -> Result<BigNumber, String> {
        self.token_balances.get(token_address).cloned().ok_or("bad token".to_string())
    }

    pub fn set_reserves_via_ordered_balances(&mut self, balances: Vec<BigNumber>) {
        self.set_reserves_via_matching_array(&self.tokens, balances);
    }

    pub fn set_reserves_via_matching_array(
        &mut self,
        tokens: &[Address],
        balances: Vec<BigNumber>
    ) {
        let token_balances: HashMap<Address, BigNumber> = tokens
            .iter()
            .cloned()
            .zip(balances.into_iter())
            .collect();
        if self.token_balances != token_balances {
            self.token_balances = token_balances;
        }
    }

    pub fn get_tokens_in(
        &self,
        token_in: &Address,
        token_out: &Address,
        amount_out: BigNumber
    ) -> Result<BigNumber, String> {
        let reserve_in = self.get_balance(token_in)?;
        let reserve_out = self.get_balance(token_out)?;
        Ok(self.get_amount_in(reserve_in, reserve_out, amount_out))
    }

    pub fn get_tokens_out(
        &self,
        token_in: &Address,
        token_out: &Address,
        amount_in: BigNumber
    ) -> Result<BigNumber, String> {
        let reserve_in = self.get_balance(token_in)?;
        let reserve_out = self.get_balance(token_out)?;
        Ok(self.get_amount_out(reserve_in, reserve_out, amount_in))
    }

    pub fn get_amount_in(
        &self,
        reserve_in: BigNumber,
        reserve_out: BigNumber,
        amount_out: BigNumber
    ) -> BigNumber {
        let numerator: BigNumber = reserve_in * amount_out * 1000;
        let denominator: BigNumber = (reserve_out - amount_out) * 997;
        numerator / denominator + 1
    }

    pub fn get_amount_out(
        &self,
        reserve_in: BigNumber,
        reserve_out: BigNumber,
        amount_in: BigNumber
    ) -> BigNumber {
        let amount_in_with_fee: BigNumber = amount_in * 997;
        let numerator = amount_in_with_fee * reserve_out;
        let denominator = reserve_in * 1000 + amount_in_with_fee;
        numerator / denominator
    }

    pub async fn sell_tokens_to_next_market(
        &self,
        token_in: &Address,
        amount_in: BigNumber,
        eth_market: &EthMarket
    ) -> Result<MultipleCallData, Box<dyn Error>> {
        if eth_market.receive_directly(token_in) {
            let exchange_call = self.sell_tokens(
                token_in,
                amount_in,
                &eth_market.marketAddress
            ).await?;
            Ok(MultipleCallData {
                data: vec![exchange_call],
                targets: vec![self.marketAddress],
            })
        } else {
            let exchange_call = self.sell_tokens(
                token_in,
                amount_in,
                &eth_market.marketAddress
            ).await?;
            Ok(MultipleCallData {
                data: vec![exchange_call],
                targets: vec![self.marketAddress],
            })
        }
    }

    pub async fn sell_tokens(
        &self,
        token_in: &Address,
        amount_in: BigNumber,
        recipient: &Address
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut amount0_out = BigNumber::from(0);
        let mut amount1_out = BigNumber::from(0);
        let token_out: Address;

        if token_in == &self.tokens[0] {
            token_out = self.tokens[1];
            amount1_out = self.get_tokens_out(token_in, &token_out, amount_in)?;
        } else if token_in == &self.tokens[1] {
            token_out = self.tokens[0];
            amount0_out = self.get_tokens_out(token_in, &token_out, amount_in)?;
        } else {
            return Err("Bad token input address".into());
        }

        let uniswap_interface = Contract::from_json(
            &self.provider,
            self.marketAddress,
            UNISWAP_PAIR_ABI
        )?;
        let calldata = uniswap_interface.encode(
            "swap",
            &(amount0_out, amount1_out, recipient, Vec::<u8>::new())
        )?;
        Ok(calldata)
    }

    // Don't forget to implement the necessary traits for your struct, such as `Debug`, `Clone`, `Serialize`, and `Deserialize`.
}