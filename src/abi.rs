use serde::Serialize;
use ethers::prelude::{Address, U256};

#[derive(Serialize)]
pub struct UniswapQueryAbi {
    inputs: Vec<UniswapQueryAbiInputs>,
    name: String,
    outputs: Vec<UniswapQueryAbiOutputs>,
    state_mutability: String,
    abi_type: String,
}

#[derive(Serialize)]
pub struct UniswapQueryAbiInputs {
    internal_type: String,
    name: String,
    abi_type: String,
}

#[derive(Serialize)]
pub struct UniswapQueryAbiOutputs {
    internal_type: String,
    name: String,
    abi_type: String,
}

#[derive(Serialize)]
pub struct BundleExecutorAbi {
    inputs: Vec<BundleExecutorAbiInputs>,
    name: Option<String>,
    outputs: Option<Vec<BundleExecutorAbiOutputs>>,
    state_mutability: String,
    abi_type: String,
}

#[derive(Serialize)]
pub struct BundleExecutorAbiInputs {
    internal_type: String,
    name: String,
    abi_type: String,
}

#[derive(Serialize)]
pub struct BundleExecutorAbiOutputs {
    internal_type: String,
    name: String,
    abi_type: String,
}

#[derive(Serialize)]
pub struct UniswapPairAbi {
    inputs: Option<Vec<UniswapPairAbiInputs>>,
    payable: Option<bool>,
    state_mutability: String,
    abi_type: String,
    anonymous: Option<bool>,
    name: Option<String>,
    indexed: Option<bool>,
    constant: Option<bool>,
    outputs: Option<Vec<UniswapPairAbiOutputs>>,
}

#[derive(Serialize)]
pub struct UniswapPairAbiInputs {
    indexed: Option<bool>,
    internal_type: String,
    name: String,
    abi_type: String,
}

#[derive(Serialize)]
pub struct UniswapPairAbiOutputs {
    internal_type: String,
    name: String,
    abi_type: String,
}
