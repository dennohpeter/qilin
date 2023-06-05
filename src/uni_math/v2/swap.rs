use crate::uni_math::u256_2_f64;
use cfmms::pool::uniswap_v2::UniswapV2Pool;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::U256;
use std::error::Error;
use std::sync::Arc;

pub async fn get_pool_data(pool: UniswapV2Pool, provider: Arc<Provider<Ws>>) -> (u128, u128) {
    let (token0, token1) = pool.get_reserves(provider).await.unwrap();
    (token0, token1)
}

/// takes either token 0 or 1 out, but not both
pub fn get_tokens_out_from_tokens_in(
    token0_in: Option<f64>,
    token1_in: Option<f64>,
    token0_reserve: &f64,
    token1_reserve: &f64,
) -> Result<f64, Box<dyn Error>> {
    match token0_in {
        Some(val) => {
            if token1_in.is_some() {
                return Err("Cannot take two tokens").unwrap();
            };

            if val == 0.0 {
                return Err("token0_in is zero").unwrap();
            };

            let amount_in_with_fee = val * (u256_2_f64(U256::from(997)));
            let result = (token1_reserve * &amount_in_with_fee)
                / (token0_reserve * u256_2_f64(U256::from(1000)) + &amount_in_with_fee);
            Ok(result)
        }
        None => match token1_in {
            Some(val) => {
                if val == 0.0 {
                    return Err("token1_in is zero").unwrap();
                };

                let amount_in_with_fee = val * (u256_2_f64(U256::from(997)));
                let result = (token0_reserve * &amount_in_with_fee)
                    / (token1_reserve * u256_2_f64(U256::from(1000)) + &amount_in_with_fee);
                Ok(result)
            }
            None => Err("At least one token needs to be provided").unwrap(),
        },
    }
}

pub fn get_tokens_in_from_tokens_out(
    token0_out: Option<f64>,
    token1_out: Option<f64>,
    token0_reserve: &f64,
    token1_reserve: &f64,
) -> Result<f64, Box<dyn Error>> {
    match token0_out {
        Some(val) => {
            if token1_out.is_some() {
                return Err("Cannot take two tokens").unwrap();
            };

            if val == 0.0 {
                return Err("token0_out is zero").unwrap();
            };

            let result =
                (token1_reserve * val) / ((token0_reserve - val) * (u256_2_f64(U256::from(997))));

            Ok(result)
        }

        None => match token1_out {
            Some(val) => {
                if val == 0.0 {
                    return Err("token1_out is zero").unwrap();
                };

                let result = (token0_reserve * val)
                    / ((token1_reserve - val) * (u256_2_f64(U256::from(997))));

                Ok(result)
            }
            None => Err("At least one token needs to be provided").unwrap(),
        },
    }
}
