pub mod state;
pub mod abi;
pub mod utils;

use std::sync::Arc;
use async_trait::async_trait;

use artemis::types::{
	Strategy,
};
use crate::types::{
	Event,
	Action,
};
use dashmap::DashMap;
use qilin_cfmms::pool::Pool;
use parking_lot::RwLock;

use crate::sandwich::state::BotState;

use ethers::{
	providers::{Middleware, Provider, Ws, PubsubClient, JsonRpcClient},
	types::{Address, Block, BlockId, Transaction, H160, H256, U64},
};
use fork_database::forked_db::ForkedDatabase;
use eyre::Result;

type AllPools = Arc<RwLock<DashMap<Address, Pool>>>;

/// sandwich strategy directly ported from RustySando repo
/// https://github.com/mouseless-eth/rusty-sando
#[derive(Clone, Debug)]
pub struct RustySandoStrategy<M>
{
	provider: Arc<M>,
	inception_block: U64,
	sandwich_state: Arc<BotState>,
	all_pools: AllPools,
	fork_db: Arc<ForkedDatabase>,
	// TODO: add bundle sender
}

// #[async_trait]
// impl<M: Middleware + 'static> Strategy<Event, Action> for RustySandoStrategy<M> {

// 	async fn sync_state(&mut self) -> Result<()> {
// 		// state synced in the initial setup
// 		Ok(())
// 	}

// 	async fn process_event(&mut self, event: Event) -> Result<Vec<Action>> {
// 		todo!()
// 	}
// }


impl<M> RustySandoStrategy<M> 
where 
	M: Middleware + 'static, 
	M::Provider: PubsubClient,
	M::Provider: JsonRpcClient,
{
	pub async fn new(
		init_block: U64, 
		provider: Arc<M>, 
		all_pools: AllPools, 
		fork_db: Arc<ForkedDatabase>
	) -> Result<Self> {

		let sandwich_state = Arc::new(BotState::new(init_block, &provider).await?);

		Ok(
			Self {
				provider,
				inception_block: init_block,
				sandwich_state,
				all_pools,
				fork_db,
			}
		)
	}

}