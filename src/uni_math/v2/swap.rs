use ethers::types::U256;
use std::error::Error;

/// takes either token 0 or 1 out, but not both
pub fn get_tokens_out_from_tokens_in(
    token0_in: Option<U256>,
    token1_in: Option<U256>,
    token0_reserve: U256,
    token1_reserve: U256,
) -> Result<U256, Box<dyn Error>> {
    match token0_in {
        Some(val) => {
            if token1_in.is_some() {
                return Err("Cannot take two tokens").unwrap();
            };

            if val.is_zero() {
                return Err("token0_in is zero").unwrap();
            };

	    let amount_in_with_fee = val * (U256::from(997));
            let result = (token1_reserve * &amount_in_with_fee)
                / (token0_reserve * U256::from(1000) + &amount_in_with_fee);
            return Ok(result);
        }
        None => match token1_in {
            Some(val) => {
                if val.is_zero() {
                    return Err("token1_in is zero").unwrap();
                };

		let amount_in_with_fee = val * (U256::from(997));
                let result = (token0_reserve *  &amount_in_with_fee)
                    / (token1_reserve * U256::from(1000) + &amount_in_with_fee);
                return Ok(result);
            }
            None => {
                return Err("At least one token needs to be provided").unwrap();
            }
        },
    }
}

pub fn get_tokens_in_from_tokens_out(
    token0_out: Option<U256>,
    token1_out: Option<U256>,
    token0_reserve: U256,
    token1_reserve: U256,
) -> Result<U256, Box<dyn Error>> {
    match token0_out {
        Some(val) => {
            if token1_out.is_some() {
                return Err("Cannot take two tokens").unwrap();
            };

            if val.is_zero() {
                return Err("token0_out is zero").unwrap();
            };

            let result = (token1_reserve * val) / ((token0_reserve - val) * (U256::from(997)));

            return Ok(result);
        }

        None => match token1_out {
            Some(val) => {
                if val.is_zero() {
                    return Err("token1_out is zero").unwrap();
                };

                let result =
                    (token0_reserve * val) / ((token1_reserve - val) * (U256::from(997)));

                return Ok(result);
            }
            None => {
                return Err("At least one token needs to be provided").unwrap();
            }
        },
    }
}
