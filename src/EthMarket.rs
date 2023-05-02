use ethers::types::U256;
use async_trait::async_trait;
use std::collections::HashMap;

pub type TokenBalances = HashMap<String, U256>;

pub struct MultipleCallData {
    pub targets: Vec<String>,
    pub data: Vec<String>,
}

pub struct CallDetails {
    pub target: String,
    pub data: String,
    pub value: Option<U256>,
}

pub trait EthMarket {
    fn tokens(&self) -> &[String];

    fn market_address(&self) -> &str;

    fn protocol(&self) -> &str;

    fn get_tokens_out(&self, token_in: &str, token_out: &str, amount_in: U256) -> U256;

    fn get_tokens_in(&self, token_in: &str, token_out: &str, amount_out: U256) -> U256;

    async fn sell_tokens_to_next_market(&self, token_in: &str, amount_in: U256, eth_market: &dyn EthMarket) -> MultipleCallData;

    async fn sell_tokens(&self, token_in: &str, amount_in: U256, recipient: &str) -> String;

    fn receive_directly(&self, token_address: &str) -> bool;

    async fn prepare_receive(&self, token_address: &str, amount_in: U256) -> Vec<CallDetails>;
}

pub struct ExampleEthMarket {
    tokens: Vec<String>,
    market_address: String,
    protocol: String,
}

impl ExampleEthMarket {
    pub fn new(marketAddress: String, tokens: Vec<String>, protocol: String) -> Self {
        Self {
            tokens,
            market_address,
            protocol,
        }
    }
}

#[async_trait]
impl EthMarket for ExampleEthMarket {
    fn tokens(&self) -> &[String] {
        &self.tokens
    }

    fn market_address(&self) -> &str {
        &self.market_address
    }

    fn protocol(&self) -> &str {
        &self.protocol
    }

    // Implement the other methods as needed for your use case.
}
