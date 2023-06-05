pub mod v2;
pub mod v3;

use crate::batch_requests::uniswap_v3::UniswapV3TickData;
use crate::cfmm::pool::{Pool, PoolType};
use argmin::core::observers::{ObserverMode, SlogLogger};
use argmin::core::{CostFunction, Error, Executor};
use argmin::solver::brent::BrentOpt;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{H160, U256};
use std::sync::Arc;

struct ArbPool {
    borrowing_pool_reserve_0: f64,
    borrowing_pool_reserve_1: f64,
    repay_pool_reserve_0: f64,
    repay_pool_reserve_1: f64,
    borrowing_pool_type: PoolType,
    repay_pool_type: PoolType,
    borrow_0_buy_1: bool,
    borrowing_pool_fee: Option<u32>,         // V3 only
    borrowing_pool_liquidity: Option<u128>,  // V3 only
    borrowing_pool_sqrt_price: Option<U256>, // V3 only
    borrowing_pool_tick: Option<i32>,        // V3 only
    borrowing_pool_tick_data: Option<Vec<UniswapV3TickData>>,
    borrowing_pool_liquidity_net: Option<i128>, // V3 only
    repay_pool_fee: Option<u32>,                // V3 only
    repay_pool_liquidity: Option<u128>,         // V3 only
    repay_pool_sqrt_price: Option<U256>,        // V3 only
    repay_pool_tick: Option<i32>,               // V3 only
    repay_pool_tick_data: Option<Vec<UniswapV3TickData>>, // V3 only
    repay_pool_liquidity_net: Option<i128>,     // V3 only
}

impl ArbPool {
    fn new(
        borrowing_pool_reserve_0: f64,
        borrowing_pool_reserve_1: f64,
        repay_pool_reserve_0: f64,
        repay_pool_reserve_1: f64,
        borrowing_pool_type: PoolType,
        repay_pool_type: PoolType,
        borrow_0_buy_1: bool,
        borrowing_pool_fee: Option<u32>,         // V3 only
        borrowing_pool_liquidity: Option<u128>,  // V3 only
        borrowing_pool_sqrt_price: Option<U256>, // V3 only
        borrowing_pool_tick: Option<i32>,        // V3 only
        borrowing_pool_tick_data: Option<Vec<UniswapV3TickData>>, // V3 only
        borrowing_pool_liquidity_net: Option<i128>, // V3 only
        repay_pool_fee: Option<u32>,             // V3 only
        repay_pool_liquidity: Option<u128>,      // V3 only
        repay_pool_sqrt_price: Option<U256>,     // V3 only
        repay_pool_tick: Option<i32>,            // V3 only
        repay_pool_tick_data: Option<Vec<UniswapV3TickData>>, // V3 only
        repay_pool_liquidity_net: Option<i128>,  // V3 only
    ) -> Self {
        #[allow(clippy::too_many_arguments)]
        Self {
            borrowing_pool_reserve_0,
            borrowing_pool_reserve_1,
            repay_pool_reserve_0,
            repay_pool_reserve_1,
            borrowing_pool_type,
            repay_pool_type,
            borrow_0_buy_1,
            borrowing_pool_fee,
            borrowing_pool_liquidity,
            borrowing_pool_sqrt_price,
            borrowing_pool_tick,
            borrowing_pool_tick_data,
            borrowing_pool_liquidity_net,
            repay_pool_fee,
            repay_pool_liquidity,
            repay_pool_sqrt_price,
            repay_pool_tick,
            repay_pool_tick_data,
            repay_pool_liquidity_net,
        }
    }

