// use super::error::UniswapV3MathError;
use super::{liquidity_math, swap_step, tick_bitmap, tick_math};
use ethers::types::{I256, U256};
use std::collections::HashMap;
use std::error::Error;

struct Cache {
    liquidity_start: u128,
    // tick_cumulative: i64,
}

struct State {
    amount_specified_remaining: I256,
    amount_calculated: I256,
    sqrt_price_x96: U256,
    tick: i32,
    liquidity: u128,
}

#[derive(Default)]
struct Step {
    sqrt_price_start_x96: U256,
    tick_next: i32,
    initialized: bool,
    sqrt_price_next_x96: U256,
    amount_in: U256,
    amount_out: U256,
    fee_amount: U256,
}

pub struct TickData {
    // tick: i32,
    liquidity_net: i32,
    // liquidity_gross: i32,
}

pub fn swap(
    zero_for_one: bool,
    amount_specified: I256,
    sqrt_price_limit_x96: U256,
    liquidity: u128,
    sqrt_price_x96: U256,
    fee: u32,
    tick: i32,
    tick_spacing: i32,
    tick_bitmap: &mut HashMap<i16, U256>,
    tick_data: &mut HashMap<i16, TickData>,
) -> Result<(I256, I256, U256, u128, i32), Box<dyn Error>> {
    if sqrt_price_limit_x96 == U256::zero() {
        if zero_for_one {
            tick_math::MIN_SQRT_RATIO + 1
        } else {
            tick_math::MAX_SQRT_RATIO - 1
        }
    } else {
        sqrt_price_limit_x96
    };

    let cache = Cache {
        liquidity_start: liquidity,
        // tick_cumulative: 0,
    };

    let exact_input = amount_specified > I256::zero();

    let mut state = State {
        amount_specified_remaining: amount_specified,
        amount_calculated: I256::zero(),
        sqrt_price_x96,
        tick,
        liquidity: cache.liquidity_start,
    };

    while state.amount_specified_remaining != I256::zero()
        && state.sqrt_price_x96 != sqrt_price_limit_x96
    {
        let mut step = Step {
            sqrt_price_start_x96: state.sqrt_price_x96,
            tick_next: 0,
            initialized: false,
            sqrt_price_next_x96: U256::zero(),
            amount_in: U256::zero(),
            amount_out: U256::zero(),
            fee_amount: U256::zero(),
        };

        step.sqrt_price_start_x96 = state.sqrt_price_x96;

        let mut keep_searching = true;

        while keep_searching {
            match tick_bitmap::next_initialized_tick_within_one_word(
                tick_bitmap,
                state.tick,
                tick_spacing,
                zero_for_one,
            ) {
                // TODO: if tick is in next word, we need to update the word in the bitmap
                Ok((tick_next, initialized)) => {
                    step.tick_next = tick_next.clamp(tick_math::MIN_TICK, tick_math::MAX_TICK);
                    step.sqrt_price_next_x96 = tick_math::get_sqrt_ratio_at_tick(step.tick_next)?;
                    step.initialized = initialized;
                    if initialized {
                        keep_searching = false;
                    };
                }
                Err(e) => return Err(Box::new(e)),
            };
        }

        // prevent overshooting
        if step.tick_next < tick_math::MIN_TICK {
            step.tick_next = tick_math::MIN_TICK;
        } else if step.tick_next > tick_math::MAX_TICK {
            step.tick_next = tick_math::MAX_TICK;
        };

        match swap_step::compute_swap_step(
            state.sqrt_price_x96,
            if (zero_for_one && step.sqrt_price_next_x96 < sqrt_price_limit_x96)
                || (!zero_for_one && step.sqrt_price_next_x96 > sqrt_price_limit_x96)
            {
                sqrt_price_limit_x96
            } else {
                step.sqrt_price_next_x96
            },
            state.liquidity,
            I256::from(state.liquidity),
            fee,
        ) {
            Ok((sqrt_price_x96, amount_in, amount_out, fee_amount)) => {
                state.sqrt_price_x96 = sqrt_price_x96;
                step.amount_in = amount_in;
                step.amount_out = amount_out;
                step.fee_amount = fee_amount;
            }

            Err(e) => return Err(Box::new(e)),
        }

        if exact_input {
            state.amount_specified_remaining -= I256::from_dec_str(&step.amount_in.to_string())
                .unwrap()
                + I256::from_dec_str(&step.fee_amount.to_string()).unwrap();
            state.amount_calculated -= I256::from_dec_str(&step.amount_out.to_string()).unwrap();
        } else {
            state.amount_specified_remaining +=
                I256::from_dec_str(&step.amount_out.to_string()).unwrap();
            state.amount_calculated = state.amount_calculated
                + I256::from_dec_str(&step.amount_in.to_string()).unwrap()
                + I256::from_dec_str(&step.fee_amount.to_string()).unwrap();
        }

        step.sqrt_price_next_x96 = tick_math::get_sqrt_ratio_at_tick(step.tick_next)?;

        // shift tick if we reached the next price
        if state.sqrt_price_x96 == step.sqrt_price_next_x96 {
            // if the tick is initialized, run the tick transition
            if step.initialized {
                let (next_word, _) = tick_bitmap::position(step.tick_next);
                let tick_ = tick_data.get(&next_word).ok_or("Failed to get tick data")?;
                let mut liquidity_net = tick_.liquidity_net;

                if zero_for_one {
                    liquidity_net = -liquidity_net;
                }

                state.liquidity =
                    liquidity_math::add_delta(state.liquidity, liquidity_net as i128)?;
            };

            state.tick = if zero_for_one {
                step.tick_next.wrapping_sub(1)
            } else {
                step.tick_next
            };
        } else if state.sqrt_price_x96 != step.sqrt_price_start_x96 {
            state.tick = tick_math::get_tick_at_sqrt_ratio(state.sqrt_price_x96)?;
        };
    }
    let (amount0, amount1) = if zero_for_one == exact_input {
        (
            amount_specified - state.amount_specified_remaining,
            state.amount_calculated,
        )
    } else {
        (
            state.amount_calculated,
            amount_specified - state.amount_specified_remaining,
        )
    };
    Ok((
        amount0,
        amount1,
        state.sqrt_price_x96,
        state.liquidity,
        state.tick,
    ))
}

