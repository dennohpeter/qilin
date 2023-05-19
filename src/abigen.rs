use crate::utils::constants::{
    DAI_ADDRESS, UNISWAP_UNIVERSAL_ROUTER, UNISWAP_V2_FACTORY, UNISWAP_V2_ROUTER_1,
    UNISWAP_V2_ROUTER_2, UNISWAP_V3_FACTORY, UNISWAP_V3_QUOTER, UNISWAP_V3_QUOTER_V2,
    UNISWAP_V3_ROUTER_1, UNISWAP_V3_ROUTER_2, UNISWAP_V3_WETH_DAI_LP, USDC_ADDRESS, USDT_ADDRESS,
    WETH_ADDRESS,
};
use ethers::core::types::Chain;
use ethers::etherscan::Client;
use ethers::prelude::Abigen;
use ethers::types::H160;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;

async fn generate_abigen(
    client: &Client,
    contract_name: &str,
    contract_address: H160,
) -> Result<(), Box<dyn Error>> {
    let metadata = client.contract_source_code(contract_address).await?;
    let abi = metadata.items[0].abi.as_str();
    println!("ABI: {:?}", abi);

    // let abi_json = serde_json::from_str::<Value>(abi)?;
    let mut file = File::create(format!("abi/{}.json", contract_name.to_lowercase()))?;
    file.write_all(abi.as_bytes())?;
    println!("writing to file: {:?}", contract_name.to_lowercase());

    let abi_source = format!("./abi/{}.json", contract_name.to_lowercase());
    Abigen::new(contract_name.to_lowercase(), abi_source)?
        .generate()?
        .write_to_file(format!("src/bindings/{}.rs", contract_name.to_lowercase()))?;

    Ok(())
}

pub async fn generate_abigen_for_addresses() -> Result<(), Box<dyn Error>> {
    let _etherscan_key = env::var("ETHERSCAN_API_KEY").clone().unwrap();
    let etherscan_client = Client::new(Chain::Mainnet, _etherscan_key).unwrap();

    let mut address_book = HashMap::new();

    // address_book.insert("DAI", DAI_ADDRESS);
    // address_book.insert("USDC", USDC_ADDRESS);
    // address_book.insert("USDT", USDT_ADDRESS);
    // address_book.insert("WETH", WETH_ADDRESS);
    // address_book.insert("UNISWAP_V2_ROUTER_1", UNISWAP_V2_ROUTER_1);
    // address_book.insert("UNISWAP_V2_ROUTER_2", UNISWAP_V2_ROUTER_2);
    // address_book.insert("UNISWAP_V3_ROUTER_1", UNISWAP_V3_ROUTER_1);
    // address_book.insert("UNISWAP_V3_ROUTER_2", UNISWAP_V3_ROUTER_2);
    // address_book.insert("UNISWAP_UNIVERSAL_ROUTER", UNISWAP_UNIVERSAL_ROUTER);
    // address_book.insert("UNISWAP_V3_WETH_DAI_LP", UNISWAP_V3_WETH_DAI_LP);
    // address_book.insert("UNISWAP_V3_QUOTER", UNISWAP_V3_QUOTER);
    // address_book.insert("UNISWAP_V3_QUOTER_V2", UNISWAP_V3_QUOTER_V2);
    // address_book.insert("UNISWAP_V2_FACTORY", UNISWAP_V2_FACTORY);
    address_book.insert("UNISWAP_V3_FACTORY", UNISWAP_V3_FACTORY);

    let mut parsed_addr;
    for (name, addr) in address_book {
        parsed_addr = addr.parse::<H160>()?;
        generate_abigen(&etherscan_client, name, parsed_addr).await?;
    }

    Ok(())
}
