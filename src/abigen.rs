use std::error::Error;
use ethers::{
    core::utils::Anvil,
    middleware::SignerMiddleware,
    prelude::{abigen, Abigen},
};

pub async fn generate_abigen() -> Result<(), Box<dyn Error>>{
    println!("Hello, world!");
    let abi_source = "./abi/uniswap_v3_router.json";
    Abigen::new("SwapRouter", abi_source)?.generate()?.write_to_file("swap_router.rs")?;
    // // abigen!(
    // //     SwapRouter,
    // //     "etherscan:0xe592427a0aece92de3edee1f18e0157c05861564"
    // //  );
    // let etherscan_client = Client::new(Chain::Mainnet, _etherscan_key).unwrap();
    // let metadata = etherscan_client
    //     .contract_source_code("0xE592427A0AEce92De3Edee1F18E0157C05861564".parse().unwrap())
    //     .await
    //     .unwrap();
    // println!("{:?}", metadata);
    Ok(())
}
