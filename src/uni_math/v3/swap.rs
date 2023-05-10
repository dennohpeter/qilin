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

    use crate::bindings::{
        uniswap_v3_router_1::uni_v3_swap_router_1_contract,
        uniswap_v3_weth_dai_lp::uniswap_v3_weth_dai_lp_contract, weth::weth_contract,
    };
    use crate::utils::constants::{
        DAI_ADDRESS, UNISWAP_V3_ROUTER_1, UNISWAP_V3_WETH_DAI_LP, WETH_ADDRESS,
    };
    use ethers::{
        abi::AbiDecode,
        core::utils::Anvil,
        middleware::SignerMiddleware,
        prelude::{abigen, Abigen},
        providers::{Http, Middleware, Provider},
        signers::{LocalWallet, Signer},
        types::{TransactionReceipt, H160},
    };
    use std::env;
    use std::error::Error;
    use std::sync::Arc;

    use ethers::{
        types::{NameOrAddress, U256},
        utils::parse_units,
    };
    use hex;

    use crate::uni_math::v3::utils::v3_get_ticks;

    #[tokio::test]
    async fn test_swap() -> Result<(), Box<dyn Error>> {
        let FIVE_HUNDRED_ETHER: U256 = U256::from(parse_units("500.0", "ether").unwrap());
        // create a LocalWallet instance from local node's available account's private key
        let wallet: LocalWallet =
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                .parse::<LocalWallet>()?;
        let provider = Provider::<Http>::try_from("http://localhost:8545")?;

        // create a singermiddleware that wraps default provider and wallet
        let client = Arc::new(SignerMiddleware::new(provider, wallet));
        let clone_client = Arc::clone(&client);

        // get account address
        let address = Arc::clone(&client).address();
        println!("Account Address: {:?}", address);

        //get account balance
        let balance = Arc::clone(&client)
            .get_balance(address.clone(), None)
            .await?;
        println!("Wallet Balance: {:?}", balance);

        // create an instance of WETH smart contract fomr binding
        let weth_instance =
            weth_contract::weth::new(WETH_ADDRESS.parse::<H160>()?, Arc::clone(&client));

        // deposit 500 ETH to get WETH
        let _weth = weth_instance
            .deposit()
            .value(FIVE_HUNDRED_ETHER)
            .send()
            .await?
            .await?
            .expect("no receipt found");
        let weth_balance = weth_instance.balance_of(address).call().await?;
        println!("Current WETH Balance: {:?}", weth_balance);

        // create an instance of WETH/DAI smart contract from bindings
        let _weth_dai_lp = uniswap_v3_weth_dai_lp_contract::uniswap_v3_weth_dai_lp::new(
            UNISWAP_V3_WETH_DAI_LP.parse::<H160>()?,
            Arc::clone(&client),
        );

        // get current WETH/DAI pool's info
        let (
            // current price
            sqrt_price_x_96,
            // current tick
            tick,
            _,
            _,
            _,
            // pool fee
            fee,
            _,
        ) = _weth_dai_lp.slot_0().call().await?;

        let tick_spacing = _weth_dai_lp.tick_spacing().call().await?;
        println!("WETH/DAI V3 Pool sqrtPriceX96: {:?}", sqrt_price_x_96);
        println!("WETH/DAI V3 Pool Current Tick: {:?}", tick);
        println!("WETH/DAI V3 Pool Fee: {:?}", fee);
        println!("WETH/DAI V3 Tick Spacing: {:?}", tick_spacing);

        // get upper and lower ticks
        let (lower_tick, upper_tick) = v3_get_ticks(tick, tick_spacing);
        println!(
            "Current Upper Tick: {:?}, Lower Tick: {:?}",
            upper_tick, lower_tick
        );

        // create an instance of router smart contract from the bindingd
        let uni_v3_router_1 = uni_v3_swap_router_1_contract::SwapRouter::new(
            UNISWAP_V3_ROUTER_1.parse::<H160>()?,
            Arc::clone(&client),
        );

        let _ = weth_instance
            .approve(UNISWAP_V3_ROUTER_1.parse::<H160>()?, U256::MAX)
            .send()
            .await?
            .await?;

        let input_param = uni_v3_swap_router_1_contract::ExactInputSingleParams {
            token_in: WETH_ADDRESS.parse::<H160>().unwrap(),
            token_out: DAI_ADDRESS.parse::<H160>().unwrap(),
            fee: 3000,                  //fee
            recipient: address.clone(), //recipient
            deadline: U256::MAX,
            amount_in: U256::from(parse_units("50.0", "ether").unwrap()), //amount in
            amount_out_minimum: U256::from(0),                            // amount out minimum
            sqrt_price_limit_x96: U256::from(0),                          //deadline
        };

        // let data = "08c379a0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000135472616e73616374696f6e20746f6f206f6c6400000000000000000000000000";
        // let data = "08c379a0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000035354460000000000000000000000000000000000000000000000000000000000";
        // let bytes = hex::decode(&data[130..])?;
        // if let Ok(s) = std::str::from_utf8(&bytes) {
        //     println!("Decoded string: {}", s);
        // } else {
        //     println!("Could not decode string");
        // }

        let amount_out_res = uni_v3_router_1
            .exact_input_single(input_param)
            .send()
            .await?
            .await?;

        // println!("{:?}", amount_out_res);

        //get after swap pool info
        let (
            // current price
            sqrt_price_x_96_after,
            // current tick
            tick_after,
            _,
            _,
            _,
            _,
            _,
        ) = _weth_dai_lp.slot_0().call().await?;
        println!("WETH/DAI V3 Pool sqrtPriceX96 after trade: {:?}", sqrt_price_x_96_after);
        println!("WETH/DAI V3 Pool Current Tick: {:?}", tick_after);

        Ok(())
    }
}
