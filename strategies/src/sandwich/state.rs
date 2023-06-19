// ported directly from RustySando repo
// https://github.com/mouseless-eth/rusty-sando/blob/master/bot/src/runner/state.rs
use std::sync::Arc;

use dotenv;
use ethers::prelude::*;
use eyre::Result;
use log;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::str::FromStr;

use crate::sandwich::{abi::Erc20, utils};

#[derive(Clone, Debug)]
/// Holds the state of the bot
pub struct BotState {
    pub token_dust: Arc<RwLock<Vec<Address>>>,
    pub weth_balance: Arc<RwLock<U256>>,
}

impl BotState {
    // Create a new instance of the bot state
    //
    // Arguments:
    // * `sandwich_inception_block`: block number sandwich was deployed
    // * `client`: websocket provider to use for fetching data
    // * `test`: true if in test environment, false otherwise
    // * `sandwich_address`: address of sandwich contract needs to be provided if in test environment
    //
    // Returns:
    // Ok(BotState) if successful
    // Err(eyre::Error) if failed to create instance
    pub async fn new<M>(
        sandwich_inception_block: U64, 
        client: &Arc<M>, 
        test: bool, 
        sandwich_address: Option<Address>
    ) -> Result<Self>
    where
        M: Middleware + 'static,
        M::Provider: PubsubClient,
        M::Provider: JsonRpcClient,
    {
        let token_dust = Self::find_all_dust(sandwich_inception_block, client, test, sandwich_address).await?;
        let token_dust = Arc::new(RwLock::new(token_dust));

        let sandy_addr = get_sandy_addr(test, sandwich_address);

        let weth_contract =
            utils::contracts::get_erc20_contract(&utils::constants::get_weth_address(), client);
        let weth_balance = weth_contract
            .balance_of(sandy_addr)
            .call()
            .await?;
        let weth_balance = Arc::new(RwLock::new(weth_balance));

        Ok(BotState {
            token_dust,
            weth_balance,
        })
    }

    // Check if contract has dust for specific token
    //
    // Arguments:
    // * `&self`: refernce to `BotState` instance
    // * `token`: token to check dust for
    //
    // Returns:
    // bool: true if contract has dust for token, false otherwise
    pub async fn has_dust(&self, token: &Address) -> bool {
        self.token_dust.read().contains(token)
    }

    // Add dust to contract
    //
    // Arguments:
    // * `&self`: reference to `BotState` instance
    // * `token`: token to add dust for
    pub async fn add_dust(&self, token: Address) {
        let mut dust = self.token_dust.write();
        dust.push(token);
    }

    // Update the WETH balance of the contract
    //
    // Arguments:
    // * `&self`: reference to `BotState` instance
    //
    // Returns: nothing
    pub async fn update_weth_balance(&self, value_to_add: U256) {
        let mut lock = self.weth_balance.write();
        *lock += value_to_add;
    }

    // Find dust that bot has collected from a specific block onwards
    //
    // Arguments:
    // * `start_block`: block to start searching for dust
    // * `client`: websocket provider to use for fetching data
    //
    // Returns:
    // `Ok(Vec<Address>)`: address of token dust collected by bot
    // `Err(eyre::Error)`: failed to find dust
    async fn find_all_dust<M>(
        start_block: U64, 
        client: &Arc<M>,
        test: bool,
        sandwich_address: Option<Address>
    ) -> Result<Vec<Address>>
    where
        M: Middleware + 'static,
        M::Provider: PubsubClient,
        M::Provider: JsonRpcClient,
    {
        let sandy_addr = get_sandy_addr(test, sandwich_address);

        // Define the step for searching a range of block logs for transfer events
        let step = 10000;

        // Find dust upto this block
        let current_block = match client.get_block_number().await {
            Ok(block) => block.as_u64(),
            Err(e) => {
                log::error!("Failed to get current_block {:?}", e);
                eyre::bail!("todo error msg here");
            }
        };

        let start_block = start_block.as_u64();

        // holds erc20 and associated balance
        let mut address_interacted_with = HashSet::new();

        // for each block within the range, get all transfer events asynchronously
        for from_block in (start_block..=current_block).step_by(step) {
            let to_block = from_block + step as u64;

            // check for all incoming and outgoing txs within step range
            let transfer_logs = client
                .get_logs(
                    &Filter::new()
                        .topic0(utils::constants::get_erc20_transfer_event_signature())
                        .topic1(sandy_addr.clone())
                        .from_block(BlockNumber::Number(U64([from_block])))
                        .to_block(BlockNumber::Number(U64([to_block]))),
                )
                .await?;

            let receive_logs = client
                .get_logs(
                    &Filter::new()
                        .topic0(utils::constants::get_erc20_transfer_event_signature())
                        .topic2(sandy_addr.clone())
                        .from_block(BlockNumber::Number(U64([from_block])))
                        .to_block(BlockNumber::Number(U64([to_block]))),
                )
                .await?;

            // combine all logs
            for log in transfer_logs {
                address_interacted_with.insert(log.address);
            }
            for log in receive_logs {
                address_interacted_with.insert(log.address);
            }
        }

        let mut token_dust = vec![];

        // doing calls to remove false positives
        for touched_addr in address_interacted_with {
            let erc20 = Erc20::new(touched_addr, client.clone());
            let balance: U256 = erc20.balance_of(sandy_addr).await?;

            if !balance.is_zero() {
                token_dust.push(touched_addr);
            }
        }

        log::info!("Found {:?} tokens worth of dust", token_dust.len());

        Ok(token_dust)
    }
}

/// Returns the configured Sandwich Contract Address
pub fn get_sandwich_contract_address() -> Address {
    dotenv::dotenv().ok();
    let addr = std::env::var("SANDWICH_CONTRACT")
        .expect("Required environment variable \"SANDWICH_CONTRACT\" not set");
    Address::from_str(&addr).expect("Failed to parse \"SANDWICH_CONTRACT\"")
}
/// Construct the searcher wallet
pub fn get_searcher_wallet() -> LocalWallet {
    dotenv::dotenv().ok();
    let searcher_private_key = std::env::var("PRIVATE_KEY")
        .expect("Required environment variable \"SEARCHER_PRIVATE_KEY\" not set");
    searcher_private_key
        .parse::<LocalWallet>()
        .expect("Failed to parse private key")
}

fn get_sandy_addr(test: bool, sandwich_address: Option<Address>) -> Address {
    let mut sandy_addr: Address;
    if test {
        sandy_addr = match sandwich_address {
            Some(addr) => addr,
            None => {
                panic!("Sandwich address not provided in testing environment");
            }
        }
    } else {
        sandy_addr = get_sandwich_contract_address();
    }
    sandy_addr
}