#[cfg(test)]
mod test {

    use crate::bindings::{
        uniswap_v3_router_1::uni_v3_swap_router_1_contract,
        uniswap_v3_weth_dai_lp::uniswap_v3_weth_dai_lp_contract, weth::weth_contract,
    };
    use crate::uni_math::v3::utils::v3_get_ticks;
    use crate::utils::constants::{
        DAI_ADDRESS, UNISWAP_V3_ROUTER_1, UNISWAP_V3_WETH_DAI_LP, WETH_ADDRESS,
    };
    use ethers::{
        middleware::SignerMiddleware,
        providers::{Http, Middleware, Provider},
        signers::LocalWallet,
        types::H160,
    };
    use ethers::{types::U256, utils::parse_units};
    use std::error::Error;
    use std::sync::Arc;

    #[tokio::test]
    #[ignore]
    async fn test_swap() -> Result<(), Box<dyn Error>> {
        let five_hundred_ether: U256 = U256::from(parse_units("500.0", "ether").unwrap());
        // create a LocalWallet instance from local node's available account's private key
        let wallet: LocalWallet =
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                .parse::<LocalWallet>()?;
        let provider = Provider::<Http>::try_from("http://localhost:8545")?;

        // create a singermiddleware that wraps default provider and wallet
        let client = Arc::new(SignerMiddleware::new(provider, wallet));

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
            .value(five_hundred_ether)
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
            amount_in: U256::from(parse_units("50.0", "ether").unwrap()),
            amount_out_minimum: U256::from(0),
            sqrt_price_limit_x96: U256::from(0),
        };

        let amount_out_res = uni_v3_router_1
            .exact_input_single(input_param)
            .send()
            .await?
            .await?;

        println!("{:?}", amount_out_res);

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
        println!(
            "WETH/DAI V3 Pool sqrtPriceX96 after trade: {:?}",
            sqrt_price_x_96_after
        );
        println!("WETH/DAI V3 Pool Current Tick: {:?}", tick_after);

        Ok(())
    }
}
//paulclaudius
