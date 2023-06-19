use ethers::{
    contract::{abigen, ContractFactory},
    core::utils::Anvil,
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    solc::Solc,
};
use eyre::Result;
use std::{path::Path, sync::Arc, time::Duration, str::FromStr};

// abigen!(
//     SandwichDeployer,
//     r#"[
//         function setUp(address) internal;
//     ]"#,
//     event_derives(serde::Deserialize, serde::Serialize)
// );


pub fn deploy_sandwich_contract_to_anvil() -> Result<()> {

	let source = Path::new(&env!("CARGO_MANIFEST_DIR")).join("src/sandwitch/contracts/src/SandwichDeployer.sol");
	println!("source: {:?}", source);
	let compiled = Solc::default().compile_source(source).expect("could not compile source");
	println!("compiled: {:?}", compiled);
	let (abi, bytecode, runtime_bytecode) = compiled.find("SimpleStorage").expect("could not find contract").into_parts_or_default();


	Ok(())

}


#[cfg(test)]
mod tests {
	use crate::sandwich::utils::contract_deployer::deploy_sandwich_contract_to_anvil;

	#[test]
	fn test_deploy_sandwich_contract() {
		deploy_sandwich_contract_to_anvil().unwrap();
	}

}