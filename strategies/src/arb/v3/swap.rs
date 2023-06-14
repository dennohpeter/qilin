use super::errors::UniswapV3MathError;
use qilin_cfmms::batch_requests;
use qilin_cfmms::batch_requests::uniswap_v3::UniswapV3TickData;

use cfmms::errors::CFMMError;
use cfmms::pool::uniswap_v3::UniswapV3Pool;
use ethers::providers::{Provider, Ws};
use ethers::types::{I256, U256};
use std::sync::Arc;
use uniswap_v3_math::{liquidity_math, tick_math};

pub const MIN_SQRT_RATIO: U256 = U256([4295128739, 0, 0, 0]);
pub const MAX_SQRT_RATIO: U256 = U256([6743328256752651558, 17280870778742802505, 4294805859, 0]);

#[derive(Default, Clone)]
pub struct Step {
    pub sqrt_price_start_x96: U256,
    pub tick_next: i32,
    pub initialized: bool,
    pub sqrt_price_next_x96: U256,
    pub amount_in: U256,
    pub amount_out: U256,
    pub fee_amount: U256,
}

pub struct CurrentState {
    amount_specified_remaining: I256,
    amount_calculated: I256,
    sqrt_price_x96: U256,
    tick: i32,
    liquidity: u128,
}

pub struct Tick {
    pub liquidity_gross: u128,
    pub liquidity_net: i128,
    pub fee_growth_outside_0_x_128: U256,
    pub fee_growth_outside_1_x_128: U256,
    pub tick_cumulative_outside: U256,
    pub seconds_per_liquidity_outside_x_128: U256,
    pub seconds_outside: u32,
    pub initialized: bool,
}

pub async fn get_pool_data(
    uniswapv3_pool: UniswapV3Pool,
    zero_for_one: bool,
    provider: Arc<Provider<Ws>>,
) -> Result<
    (
        u128,
        u128,
        u32,
        u128,
        U256,
        i32,
        Vec<UniswapV3TickData>,
        i128,
    ),
    CFMMError<Provider<Ws>>,
> {
    let fee = uniswapv3_pool.fee;
    let sqrt_price = uniswapv3_pool.sqrt_price;
    let mut tick = uniswapv3_pool.tick;
    let liquidity = uniswapv3_pool.liquidity;
    let (reserver_0, reserve_1) = uniswapv3_pool.calculate_virtual_reserves().unwrap();

    let mut tick_data = Vec::new();
    while tick_data.len() < 500 {
        if let Ok((_tick_data, _)) =
            batch_requests::uniswap_v3::get_uniswap_v3_tick_data_batch_request(
                &uniswapv3_pool,
                tick,
                zero_for_one,
                // TODO: increase num_ticks iteratively
                200,
                None,
                provider.clone(),
            )
            .await
        {
            tick += 200;
            tick_data.extend(_tick_data);
        } else {
            return Err(CFMMError::NoInitializedTicks);
        };
    }

    let liquidity_net = uniswapv3_pool
        .get_liquidity_net(tick, provider.clone())
        .await
        .unwrap();

    Ok((
        reserver_0,
        reserve_1,
        fee,
        liquidity,
        sqrt_price,
        tick,
        tick_data,
        liquidity_net,
    ))
}

pub fn get_tokens_in_from_tokens_out(
    token0_out: Option<f64>,
    token1_out: Option<f64>,
    tick: &i32,
    sqrt_price: &U256,
    liquidity: &u128,
    liquidity_net: i128,
    tick_data: &Vec<UniswapV3TickData>,
    fee: &u32,
) -> Result<f64, UniswapV3MathError> {
    match token0_out {
        Some(val) => {
            if token1_out.is_some() {
                return Err("Cannot take two tokens").unwrap();
            };
            if let Ok((amt_0, _, _, _, _)) = swap(
                -val,
                tick,
                sqrt_price,
                liquidity,
                tick_data,
                liquidity_net,
                &false,
                fee,
            ) {
                return Ok(amt_0);
            } else {
                return Err(UniswapV3MathError::SwapSimulationError);
            }
        }
        None => match token1_out {
            Some(val) => {
                if let Ok((_, amt_1, _, _, _)) = swap(
                    -val,
                    tick,
                    sqrt_price,
                    liquidity,
                    tick_data,
                    liquidity_net,
                    &true,
                    fee,
                ) {
                    return Ok(amt_1);
                } else {
                    return Err(UniswapV3MathError::SwapSimulationError);
                }
            }
            None => Err("At least one token needs to be provided").unwrap(),
        },
    }
}

