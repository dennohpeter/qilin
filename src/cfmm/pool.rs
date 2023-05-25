use cfmms::{dex, pool};
use ethers::prelude::*;
use std::sync::Arc;
use std::{
    hash::{Hash, Hasher},
    str::FromStr,
};

pub type PoolVariant = dex::DexVariant;
pub type PoolType = pool::Pool;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pool {
    pub address: Address,
    pub token_0: Address,
    pub token_1: Address,
    pub swap_fee: U256,
    pub pool_variant: PoolVariant,
    pub pool_type: PoolType,
}

impl Pool {
    // Creates a new pool instance
    pub async fn new(
        provider: Arc<Provider<Ws>>,
        address: Address,
        token_a: Address,
        token_b: Address,
        swap_fee: U256,
        pool_variant: PoolVariant,
    ) -> Option<Pool> {
        let (token_0, token_1) = if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };
        match pool_variant {
            PoolVariant::UniswapV2 => {
                // TODO: function to query pool info
                if let Ok(_pool_type) =
                    pool::UniswapV2Pool::new_from_address(address, provider).await
                {
                    println!("Getting Uni V2 Pool: {:?}", _pool_type);

                    Some(Pool {
                        address,
                        token_0,
                        token_1,
                        swap_fee,
                        pool_variant,
                        pool_type: PoolType::UniswapV2(_pool_type),
                    })
                } else {
                    None
                }
            }
            PoolVariant::UniswapV3 => {
                if let Ok(_pool_type) =
                    pool::UniswapV3Pool::new_from_address(address, provider).await
                {
                    println!("Getting Uni V3 Pool: {:?}", _pool_type);
                    Some(Pool {
                        address,
                        token_0,
                        token_1,
                        swap_fee,
                        pool_variant,
                        pool_type: PoolType::UniswapV3(_pool_type),
                    })
                } else {
                    None
                }
            }
        }
    }
}
