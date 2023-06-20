use ethers::{
    contract::{abigen, ContractFactory},
    core::utils::Anvil,
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    solc::{Artifact, Project, ProjectPathsConfig},
    types::U256,
    utils::parse_units,
};
use eyre::Result;
use std::{path::PathBuf, str::FromStr, sync::Arc, time::Duration};

abigen!(
    SandwichDeployer,
    "./src/sandwich/contracts/out/SandwichDeployer.sol/SandwichDeployer.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

pub async fn deploy_contract_to_anvil() -> Result<()> {
    let anvil = Anvil::new()
	.chain_id(1 as u64)
        .fork("https://eth.llamarpc.com")
        .fork_block_number(17508706 as u64)
        .spawn();

    let wallet: LocalWallet = anvil.keys()[0].clone().into();
    let url = anvil.endpoint().to_string();
    let provider =
        Arc::new(Provider::<Http>::try_from(url)?.interval(Duration::from_millis(10u64)));

    let client = SignerMiddleware::new(provider, wallet);
    let client = Arc::new(client);

    let contract = match SandwichDeployer::deploy(client.clone(), ())?
        .send()
        .await
    {
        Ok(contract) => contract,
        Err(e) => return Err(eyre::eyre!("Error deploying contract: {}", e)),
    };

    let contract = SandwichDeployer::new(contract.address(), client.clone());

    let value = U256::from(
	parse_units("1000.0", "ether").unwrap()
    );

//     let addr = contract.run().value(value).send().await?.await?;
    let addr = contract.run().send().await?.await?;



    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::sandwich::utils::contract_deployer::deploy_contract_to_anvil;

    #[tokio::test]
    async fn test_deploy_contract() {
        deploy_contract_to_anvil().await.unwrap();
    }
}
