use cfmms::{
	sync,
	dex::{
		Dex
	},
};
use std::error::Error;
use ethers::providers::{Provider, Ws};
use std::sync::{Arc};


pub async fn sync_pools(
	dexes: Vec<Dex>,
	middleware: Arc<Provider<Ws>>,
) -> Result<(), Box<dyn Error>> {
	sync::sync_pairs(dexes, middleware, None).await?;
	Ok(())
}