    /// Called by arb function to calculate the optimal trade size
    pub async fn calc_optimal_arb(
        provider: Arc<Provider<Ws>>,
        borrowing_pool: &Pool,
        repay_pool: &Pool,
        borrow_0_buy_1: bool,
    ) -> f64 {
        let mut cost: ArbPool;
        match borrowing_pool.pool_type {
            PoolType::UniswapV3(uni_v3_pool) => {
                let (
                    borrowing_pool_reserve_0,
                    borrowing_pool_reserve_1,
                    borrowing_pool_fee,
                    borrowing_pool_liquidity,
                    borrowing_pool_sqrt_price,
                    borrowing_pool_tick,
                    borrowing_pool_tick_data,
                    borrowing_pool_liquidity_net,
                ) = v3::swap::get_pool_data(uni_v3_pool, borrow_0_buy_1, provider.clone())
                    .await
                    .unwrap();

                cost = ArbPool::new(
                    borrowing_pool_reserve_0 as f64,
                    borrowing_pool_reserve_1 as f64,
                    0.0,
                    0.0,
                    borrowing_pool.pool_type,
                    repay_pool.pool_type,
                    borrow_0_buy_1,
                    Some(borrowing_pool_fee),
                    Some(borrowing_pool_liquidity),
                    Some(borrowing_pool_sqrt_price),
                    Some(borrowing_pool_tick),
                    Some(borrowing_pool_tick_data),
                    Some(borrowing_pool_liquidity_net),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
            }
            PoolType::UniswapV2(uni_v2_pool) => {
                let (borrowing_pool_reserve_0, borrowing_pool_reserve_1) =
                    v2::swap::get_pool_data(uni_v2_pool, provider.clone()).await;

                cost = ArbPool::new(
                    borrowing_pool_reserve_0 as f64,
                    borrowing_pool_reserve_1 as f64,
                    0.0,
                    0.0,
                    borrowing_pool.pool_type,
                    repay_pool.pool_type,
                    borrow_0_buy_1,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
            }
        };

        match repay_pool.pool_type {
            PoolType::UniswapV3(uni_v3_pool) => {
                let (
                    repay_pool_reserve_0,
                    repay_pool_reserve_1,
                    repay_pool_fee,
                    repay_pool_liquidity,
                    repay_pool_sqrt_price,
                    repay_pool_tick,
                    repay_pool_tick_data,
                    repay_pool_liquidity_net,
                ) = v3::swap::get_pool_data(uni_v3_pool, borrow_0_buy_1, provider.clone())
                    .await
                    .unwrap();

                cost.repay_pool_reserve_0 = repay_pool_reserve_0 as f64;
                cost.repay_pool_reserve_1 = repay_pool_reserve_1 as f64;
                cost.repay_pool_fee = Some(repay_pool_fee);
                cost.repay_pool_liquidity = Some(repay_pool_liquidity);
                cost.repay_pool_sqrt_price = Some(repay_pool_sqrt_price);
                cost.repay_pool_tick = Some(repay_pool_tick);
                cost.repay_pool_tick_data = Some(repay_pool_tick_data);
                cost.repay_pool_liquidity_net = Some(repay_pool_liquidity_net);
            }
            PoolType::UniswapV2(uni_v2_pool) => {
                let (repay_pool_reserve_0, repay_pool_reserve_1) =
                    v2::swap::get_pool_data(uni_v2_pool, provider.clone()).await;

                cost.repay_pool_reserve_0 = repay_pool_reserve_0 as f64;
                cost.repay_pool_reserve_1 = repay_pool_reserve_1 as f64;
            }
        };

        let mut _solver = BrentOpt::new(0.0, 0.0);

        match borrow_0_buy_1 {
            true => {
                _solver = BrentOpt::new(
                    0.01 * cost.borrowing_pool_reserve_0,
                    0.05 * cost.borrowing_pool_reserve_0,
                );
            }
            false => {
                _solver = BrentOpt::new(
                    0.01 * cost.borrowing_pool_reserve_1,
                    0.05 * cost.borrowing_pool_reserve_1,
                );
            }
        }

        let executor = Executor::new(cost, _solver);

        let res = executor
            .add_observer(SlogLogger::term(), ObserverMode::Always)
            .run()
            .unwrap();

        res.state().best_param.unwrap()
    }
}

impl CostFunction for ArbPool {
    type Param = f64;
    type Output = f64;

    fn cost(&self, p: &Self::Param) -> Result<Self::Output, Error> {
        // added minus to maximize profit
        Ok(-maximize_arb_profit(
            &p,
            &self.borrowing_pool_reserve_0,
            &self.borrowing_pool_reserve_1,
            &self.repay_pool_reserve_0,
            &self.repay_pool_reserve_1,
            &self.borrow_0_buy_1,
            &self.borrowing_pool_type,
            &self.repay_pool_type,
            &self.borrowing_pool_fee,
            &self.borrowing_pool_liquidity,
            &self.borrowing_pool_sqrt_price,
            &self.borrowing_pool_tick,
            &self.borrowing_pool_tick_data,
            &self.borrowing_pool_liquidity_net,
            &self.repay_pool_fee,
            &self.repay_pool_liquidity,
            &self.repay_pool_sqrt_price,
            &self.repay_pool_tick,
            &self.repay_pool_tick_data,
            &self.repay_pool_liquidity_net,
        ))
    }
}

fn maximize_arb_profit(
    borrow_amt: &f64,
    borrowing_pool_reserve_0: &f64,
    borrowing_pool_reserve_1: &f64,
    repay_pool_reserve_0: &f64,
    repay_pool_reserve_1: &f64,
    borrow_0_buy_1: &bool,
    borrowing_pool_type: &PoolType,
    repay_pool_type: &PoolType,
    borrowing_pool_fee: &Option<u32>,         // V3 only
    borrowing_pool_liquidity: &Option<u128>,  // V3 only
    borrowing_pool_sqrt_price: &Option<U256>, // V3 only
    borrowing_pool_tick: &Option<i32>,        // V3 only
    borrowing_pool_tick_data: &Option<Vec<UniswapV3TickData>>, // V3 only
    borrowing_pool_liquidity_net: &Option<i128>, // V3 only
    repay_pool_fee: &Option<u32>,             // V3 only
    repay_pool_liquidity: &Option<u128>,      // V3 only
    repay_pool_sqrt_price: &Option<U256>,     // V3 only
    repay_pool_tick: &Option<i32>,            // V3 only
    repay_pool_tick_data: &Option<Vec<UniswapV3TickData>>, // V3 only
    repay_pool_liquidity_net: &Option<i128>,  // V3 only
) -> f64 {
    let mut _debt: f64 = 0.0;
    let mut _repay: f64 = 0.0;

    match borrowing_pool_type {
        PoolType::UniswapV2(_) => match borrow_0_buy_1 {
            true => {
                _debt = v2::swap::get_tokens_out_from_tokens_in(
                    Some(*borrow_amt),
                    None,
                    borrowing_pool_reserve_0,
                    borrowing_pool_reserve_1,
                )
                .unwrap();
            }
            false => {
                _debt = v2::swap::get_tokens_out_from_tokens_in(
                    None,
                    Some(*borrow_amt),
                    borrowing_pool_reserve_0,
                    borrowing_pool_reserve_1,
                )
                .unwrap();
            }
        },
        PoolType::UniswapV3(_) => match borrow_0_buy_1 {
            true => {
                _debt = v3::swap::get_tokens_out_from_tokens_in(
                    Some(*borrow_amt),
                    None,
                    &borrowing_pool_tick.unwrap(),
                    &borrowing_pool_sqrt_price.unwrap(),
                    &borrowing_pool_liquidity.unwrap(),
                    borrowing_pool_liquidity_net.unwrap(),
                    borrowing_pool_tick_data.as_ref().unwrap(),
                    &borrowing_pool_fee.unwrap(),
                )
                .unwrap()
            }
            false => {
                _debt = v3::swap::get_tokens_out_from_tokens_in(
                    None,
                    Some(*borrow_amt),
                    &borrowing_pool_tick.unwrap(),
                    &borrowing_pool_sqrt_price.unwrap(),
                    &borrowing_pool_liquidity.unwrap(),
                    borrowing_pool_liquidity_net.unwrap(),
                    borrowing_pool_tick_data.as_ref().unwrap(),
                    &borrowing_pool_fee.unwrap(),
                )
                .unwrap()
            }
        },
    }

    match repay_pool_type {
        PoolType::UniswapV2(_) => match borrow_0_buy_1 {
            true => {
                _repay = v2::swap::get_tokens_in_from_tokens_out(
                    None,
                    Some(*borrow_amt),
                    repay_pool_reserve_0,
                    repay_pool_reserve_1,
                )
                .unwrap();
            }
            false => {
                _repay = v2::swap::get_tokens_in_from_tokens_out(
                    Some(*borrow_amt),
                    None,
                    repay_pool_reserve_0,
                    repay_pool_reserve_1,
                )
                .unwrap();
            }
        },
        PoolType::UniswapV3(_) => match borrow_0_buy_1 {
            true => {
                _repay = v3::swap::get_tokens_in_from_tokens_out(
                    None,
                    Some(*borrow_amt),
                    &repay_pool_tick.unwrap(),
                    &repay_pool_sqrt_price.unwrap(),
                    &repay_pool_liquidity.unwrap(),
                    repay_pool_liquidity_net.unwrap(),
                    repay_pool_tick_data.as_ref().unwrap(),
                    &repay_pool_fee.unwrap(),
                )
                .unwrap()
            }
            false => {
                _repay = v3::swap::get_tokens_in_from_tokens_out(
                    Some(*borrow_amt),
                    None,
                    &repay_pool_tick.unwrap(),
                    &repay_pool_sqrt_price.unwrap(),
                    &repay_pool_liquidity.unwrap(),
                    repay_pool_liquidity_net.unwrap(),
                    repay_pool_tick_data.as_ref().unwrap(),
                    &repay_pool_fee.unwrap(),
                )
                .unwrap()
            }
        },
    };

    return _repay - _debt;
}

pub fn u256_2_f64(u256: U256) -> f64 {
    u256.as_u128() as f64
}

pub fn f64_2_u256(f64: f64) -> U256 {
    U256::from(f64 as u128)
}

pub fn q64_2_f64(x: u128) -> f64 {
    let decimals = ((x & 0xFFFFFFFFFFFFFFFF_u128) >> 48) as u32;
    let integers = ((x >> 64) & 0xFFFF) as u32;

    ((integers << 16) + decimals) as f64 / 2_f64.powf(16.0)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cfmm::pool::{Pool, PoolType, PoolVariant};
    use crate::utils::constants::{
        UNISWAP_V2_WETH_USDC_LP, UNISWAP_V3_WETH_USDC_LP_0_01, USDC_ADDRESS, WETH_ADDRESS,
    };
    use ethers::{
        core::types::{Block, H160, U256},
        providers::{Middleware, Provider, Ws},
        signers::LocalWallet,
    };

    #[tokio::test]
    async fn test_calc_optimal_arb() {
        // create a LocalWallet instance from local node's available account's private key
        let wallet: LocalWallet =
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                .parse::<LocalWallet>()
                .unwrap();
        let provider = Arc::new(
            Provider::<Ws>::connect("http://localhost:8545")
                .await
                .unwrap(),
        );

        let v2_pool = Pool::new(
            provider.clone(),
            UNISWAP_V2_WETH_USDC_LP.parse::<H160>().unwrap(),
            WETH_ADDRESS.parse::<H160>().unwrap(),
            USDC_ADDRESS.parse::<H160>().unwrap(),
            U256::from(300),
            PoolVariant::UniswapV2,
        )
        .await
        .unwrap();

        let v3_pool = Pool::new(
            provider.clone(),
            UNISWAP_V3_WETH_USDC_LP_0_01.parse::<H160>().unwrap(),
            WETH_ADDRESS.parse::<H160>().unwrap(),
            USDC_ADDRESS.parse::<H160>().unwrap(),
            U256::from(10),
            PoolVariant::UniswapV3,
        )
        .await
        .unwrap();

        println!("v2_pool: {:?}", v2_pool);
        println!("v3_pool: {:?}", v3_pool);

        let amt = ArbPool::calc_optimal_arb(provider.clone(), &v2_pool, &v3_pool, true).await;

        println!("amt: {:?}", amt);
    }
}