pub fn get_tokens_out_from_tokens_in(
    token0_in: Option<f64>,
    token1_in: Option<f64>,
    tick: &i32,
    sqrt_price: &U256,
    liquidity: &u128,
    liquidity_net: i128,
    tick_data: &Vec<UniswapV3TickData>,
    fee: &u32,
) -> Result<f64, UniswapV3MathError> {
    match token0_in {
        Some(val) => {
            if token1_in.is_some() {
                return Err("Cannot take two tokens").unwrap();
            };
            if let Ok((amt_0, _, _, _, _)) = swap(
                val,
                tick,
                sqrt_price,
                liquidity,
                tick_data,
                liquidity_net,
                &false,
                fee,
            ) {
                return Ok(amt_0);
            } else {
                return Err(UniswapV3MathError::SwapSimulationError);
            }
        }
        None => match token1_in {
            Some(val) => {
                if let Ok((_, amt_1, _, _, _)) = swap(
                    val,
                    tick,
                    sqrt_price,
                    liquidity,
                    tick_data,
                    liquidity_net,
                    &true,
                    fee,
                ) {
                    return Ok(amt_1);
                } else {
                    return Err(UniswapV3MathError::SwapSimulationError);
                }
            }
            None => Err("At least one token needs to be provided").unwrap(),
        },
    }
}

// function assumes getting exact amount out
pub fn swap(
    amount_in: f64,
    tick: &i32,
    sqrt_price_x96: &U256,
    liquidity: &u128,
    tick_data: &Vec<UniswapV3TickData>,
    liquidity_net: i128,
    zero_for_one: &bool,
    fee: &u32,
) -> Result<(f64, f64, U256, u128, i32), UniswapV3MathError> {
    let mut tick_data_iter = tick_data.iter();
    let mut liquidity_net = liquidity_net.clone();

    let mut state = CurrentState {
        // type case f64 to i128 for I256 convertion
        // might lead to loss of precision
        amount_specified_remaining: I256::from(amount_in as i128),
        amount_calculated: I256::from(0),
        sqrt_price_x96: *sqrt_price_x96,
        tick: *tick,
        liquidity: *liquidity,
    };

    let sqrt_price_limit_x96 = if *zero_for_one {
        MIN_SQRT_RATIO + 1
    } else {
        MAX_SQRT_RATIO - 1
    };

    let exact_input = amount_in > 0.0 as f64;

    while state.amount_specified_remaining != I256::zero()
        && state.sqrt_price_x96 != sqrt_price_limit_x96
    {
        let mut step = Step {
            sqrt_price_start_x96: state.sqrt_price_x96,
            ..Default::default()
        };

        let next_tick_data = if let Some(tick_data) = tick_data_iter.next() {
            tick_data
        } else {
            // currently return if tick_data is exhausted
            // later should add a function to return a HashMap that represents the tick_bitmap
            return Err(UniswapV3MathError::TickDataError);
        };

        // TODO: add a tick_bitmap finder like balanceOf slot_finder to use here
        // let mut keep_searching = true;
        // while keep_searching {
        //     match tick_bitmap::next_initialized_tick_within_one_word(
        //         tick_bitmap,
        //         state.tick,
        //         tick_spacing,
        //         zero_for_one,
        //     ) {
        //         Ok((tick_next, initialized)) => {
        //             step.tick_next = tick_next.clamp(tick_math::MIN_TICK, tick_math::MAX_TICK);
        //             step.sqrt_price_next_x96 = tick_math::get_sqrt_ratio_at_tick(step.tick_next)?;
        //             step.initialized = initialized;
        //             if initialized {
        //                 keep_searching = false;
        //             };
        //         }
        //         Err(e) => return Err(Box::new(e)),
        //     };
        // }

        step.tick_next = next_tick_data.tick;

        // prevent overshooting
        if step.tick_next < tick_math::MIN_TICK {
            step.tick_next = tick_math::MIN_TICK;
        } else if step.tick_next > tick_math::MAX_TICK {
            step.tick_next = tick_math::MAX_TICK;
        };

        step.sqrt_price_next_x96 =
            match uniswap_v3_math::tick_math::get_sqrt_ratio_at_tick(step.tick_next) {
                Ok(val) => val,
                Err(e) => return Err(UniswapV3MathError::TickDataError),
            };

        match uniswap_v3_math::swap_math::compute_swap_step(
            state.sqrt_price_x96,
            if (*zero_for_one && step.sqrt_price_next_x96 < sqrt_price_limit_x96)
                || (!zero_for_one && step.sqrt_price_next_x96 > sqrt_price_limit_x96)
            {
                sqrt_price_limit_x96
            } else {
                step.sqrt_price_next_x96
            },
            state.liquidity,
            state.amount_specified_remaining,
            *fee,
        ) {
            Ok((sqrt_price_x96, amount_in, amount_out, fee_amount)) => {
                state.sqrt_price_x96 = sqrt_price_x96;
                step.amount_in = amount_in;
                step.amount_out = amount_out;
                step.fee_amount = fee_amount;
            }

            Err(_) => return Err(UniswapV3MathError::StepComputationError),
        }

        if exact_input {
            state.amount_specified_remaining = state
                .amount_specified_remaining
                .overflowing_sub(I256::from_raw(
                    step.amount_in.overflowing_add(step.fee_amount).0,
                ))
                .0;
            state.amount_calculated -= I256::from_raw(step.amount_out);
        } else {
            state.amount_specified_remaining = state
                .amount_specified_remaining
                .overflowing_add(I256::from_raw(step.amount_out))
                .0;
            state.amount_calculated = state
                .amount_calculated
                .overflowing_add(I256::from_raw(
                    step.amount_in.overflowing_add(step.fee_amount).0,
                ))
                .0;
        }

        // shift tick if we reached the next price
        if state.sqrt_price_x96 == step.sqrt_price_next_x96 {
            // if the tick is initialized, run the tick transition
            if next_tick_data.initialized {
                liquidity_net = next_tick_data.liquidity_net;

                if *zero_for_one {
                    liquidity_net = -liquidity_net;
                }

                state.liquidity = if liquidity_net < 0 {
                    state.liquidity - (-liquidity_net as u128)
                } else {
                    state.liquidity + (liquidity_net as u128)
                };
            };

            state.tick = if *zero_for_one {
                step.tick_next.wrapping_sub(1)
            } else {
                step.tick_next
            };
        } else if state.sqrt_price_x96 != step.sqrt_price_start_x96 {
            state.tick = match tick_math::get_tick_at_sqrt_ratio(state.sqrt_price_x96) {
                Ok(val) => val,
                Err(e) => return Err(UniswapV3MathError::TickDataError),
            };
        };
    }

    let (amount0, amount1) = if *zero_for_one == exact_input {
        (
            I256::from(amount_in as i128) - state.amount_specified_remaining,
            state.amount_calculated,
        )
    } else {
        (
            state.amount_calculated,
            I256::from(amount_in as i128) - state.amount_specified_remaining,
        )
    };

    Ok((
        amount0.as_i128() as f64,
        amount1.as_i128() as f64,
        state.sqrt_price_x96,
        state.liquidity,
        state.tick,
    ))
}

