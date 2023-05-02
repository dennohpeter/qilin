use ethers::{types::U256, wallet::Wallet, prelude::Lazy};
use std::str::FromStr;

pub const ETHER: Lazy<U256> = Lazy::new(|| U256::from(10).pow(U256::from(18)));

pub fn big_number_to_decimal(value: &U256, base: u32) -> f64 {
    let divisor = U256::from(10).pow(U256::from(base));
    let result = value * U256::from(10000) / divisor;
    result.as_u64() as f64 / 10000.0
}

pub fn get_default_relay_signing_key() -> String {
    println!("You have not specified an explicity FLASHBOTS_RELAY_SIGNING_KEY environment variable. Creating random signing key, this searcher will not be building a reputation for next run");
    Wallet::random().to_string()
}
