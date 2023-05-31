use ethers::abi::Abi;
use ethers::prelude::*;
use ethers::types::U256;
use ethers::{
    contract::Contract,
    providers::{Middleware, Provider},
    types::H160,
};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

pub async fn slot_finder(
    provider: Arc<Provider<Ws>>,
    token_address: H160,
    pool_address: H160,
) -> Option<U256> {
    let mut file = File::open("src/bin/erc20.json").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let abi: Abi = serde_json::from_str(&contents).unwrap();

    let token = Contract::new(token_address, abi.clone(), provider.clone());

    //     let token_decimals = token.method::<_, u128>("decimals", ()).unwrap().call().await.unwrap();
    //     let token_symnol = token.method::<_, String>("symbol", ()).unwrap().call().await.unwrap();
    //     println!("token_decimals: {:?}", token_decimals);
    //     println!("token_symnol: {:?}", token_symnol);

    let balance = match token
        .method::<_, U256>("balanceOf", pool_address.clone())
        .unwrap()
        .call()
        .await
    {
        Ok(b) => b,
        Err(e) => {
            println!("Error: {}", e);
            return None;
        }
    };

    let mut slot;
    // TODO: use threads
    for i in 0..=100 {
        slot = U256::from(i);
        let tx_hash = TxHash::from(ethers::utils::keccak256(abi::encode(&[
            abi::Token::Address(pool_address),
            abi::Token::Uint(slot.clone()),
        ])));

        let storage_value = provider
            .clone()
            .get_storage_at(token_address.clone(), tx_hash, None)
            .await
            .unwrap();
        let storage_value_u256 = U256::from_big_endian(&storage_value.as_bytes());

        if storage_value_u256 == balance.clone() {
            return Some(U256::from(i));
        }
    }

    // TODO: add vyper support

    None
}

#[cfg(test)]
mod test {

    use super::*;
    use ethers::types::U256;
    use ethers::{providers::Provider, types::H160};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_balance_of_slot_finder() {
        // let wallet: LocalWallet =
        //     "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        //         .parse::<LocalWallet>()
        //         .unwrap();
        let client = Provider::<Ws>::connect("http://localhost:8545")
            .await
            .unwrap();

        let provider = Arc::new(client.clone());

        let val = slot_finder(
            provider.clone(),
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                .parse::<H160>()
                .unwrap(),
            "0x06da0fd433C1A5d7a4faa01111c044910A184553"
                .parse::<H160>()
                .unwrap(),
        )
        .await;

        assert_eq!(val, Some(U256::from(3)));
    }
}
