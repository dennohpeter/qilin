use ethers::types::{I256, U256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UniswapV3MathError {
    #[error("Denominator is 0")]
    DenominatorIsZero,
    #[error("Result is U256::MAX")]
    ResultIsU256MAX,
    #[error("Sqrt price is 0")]
    SqrtPriceIsZero,
    #[error("Sqrt price is less than or equal to quotient")]
    SqrtPriceIsLteQuotient,
    #[error("Can not get most significant bit or least significant bit on zero value")]
    ZeroValue,
    #[error("Liquidity is 0")]
    LiquidityIsZero,
    //TODO: Update this, shield your eyes for now
    #[error(
        "require((product = amount * sqrtPX96) / amount == sqrtPX96 && numerator1 > product);"
    )]
    ProductDivAmount,
    #[error("Denominator is less than or equal to prod_1")]
    DenominatorIsLteProdOne,
    #[error("Liquidity Sub")]
    LiquiditySub,
    #[error("Liquidity Add")]
    LiquidityAdd,
    #[error("The given tick must be less than, or equal to, the maximum tick")]
    T,
    #[error(
        "Second inequality must be < because the price can never reach the price at the max tick"
    )]
    R,
    #[error("Overflow when casting to U160")]
    SafeCastToU160Overflow,
    #[error("Middleware error when getting next_initialized_tick_within_one_word")]
    MiddlewareError(String),
}

pub fn v3_swap(
    zero_2_one: bool,
    amount_speficied: I256,
    sqrt_price_limit_x96: U256
) -> Result<(U256, U256), UniswapV3MathError> {

    // steps:
    //  1)  determine exactInput or not: if amount_speficied > 0 then yes, otherwise no
    //  2)  cache the starting liquidity and tick
    //  3)  construct the initial swapping state:
    //      - state = {
    //                  "amountSpecifiedRemaining": amountSpecified,
    //                  "amountCalculated": 0,
    //                  "sqrtPriceX96": self.sqrt_price_x96,
    //                  "tick": self.tick,
    //                  "liquidity": cache["liquidityStart"],
    //                 }
    //  4) start walking through the liquidity ranges. Stop if either 1) each the limit or 2) amount_speficied is exhausted
    //      - find the next available tick 
    //      - ensure don't overshoot the tick max/min value
    //      - compute values to swap to the target tick, price limit, or point where input/output amount is exhausted
    //      - shift to next tick if we reach the next price
    //      - if not exhausted, continure
}