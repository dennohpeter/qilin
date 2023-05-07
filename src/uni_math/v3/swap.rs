use ethers::types::{I256, U256};
use super::error::UniswapV3MathError;

// pub fn v3_swap(
//     zero_2_one: bool,
//     amount_speficied: I256,
//     sqrt_price_limit_x96: U256
// ) -> Result<(U256, U256), UniswapV3MathError> {

//     // steps:
//     //  1)  determine exactInput or not: if amount_speficied > 0 then yes, otherwise no
//     //  2)  cache the starting liquidity and tick
//     //  3)  construct the initial swapping state:
//     //      - state = {
//     //                  "amountSpecifiedRemaining": amountSpecified,
//     //                  "amountCalculated": 0,
//     //                  "sqrtPriceX96": self.sqrt_price_x96,
//     //                  "tick": self.tick,
//     //                  "liquidity": cache["liquidityStart"],
//     //                 }
//     //  4) start walking through the liquidity ranges. Stop if either 1) each the limit or 2) amount_speficied is exhausted
//     //      - find the next available tick 
//     //      - ensure don't overshoot the tick max/min value
//     //      - compute values to swap to the target tick, price limit, or point where input/output amount is exhausted
//     //      - shift to next tick if we reach the next price
//     //      - if not exhausted, continure

// }



#[cfg(test)]
mod test {

    use ethers::{
        providers::{Provider, Http},
        core::{utils::Anvil},
        signers::{LocalWallet},
        middleware::SignerMiddleware,
    };
    use std::sync::Arc;
    use std::error::Error;
    use std::env;


    
    #[tokio::test]
    async fn test_swap() -> Result<(), Box<dyn Error>> {

        let _infura_key = env::var("INFURA_API_KEY  ").clone().unwrap();
        let anvil = Anvil::new().fork(format!("https://mainnet.infura.io/v3/{}", _infura_key)).fork_block_number(15686252 as u64).spawn();

        let wallet: LocalWallet = anvil.keys()[0].clone().into();

        let provider = Arc::new({

            let provider = Provider::<Http>::try_from(anvil.endpoint())?;


            wallet.clone();

            SignerMiddleware::new(provider, wallet)
        });

        drop(anvil);
        assert_eq!(2+2, 4);
        Ok(())
        
    }

}