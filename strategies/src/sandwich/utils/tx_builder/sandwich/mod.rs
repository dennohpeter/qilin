// ported directly from RustySando repo
// https://github.com/mouseless-eth/rusty-sando/blob/master/bot/src/utils/tx_builder/sandwich/mod.rs
use std::sync::Arc;

use crate::sandwich::state;
use ethers::prelude::{k256::ecdsa::SigningKey, *};
use parking_lot::RwLock;
pub mod v2;
pub mod v3;

#[derive(Debug, Clone)]
pub struct SandwichMaker {
    pub v2: v2::SandwichLogicV2,
    pub v3: v3::SandwichLogicV3,
    pub sandwich_address: Address,
    pub searcher_wallet: Wallet<SigningKey>,
    pub nonce: Arc<RwLock<U256>>,
}

impl SandwichMaker {
    // Create a new `SandwichMaker` instance
    pub async fn new(provider: Arc<Provider<Ws>>) -> Self {
        let sandwich_address = state::get_sandwich_contract_address();
        let searcher_wallet = state::get_searcher_wallet();

        let nonce = if let Ok(n) = provider
            .get_transaction_count(searcher_wallet.address(), None)
            .await
        {
            n
        } else {
            panic!("Failed to get searcher wallet nonce...");
        };

        let nonce = Arc::new(RwLock::new(nonce));

        Self {
            v2: v2::SandwichLogicV2::new(),
            v3: v3::SandwichLogicV3::new(),
            sandwich_address,
            searcher_wallet,
            nonce,
        }
    }
}

/// Return the divisor used for encoding call value (weth amount)
pub fn get_weth_encode_divisor() -> U256 {
    U256::from(100000)
}
