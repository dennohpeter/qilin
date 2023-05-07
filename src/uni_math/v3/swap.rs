use super::error::UniswapV3MathError;
use ethers::types::{I256, U256};
use ethers::{
    core::utils::Anvil,
    middleware::SignerMiddleware,
    prelude::{abigen, Abigen},
    providers::{Http, Provider},
    signers::LocalWallet,
};
use std::env;
use std::error::Error;
use std::sync::Arc;

// pub fn v3_swap(
//     zero_2_one: bool,
//     amount_speficied: I256,
//     sqrt_price_limit_x96: U256
// ) -> Result<(), Box<dyn Error>> {

// //     // steps:
// //     //  1)  determine exactInput or not: if amount_speficied > 0 then yes, otherwise no
// //     //  2)  cache the starting liquidity and tick
// //     //  3)  construct the initial swapping state:
// //     //      - state = {
// //     //                  "amountSpecifiedRemaining": amountSpecified,
// //     //                  "amountCalculated": 0,
// //     //                  "sqrtPriceX96": self.sqrt_price_x96,
// //     //                  "tick": self.tick,
// //     //                  "liquidity": cache["liquidityStart"],
// //     //                 }
// //     //  4) start walking through the liquidity ranges. Stop if either 1) each the limit or 2) amount_speficied is exhausted
// //     //      - find the next available tick
// //     //      - ensure don't overshoot the tick max/min value
// //     //      - compute values to swap to the target tick, price limit, or point where input/output amount is exhausted
// //     //      - shift to next tick if we reach the next price
// //     //      - if not exhausted, continure

//         Ok(())
// }

#[cfg(test)]
mod test {

    use ethers::{
        core::utils::Anvil,
        middleware::SignerMiddleware,
        prelude::{abigen, Abigen},
        providers::{Http, Provider},
        signers::LocalWallet,
    };
    use std::env;
    use std::error::Error;
    use std::sync::Arc;


    // pub fn initialize_test() {
    //     INIT.call_once(|| {
    //         setup_anvil().unwrap();
    //     });

    // }

    // fn setup_anvil() -> Result<(), Box<dyn Error>>{
    //     // let _infura_key = env::var("INFURA_API_KEY").clone().unwrap();
    // }


    #[tokio::test]
    async fn test_swap() -> Result<(), Box<dyn Error>> {
        // create a LocalWallet instance from local node's available account's private key
        let wallet: LocalWallet = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".parse::<LocalWallet>()?;

        let provider = Provider::<Http>::try_from("http://localhost:8548")?;
        println!("Provider connected to: {}", provider.url());
        SignerMiddleware::new(provider, wallet);
        assert_eq!(2 + 2, 4);
        Ok(())
    }
}
