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
        providers::{Http, Middleware, Provider},
        signers::{LocalWallet, Signer},
        types::H160,
    };
    use std::env;
    use std::error::Error;
    use std::sync::Arc;
    use crate::bindings::{
        uniswap_v3_router_1, weth::weth_contract
    };
    use crate::utils::constants::{
        WETH_ADDRESS
    };

    use ethers::{
        types::{
            U256,
            NameOrAddress
        },
        utils::{
            parse_units
        },
    };

    #[tokio::test]
    async fn test_swap() -> Result<(), Box<dyn Error>> {

        let FIVE_HUNDRED_ETHER: U256 = U256::from(
            parse_units("500.0", "ether").unwrap()
        );
        // create a LocalWallet instance from local node's available account's private key
        let wallet: LocalWallet = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
            .parse::<LocalWallet>()?;
        let provider = Provider::<Http>::try_from("http://localhost:8545")?;

        let client = Arc::new(
            SignerMiddleware::new(
                provider,
                wallet
            )
        );
        let clone_client = Arc::clone(&client);
        let address = clone_client.address();
        println!("Account Address: {:?}", address);

        let mut balance = clone_client.get_balance(address.clone(), None).await?;
        println!("Wallet Balance: {:?}", balance);

        let weth_instance = weth_contract::weth::new(WETH_ADDRESS.parse::<H160>()?, client);
        // let decimals = weth_instance.decimals().call().await?;
        // deposit 1 ETH to get WETH
        let _weth = weth_instance.deposit().value(FIVE_HUNDRED_ETHER).send().await?.await?.expect("no receipt found");


        Ok(())
    }
}
