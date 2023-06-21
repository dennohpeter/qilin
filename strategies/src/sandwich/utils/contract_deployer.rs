use ethers::{
    contract::abigen,
    middleware::SignerMiddleware,
    providers::Middleware,
    signers::Signer,
    types::{H160, U256},
    utils::parse_units,
};
use eyre::Result;
use qilin_cfmms::bindings::weth::weth_contract;
use std::sync::Arc;

abigen!(
    Sandwicher,
    "./src/sandwich/contracts/out/Sandwicher.sol/Sandwicher.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

/// Deploy the Sandwicher.sol contract for test
pub(crate) async fn deploy_contract_to_anvil<M, S>(
    client: Arc<SignerMiddleware<Arc<M>, S>>,
) -> Result<Sandwicher<SignerMiddleware<Arc<M>, S>>>
where
    S: Signer + 'static,
    M: Middleware + 'static,
{
    let wallet = client.signer().clone();
    let contract = match Sandwicher::deploy(client.clone(), wallet.address())?
        .send()
        .await
    {
        Ok(contract) => contract,
        Err(e) => return Err(eyre::eyre!("Error deploying contract: {}", e)),
    };
    println!("Deployed contract ss: {}", contract.address());

    // load weth contract
    let weth_instance = weth_contract::weth::new(
        "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse::<H160>()?,
        client.clone(),
    );
    println!("Deployed contract weth: {}", weth_instance.address());

    let value = U256::from(parse_units("1000.0", "ether").unwrap());

    // deposit weth
    let _ = weth_instance
        .deposit()
        .value(value)
        .send()
        .await?
        .await?
        .expect("deposit failed");
    println!("Deposit weth success");

    // approve
    let _ = weth_instance
        .approve(contract.clone().address(), U256::MAX)
        .send()
        .await?
        .await?;

    let value = U256::from(parse_units("100.0", "ether").unwrap());

    // transfer weth to sandwich contract
    let _ = weth_instance
        .transfer(contract.clone().address(), value)
        .send()
        .await?
        .await?;

    println!("Deployed contract to address: {}", contract.address());

    let contract = Sandwicher::new(contract.address(), client.clone());

    Ok(contract)
}

#[cfg(test)]
mod tests {
    use crate::sandwich::utils::contract_deployer::deploy_contract_to_anvil;
    use ethers::{
        core::utils::Anvil,
        middleware::SignerMiddleware,
        providers::{Provider, Ws},
        signers::LocalWallet,
        types::{H160, U256},
        utils::parse_units,
    };
    use eyre::Result;
    use qilin_cfmms::bindings::weth::weth_contract;
    use std::sync::Arc;

    #[tokio::test]
    #[ignore]
    async fn test_deploy_contract() -> Result<()> {
        let anvil = Anvil::new()
            .chain_id(1 as u64)
            .fork("https://eth.llamarpc.com")
            .fork_block_number(17508706 as u64)
            .spawn();

        let url = anvil.ws_endpoint().to_string();
        let provider = Arc::new(Provider::<Ws>::connect(url).await.unwrap());

        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let client = Arc::new(SignerMiddleware::new(provider.clone(), wallet.clone()));

        let weth_instance = weth_contract::weth::new(
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse::<H160>()?,
            client.clone(),
        );

        let sandwich_contract = deploy_contract_to_anvil(client.clone()).await.unwrap();

        let balance = weth_instance
            .balance_of(sandwich_contract.clone().address())
            .call()
            .await?;

        let value = U256::from(parse_units("100.0", "ether").unwrap());

        assert_eq!(balance, value);

        Ok(())
    }
}
