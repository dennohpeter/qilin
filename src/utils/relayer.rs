use crate::utils::relayer;
use ethers::core::types::{Bytes, Eip1559TransactionRequest, NameOrAddress, U256, U64};
use ethers::prelude::SignerMiddleware;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::signers::{LocalWallet, Signer};
use ethers::solc::resolver::print;
use ethers::types::transaction::{eip2718::TypedTransaction, eip2930::AccessList};
use ethers_flashbots::{BundleRequest, BundleTransaction, FlashbotsMiddleware, SimulatedBundle};
use std::error::Error;
use std::sync::Arc;

pub async fn simulate_bundle(
    _to: NameOrAddress,
    _data: Bytes,
    flashbot_client: &Arc<
        SignerMiddleware<FlashbotsMiddleware<Arc<Provider<Ws>>, LocalWallet>, LocalWallet>,
    >,
    ws_provider: &Provider<Ws>,
    wallet: LocalWallet,
) -> Result<SimulatedBundle, Box<dyn Error>> {
    let current_block = ws_provider.get_block_number().await?;

    let wallet_address = wallet.address();
    let _nonce = flashbot_client
        .get_transaction_count(wallet_address.clone(), None)
        .await?;
    println!("Nonce: {}", _nonce);

    let test_transaction_request = Eip1559TransactionRequest {
        to: Some(_to),
        from: Some(wallet_address),
        data: Some(_data),
        chain_id: Some(U64::from(5)),
        // chain_id: Some(U64::from(1)),
        max_priority_fee_per_gas: Some(U256::from(0)),
        max_fee_per_gas: Some(U256::MAX),
        gas: Some(U256::from(550000)),
        nonce: Some(_nonce),
        value: None,
        access_list: AccessList::default(),
    };

    let frontrun_tx_typed = TypedTransaction::Eip1559(test_transaction_request);

    let tx = {
        let mut inner: TypedTransaction = frontrun_tx_typed;
        flashbot_client
            .clone()
            .fill_transaction(&mut inner, None)
            .await?;
        inner
    };
    println!("Tx: {:?}", tx);

    let signature = flashbot_client.signer().sign_transaction(&tx).await?;
    let signed_frontrun_tx = tx.rlp_signed(&signature);
    let signed_transactions = vec![signed_frontrun_tx];
    println!("signed_transactions: {:?}", signed_transactions);

    let bundle = relayer::construct_bundle(signed_transactions, current_block).map_err(|e| {
        println!("Bundle Construction Error{:?}", e);
        e
    })?;

    let simulated_bundle = flashbot_client.inner().simulate_bundle(&bundle).await?;
    println!("simulated_bundle: {:?}", simulated_bundle);

    Ok(simulated_bundle)
}

/// Helper function to help catch the various ways errors can be thrown from simulation
/// This helper function is needed as simulation response has many ways where the
/// error can be thrown.... which is not documented
pub fn validate_simulation_response(sim: &SimulatedBundle) -> eyre::Result<()> {
    // Make sure no simulated bundle transactions have errors or reverts
    for tx in &sim.transactions {
        if let Some(e) = &tx.error {
            eyre::bail!("Error in bundled transaction: {:?}", e);
        }
        if let Some(r) = &tx.revert {
            eyre::bail!("Transaction reverts: {:?}", r);
        }
    }
    Ok(())
}

/// Construct a Bundle Request for FlashBots
pub fn construct_bundle<T: Into<BundleTransaction>>(
    signed_transactions: Vec<T>,
    block_number: U64,
) -> eyre::Result<BundleRequest> {
    // Create the ethers-flashbots bundle request
    let mut bundle_request = BundleRequest::new();

    // Sign the transactions and add to the bundle
    for tx in signed_transactions {
        let bundled: BundleTransaction = tx.into();
        bundle_request = bundle_request.push_transaction(bundled);
    }

    // Set other bundle parameters
    bundle_request = bundle_request
        .set_block(block_number + 1)
        .set_simulation_block(block_number)
        .set_simulation_timestamp(0);

    // Return the constructed bundle request
    Ok(bundle_request)
}
