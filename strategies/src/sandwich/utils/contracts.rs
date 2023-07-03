use std::sync::Arc;

use crate::sandwich::abi::{Erc20, UniswapV2Pair};
use ethers::{prelude::*};

/// Create erc20 contract that we can interact with
pub fn get_erc20_contract<M>(erc20_address: &Address, client: &Arc<M>) -> Erc20<M>
where
    M: Middleware + 'static,
{
    Erc20::new(*erc20_address, client.clone())
}

/// Create v2 pair contract that we can interact with
pub fn get_pair_v2_contract<M>(pair_address: &Address, client: &Arc<M>) -> UniswapV2Pair<M>
where
    M: Middleware + 'static,
{
    UniswapV2Pair::new(*pair_address, client.clone())
}