// #[cfg(test)]
// mod test {

//     use crate::bindings::{
//         uniswap_v3_router_1::uni_v3_swap_router_1_contract,
//         uniswap_v3_weth_dai_lp::uniswap_v3_weth_dai_lp_contract, weth::weth_contract,
//     };
//     use crate::uni_math::v3::utils::v3_get_ticks;
//     use crate::utils::constants::{
//         DAI_ADDRESS, UNISWAP_V3_ROUTER_1, UNISWAP_V3_WETH_DAI_LP, WETH_ADDRESS,
//     };
//     use ethers::types::U64;
//     use ethers::{
//         middleware::SignerMiddleware,
//         providers::{Http, Middleware, Provider},
//         signers::LocalWallet,
//         types::H160,
//     };
//     use ethers::{types::U256, utils::parse_units};
//     use std::error::Error;
//     use std::sync::Arc;

//     #[tokio::test]
//     // #[ignore]
//     async fn test_swap() -> Result<(), Box<dyn Error>> {
//         let five_hundred_ether: U256 = U256::from(parse_units("500.0", "ether").unwrap());
//         // create a LocalWallet instance from local node's available account's private key
//         let wallet: LocalWallet =
//             "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
//                 .parse::<LocalWallet>()?;
//         let provider =
//             Provider::<Http>::try_from("http://localhost:8545").expect("Failed to create provider");

//         //let clone_provider = provi
//         // create a singermiddleware that wraps default provider and wallet
//         let client = Arc::new(SignerMiddleware::new(provider.clone(), wallet));

//         let block = provider.clone().get_block_number().await?;
//         let block_number: U64 = block.into();
//         println!("block: {block_number:?}");
//         // get account address
//         let address = Arc::clone(&client).address();
//         println!("Account Address: {:?}", address);

//         //get account balance
//         let balance = Arc::clone(&client)
//             .get_balance(address.clone(), None)
//             .await?;
//         println!("Wallet Balance: {:?}", balance);

