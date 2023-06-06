use std::fmt;

use ethers::prelude::{AbiError, ContractError};
use ethers::providers::{Middleware, ProviderError};
use ethers::types::{H160, U256};
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum CFMMError<M>
where
    M: Middleware,
{
    #[error("Middleware error")]
    MiddlewareError(<M as Middleware>::Error),
    #[error("Provider error")]
    ProviderError(#[from] ProviderError),
    #[error("Contract error")]
    ContractError(#[from] ContractError<M>),
    #[error("ABI Codec error")]
    ABICodecError(#[from] AbiError),
    #[error("Eth ABI error")]
    EthABIError(#[from] ethers::abi::Error),
    #[error("Join error")]
    JoinError(#[from] JoinError),
    #[error("Uniswap V3 math error")]
    UniswapV3MathError(#[from] UniswapV3MathError),
    #[error("Pair for token_a/token_b does not exist in provided dexes")]
    PairDoesNotExistInDexes(H160, H160),
    #[error("Could not initialize new pool from event log")]
    UnrecognizedPoolCreatedEventLog,
    #[error("Error when syncing pool")]
    SyncError(H160),
    #[error("Error when getting pool data")]
    PoolDataError,
    #[error("Arithmetic error")]
    ArithmeticError(#[from] ArithmeticError),
    #[error("No initialized ticks during v3 swap simulation")]
    NoInitializedTicks,
    #[error("No liquidity net found during v3 swap simulation")]
    NoLiquidityNet,
}

#[derive(Error, Debug)]
pub enum ArithmeticError {
    ShadowOverflow(U256),
    RoundingError,
    YIsZero,
    SqrtPriceOverflow,
}

impl std::fmt::Display for ArithmeticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

// TODO: make these errors better, some errors in univ3 libs are just require(condition) without a message.
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
    #[error("Overflow error")]
    Overflow,
    #[error("Tick data exhausted, need to query new tick data")]
    TickDataError,
    #[error("Step Computation Error")]
    StepComputationError,
    #[error("Swap simulation failed")]
    SwapSimulationError,
    #[error("Pool type is not recognized")]
    WrongPoolType,
    #[error("Middleware error when getting next_initialized_tick_within_one_word")]
    MiddlewareError(String),
}
