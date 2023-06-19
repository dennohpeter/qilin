pub mod abi;
pub mod state;
pub mod utils;

use async_trait::async_trait;
use std::sync::Arc;

use crate::types::{Action, Event};
use artemis::types::Strategy;
use dashmap::DashMap;
use parking_lot::RwLock;
use qilin_cfmms::pool::Pool;

use crate::sandwich::state::BotState;

use ethers::{
    providers::{JsonRpcClient, Middleware, Provider, PubsubClient, Ws},
    types::{Address, Block, BlockId, Transaction, H160, H256, U64},
};
use eyre::Result;
use fork_database::forked_db::ForkedDatabase;

type AllPools = Arc<RwLock<DashMap<Address, Pool>>>;

/// sandwich strategy directly ported from RustySando repo
/// https://github.com/mouseless-eth/rusty-sando
#[derive(Clone, Debug)]
pub struct RustySandoStrategy<M> {
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
        fork_db: Arc<ForkedDatabase>,
	test: bool,
	sandwich_address: Option<Address>
    ) -> Result<Self> {
        let sandwich_state = Arc::new(BotState::new(
		init_block, 
		&provider,
		test,
		sandwich_address
	).await?);

        Ok(Self {
            provider,
            inception_block: init_block,
            sandwich_state,
            all_pools,
            fork_db,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, Event};
    use ethers::{
        core::utils::{Anvil, AnvilInstance},
        providers::{Middleware, Provider, Ws},
        types::{Address, Block, BlockId, Transaction, H160, U256, U64},
    };
    use eyre::Result;
    use fork_database;
    use log;
    use parking_lot::RwLock;
    use qilin_cfmms::pool::{Pool, PoolVariant};
    use serde_json;
    use std::collections::BTreeMap;
    use std::fs;

    /// Setup test environment
    async fn setup() -> Result<(RustySandoStrategy<Provider<Ws>>, AnvilInstance)> {

        // setup anvil instance for testing
        // note: spawn() will panic if spawn is called without anvil being available in the user’s $PATH
        let anvil = Anvil::new()
		.fork("https://eth.llamarpc.com")
		.fork_block_number(17508706 as u64)
		.spawn();
        let url = anvil.ws_endpoint().to_string();
        let provider = Arc::new(Provider::<Ws>::connect(url).await.ok().ok_or(
		 eyre::eyre!("Error connecting to anvil instance")
	)?);

        // get block number
        let block_num = provider.get_block_number().await.unwrap();

        // load test all_pools.json
        let pool_json_data =
            match fs::read_to_string("./src/sandwich/test_data/all_pools.json") {
                Ok(data) => data,
                Err(e) => return Err(eyre::eyre!("Error reading all_pools.json: {}", e)),
            };

        let pool_btree_map: BTreeMap<Address, Pool> = serde_json::from_str(&pool_json_data)?;

        let all_pools: DashMap<Address, Pool> = DashMap::new();
        for (addr, _pool) in pool_btree_map {
            let pool = pool_initializer(&_pool, provider.clone()).await.unwrap();

            all_pools.insert(addr, pool);
        }

        // setup fork database
        let fork_db = fork_database::setup_fork_db().await;

        // initialize rusty sando strategy
        let rusty = RustySandoStrategy::new(
            block_num,
            provider.clone(),
            Arc::new(RwLock::new(all_pools)),
            Arc::new(fork_db),
	    true,
	    None
        )
        .await?;

        Ok((rusty, anvil))
    }

    #[tokio::test]
    async fn test_rusty_sando_strategy() -> Result<()> {
	let (rusty, anvil) = setup().await?;

	Ok(())

    }

    /// initialize pool for test
    pub async fn pool_initializer(_pool: &Pool, provider: Arc<Provider<Ws>>) -> Option<Pool> {
        match _pool.pool_variant {
            PoolVariant::UniswapV2 => {
                let address = _pool.address;
                let token_0 = _pool.token_0;
                let token_1 = _pool.token_1;

                let _pool = Pool::new(
                    provider.clone(),
                    address,
                    token_0,
                    token_1,
                    U256::from(3000),
                    PoolVariant::UniswapV2,
                )
                .await;
                _pool
            }
            PoolVariant::UniswapV3 => {
                let address = _pool.address;
                let token_0 = _pool.token_0;
                let token_1 = _pool.token_1;
                let fee = _pool.swap_fee;

                let _pool = Pool::new(
                    provider.clone(),
                    address,
                    token_0,
                    token_1,
                    fee,
                    PoolVariant::UniswapV3,
                )
                .await;

                _pool
            }
        }
    }
}