//         // create an instance of WETH smart contract fomr binding
//         let weth_instance =
//             weth_contract::weth::new(WETH_ADDRESS.parse::<H160>()?, Arc::clone(&client));

//         let balance_of = weth_instance
//             .balance_of(
//                 "0x06da0fd433C1A5d7a4faa01111c044910A184553"
//                     .parse::<H160>()
//                     .unwrap(),
//             )
//             .call()
//             .await?;
//         println!(
//             "Balance of 0x06da0fd433C1A5d7a4faa01111c044910A184553: {:?}",
//             balance_of
//         );
//         //println!("500: {:?}", five_hundred_ether);

//         // deposit 500 ETH to get WETH
//         let _weth = weth_instance
//             .deposit()
//             .value(five_hundred_ether)
//             .send()
//             .await?
//             .await?
//             .expect("no receipt found");
//         let weth_balance = weth_instance.balance_of(address).call().await?;
//         println!("Current WETH Balance: {:?}", weth_balance);

//         // create an instance of WETH/DAI smart contract from bindings
//         let _weth_dai_lp = uniswap_v3_weth_dai_lp_contract::uniswap_v3_weth_dai_lp::new(
//             UNISWAP_V3_WETH_DAI_LP.parse::<H160>()?,
//             Arc::clone(&client),
//         );

//         // get current WETH/DAI pool's info
//         let (
//             // current price
//             sqrt_price_x_96,
//             // current tick
//             tick,
//             _,
//             _,
//             _,
//             // pool fee
//             fee,
//             _,
//         ) = _weth_dai_lp.slot_0().call().await?;

//         let tick_spacing = _weth_dai_lp.tick_spacing().call().await?;
//         println!("WETH/DAI V3 Pool sqrtPriceX96: {:?}", sqrt_price_x_96);
//         // let f64_sqrt = sqrt_price_x_96.as_u128() as f64;
//         // println!("WETH/DAI V3 Pool sqrtPriceX96 f64: {:?}", f64_sqrt);
//         // let u128_sqrt = f64_sqrt as u128;
//         // println!("WETH/DAI V3 Pool sqrtPriceX96 128: {:?}", u128_sqrt);
//         // println!("WETH/DAI V3 Pool sqrtPriceX96 U256: {:?}", U256::from(u128_sqrt));
//         println!("WETH/DAI V3 Pool Current Tick: {:?}", tick);
//         println!("WETH/DAI V3 Pool Fee: {:?}", fee);
//         println!("WETH/DAI V3 Tick Spacing: {:?}", tick_spacing);

//         // get upper and lower ticks
//         let (lower_tick, upper_tick) = v3_get_ticks(tick, tick_spacing);
//         println!(
//             "Current Upper Tick: {:?}, Lower Tick: {:?}",
//             upper_tick, lower_tick
//         );

//         // create an instance of router smart contract from the bindingd
//         let uni_v3_router_1 = uni_v3_swap_router_1_contract::SwapRouter::new(
//             UNISWAP_V3_ROUTER_1.parse::<H160>()?,
//             Arc::clone(&client),
//         );

//         let _ = weth_instance
//             .approve(UNISWAP_V3_ROUTER_1.parse::<H160>()?, U256::MAX)
//             .send()
//             .await?
//             .await?;

//         let input_param = uni_v3_swap_router_1_contract::ExactInputSingleParams {
//             token_in: WETH_ADDRESS.parse::<H160>().unwrap(),
//             token_out: DAI_ADDRESS.parse::<H160>().unwrap(),
//             fee: 3000,                  //fee
//             recipient: address.clone(), //recipient
//             deadline: U256::MAX,
//             amount_in: U256::from(parse_units("50.0", "ether").unwrap()),
//             amount_out_minimum: U256::from(0),
//             sqrt_price_limit_x96: U256::from(0),
//         };

//         let amount_out_res = uni_v3_router_1
//             .exact_input_single(input_param)
//             .send()
//             .await?
//             .await?;

//         println!("{:?}", amount_out_res);

//         //get after swap pool info
//         let (
//             // current price
//             sqrt_price_x_96_after,
//             // current tick
//             tick_after,
//             _,
//             _,
//             _,
//             _,
//             _,
//         ) = _weth_dai_lp.slot_0().call().await?;
//         println!(
//             "WETH/DAI V3 Pool sqrtPriceX96 after trade: {:?}",
//             sqrt_price_x_96_after
//         );
//         println!("WETH/DAI V3 Pool Current Tick: {:?}", tick_after);

//         Ok(())
//     }
// }
