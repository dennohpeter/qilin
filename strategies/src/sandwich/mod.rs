pub mod abi;
pub mod state;
pub mod utils;


use std::{sync::Arc};

use crate::sandwich::state::BotState;


use dashmap::DashMap;
use parking_lot::RwLock;
use qilin_cfmms::pool::Pool;

use ethers::{
    middleware::SignerMiddleware,
    providers::{JsonRpcClient, Middleware, PubsubClient},
    signers::Signer,
    types::{Address, U64},
};
use eyre::Result;
use fork_database::forked_db::ForkedDatabase;

type AllPools = Arc<RwLock<DashMap<Address, Pool>>>;

/// Sandwich strategy directly ported from RustySando repo
/// https://github.com/mouseless-eth/rusty-sando
#[derive(Clone, Debug)]
pub struct RustySandoStrategy<M, S> {
    pub provider: Arc<M>,
    pub wallet: Arc<SignerMiddleware<Arc<M>, S>>,
    pub inception_block: U64,
    pub sandwich_state: Arc<BotState>,
    pub all_pools: AllPools,
    pub fork_db: Arc<RwLock<ForkedDatabase>>,
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

impl<M, S> RustySandoStrategy<M, S>
where
    M: Middleware + 'static,
    M::Provider: PubsubClient,
    M::Provider: JsonRpcClient,
    S: Signer + 'static,
{
    pub async fn new(
        init_block: U64,
        provider: Arc<M>,
        wallet: Arc<SignerMiddleware<Arc<M>, S>>,
        all_pools: AllPools,
        fork_db: Arc<RwLock<ForkedDatabase>>,
        test: bool,
        sandwich_address: Option<Address>,
    ) -> Result<Self> {
        let sandwich_state =
            Arc::new(BotState::new(init_block, &provider, test, sandwich_address).await?);

        Ok(Self {
            provider,
            wallet,
            inception_block: init_block,
            sandwich_state,
            all_pools,
            fork_db,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::utils::state_diff::{extract_pools, get_from_txs, to_cache_db};
    use super::*;
    
    use dotenv::dotenv;
    use env_logger;
    use env_logger::Env;
    use ethers::{
        core::k256::ecdsa::SigningKey,
        prelude::LocalWallet,
        core::utils::{Anvil},
        providers::{Middleware, Provider, Ws},
        types::{Address, BlockNumber, H256, U256},
        signers::Wallet,
    };
    use eyre::Result;
    use fork_database;
    
    use log;
    use parking_lot::RwLock;
    use qilin_cfmms::pool::{Pool, PoolVariant};
    use serde_json;
    use std::collections::BTreeMap;
    use std::env;
    use std::fs;
    use std::str::FromStr;
    use utils::contract_deployer::deploy_contract_to_anvil;

    const INIT_BLOCK: u64 = 17444939 as u64;

    /// Setup anvil test environment
    async fn setup() -> Result<RustySandoStrategy<Provider<Ws>, Wallet<SigningKey>>> {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

        dotenv().ok();
        let mainnet_http_url = env::var("HTTP_RPC").unwrap_or_else(|e| {
            log::error!("Error: {}", e);
            return e.to_string();
        });

        // setup anvil instance for testing
        // note: spawn() will panic if spawn is called without anvil being available in the user’s $PATH
        let anvil = Anvil::new()
            .fork(mainnet_http_url.clone())
            .fork_block_number(INIT_BLOCK)
            .spawn();

        let url = anvil.ws_endpoint().to_string();
        let provider = Arc::new(
            Provider::<Ws>::connect(url)
                .await
                .ok()
                .ok_or(eyre::eyre!("Error connecting to anvil instance"))?,
        );

        // get block number
        let block_num = provider.get_block_number().await.unwrap();

        // load test all_pools.json
        let pool_json_data = match fs::read_to_string("./src/sandwich/test_data/all_pools.json") {
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
        let fork_db =
            fork_database::setup_fork_db(provider.clone(), mainnet_http_url.to_string()).await;

        // setup wallet and client for sandwich contract deployment
        let wallet: LocalWallet = anvil.keys()[0].clone().into();
        let client = Arc::new(SignerMiddleware::new(provider.clone(), wallet.clone()));

        // load the sandwich contract
        let contract = deploy_contract_to_anvil(client.clone()).await?;
        log::info!("Deployed contract to address: {}", contract.address());

        // initialize rusty sando strategy
        let rusty = RustySandoStrategy::new(
            block_num,
            provider.clone(),
            client.clone(),
            Arc::new(RwLock::new(all_pools)),
            Arc::new(RwLock::new(fork_db)),
            true,
            Some(contract.address()),
        )
        .await?;

        Ok(rusty)
    }

    #[tokio::test]
    async fn test_rusty_sando_strategy() -> Result<()> {
        let rusty = setup().await?;

        dotenv().ok();
        let mainnet_url = env::var("WSS_RPC").unwrap_or_else(|e| {
            log::error!("Error: {}", e);
            return e.to_string();
        });

        let temp_provider = Arc::new(Provider::<Ws>::connect(mainnet_url).await.unwrap());

        // https://etherscan.io/tx/0xa2360f3cbd253bd3e80c25220576277a6b7f8e39e39199e1a82ca09c42667645
        let s = "0xa2360f3cbd253bd3e80c25220576277a6b7f8e39e39199e1a82ca09c42667645";
        let simulated_mempool_tx = temp_provider
            .get_transaction(H256::from_str(s).unwrap())
            .await
            .unwrap()
            .unwrap();
        log::info!("Simulated Mempool Tx: {:?}", simulated_mempool_tx);

        let res = get_from_txs(
            &temp_provider,
            &vec![simulated_mempool_tx.clone()],
            BlockNumber::Number(INIT_BLOCK.into()),
        )
        .await
        .unwrap();

        let sandwitch_pools = extract_pools(&res, &rusty.all_pools.clone().read()).unwrap();

        assert_eq!(sandwitch_pools.len(), 1);
        // Uniswap V3 USDC 3 Pool Address: 0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640
        assert_eq!(
            sandwitch_pools[0].pool.address,
            Address::from_str("0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640").unwrap()
        );

        let temp_provider_clone = temp_provider.clone();
        let rusty_fork_db_clone = rusty.fork_db.clone();
        let _cache_db = to_cache_db(&res, None, &temp_provider_clone, &rusty_fork_db_clone)
            .await
            .unwrap();

        for pool in sandwitch_pools {
            let _pool = pool.pool;
            // let pool_clone = pool.clone();
            // TODO: add sandwich simulation
        }

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
