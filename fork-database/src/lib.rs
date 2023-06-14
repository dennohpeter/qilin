pub mod backend_handler;
pub mod blockchain_db;
pub mod errors;
pub mod forked_db;
pub mod shared_backend;
pub mod snapshot;
pub mod utils;

use crate::blockchain_db::{BlockchainDb, BlockchainDbMeta};
use crate::forked_db::ForkedDatabase;
use crate::shared_backend::SharedBackend;
use dotenv::dotenv;
use ethers::providers::{Http, Middleware, Provider, Ws};
use foundry_config::Config;
use foundry_evm::executor::opts::EvmOpts;
use std::env;
use std::{collections::BTreeSet, sync::Arc};

pub async fn setup_fork_db() -> ForkedDatabase {
    dotenv().ok();
    let _blast_key = env::var("BLAST_API_KEY").unwrap();
    let mainnet_blast_http_url = format!("https://eth-mainnet.blastapi.io/{}", _blast_key);

    // EvmOpts only allows http provider
    let provider = Provider::<Http>::try_from(mainnet_blast_http_url.clone())
        .expect("could not connect to mainnet");

    let block_num = provider.get_block_number().await.unwrap();
    let config = Config::figment();
    let mut evm_opts = config.extract::<EvmOpts>().unwrap();
    evm_opts.fork_block_number = Some(block_num.as_u64().clone());

    let (env, _block) = evm_opts
        .fork_evm_env(mainnet_blast_http_url.clone())
        .await
        .unwrap();

    let _blast_key = env::var("BLAST_API_KEY").unwrap();
    let mainnet_blast_ws_url = format!("wss://eth-mainnet.blastapi.io/{}", _blast_key);

    let provider = Provider::<Ws>::connect(mainnet_blast_ws_url.clone())
        .await
        .ok()
        .unwrap();

    let meta = BlockchainDbMeta {
        cfg_env: env.cfg,
        block_env: env.block,
        hosts: BTreeSet::from([mainnet_blast_ws_url.clone().to_string()]),
    };

    let db = BlockchainDb::new(meta, None);
    let backend = SharedBackend::spawn_backend(Arc::new(provider), db.clone(), None).await;

    let forked_db = ForkedDatabase::new(backend.clone(), db.clone());

    forked_db
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain_db::{BlockchainDb, BlockchainDbMeta, JsonBlockCacheDB};
    use crate::forked_db::ForkedDatabase;
    use crate::shared_backend::SharedBackend;
    use revm::db::{DatabaseCommit, DatabaseRef};
    use revm::primitives::{Account, B160, U256 as rU256};

    use dotenv::dotenv;
    use ethers::providers::{Http, Middleware, Provider, Ws};
    use ethers::types::U64;
    use foundry_config::Config;
    use foundry_evm::executor::opts::EvmOpts;
    use hashbrown::HashMap as Map;
    use parking_lot::RwLock;
    use std::env;
    use std::{collections::BTreeSet, path::PathBuf, sync::Arc};

    async fn setup() -> (Provider<Ws>, String) {
        dotenv().ok();

        let _blast_key = env::var("BLAST_API_KEY").unwrap();
        let mainnet_blast_url = format!("wss://eth-mainnet.blastapi.io/{}", _blast_key);

        let provider = Provider::<Ws>::connect(mainnet_blast_url.clone())
            .await
            .ok()
            .unwrap();

        (provider, mainnet_blast_url)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_blockchaindb_shared_backend_syncing() {
        let (provider, mainnet_blast_url) = setup().await;

        let meta = BlockchainDbMeta {
            cfg_env: Default::default(),
            block_env: Default::default(),
            hosts: BTreeSet::from([mainnet_blast_url.clone().to_string()]),
        };

        let db = BlockchainDb::new(meta, None);
        let backend = SharedBackend::spawn_backend(Arc::new(provider), db.clone(), None).await;

        // vitalik's address
        let address: B160 = "0xd8da6bf26964af9d7eed9e03e53415d37aa96045 "
            .parse()
            .unwrap();

        let idx = rU256::from(0u64);
        let value = backend.storage(address, idx).unwrap();
        let account = backend.basic(address).unwrap().unwrap();

        // test accounts
        let mem_acc = db.accounts().read().get(&address).unwrap().clone();
        assert_eq!(account.balance, mem_acc.balance);
        assert_eq!(account.nonce, mem_acc.nonce);

        // test storage
        let slots = db.storage().read().get(&address).unwrap().clone();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots.get(&idx).copied().unwrap(), value);

        // test hash
        let num = rU256::from(10u64);
        let hash = backend.block_hash(num).unwrap();
        let mem_hash = *db.block_hashes().read().get(&num).unwrap();
        assert_eq!(hash, mem_hash);

        let handle = std::thread::spawn(move || {
            for i in 1..10 {
                let idx = rU256::from(i);
                let _ = backend.storage(address, idx);
            }
        });
        handle.join().unwrap();
        let slots = db.storage().read().get(&address).unwrap().clone();
        assert_eq!(slots.len() as u64, 10);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_flush_and_read_cache() {
        let (provider, mainnet_blast_url) = setup().await;

        let cache_path = PathBuf::from("src/cache_data/storage.json");

        let meta = BlockchainDbMeta {
            cfg_env: Default::default(),
            block_env: Default::default(),
            hosts: BTreeSet::from([mainnet_blast_url.clone().to_string()]),
        };

        let db = BlockchainDb::new(meta, Some(cache_path.clone()));
        let backend = SharedBackend::spawn_backend(Arc::new(provider), db.clone(), None).await;

        let address: B160 = "0xd8da6bf26964af9d7eed9e03e53415d37aa96045 "
            .parse()
            .unwrap();

        let idx = rU256::from(0u64);
        let _ = backend.storage(address, idx).unwrap();
        let _ = backend.basic(address).unwrap().unwrap();
        let _ = db.accounts().read().get(&address).unwrap().clone();

        // write to cache
        let _ = db.cache().flush();

        // read from cache
        let json = JsonBlockCacheDB::load(cache_path).unwrap();
        assert!(!json.db().accounts.read().is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_forkdb() {
        dotenv().ok();
        let _blast_key = env::var("BLAST_API_KEY").unwrap();
        let mainnet_blast_url = format!("https://eth-mainnet.blastapi.io/{}", _blast_key);

        // EvmOpts only allows http provider
        let provider = Provider::<Http>::try_from(mainnet_blast_url.clone())
            .expect("could not connect to mainnet");

        let block_num = provider.get_block_number().await.unwrap();
        let config = Config::figment();
        let mut evm_opts = config.extract::<EvmOpts>().unwrap();
        evm_opts.fork_block_number = Some(block_num.as_u64().clone());

        let (env, _block) = evm_opts
            .fork_evm_env(mainnet_blast_url.clone())
            .await
            .unwrap();

        assert_eq!(evm_opts.get_chain_id(), 1 as u64);

        let (provider, mainnet_ws_url) = setup().await;

        let meta = BlockchainDbMeta {
            cfg_env: env.cfg,
            block_env: env.block,
            hosts: BTreeSet::from([mainnet_ws_url.clone().to_string()]),
        };

        let db = BlockchainDb::new(meta, None);
        let backend = SharedBackend::spawn_backend(Arc::new(provider), db.clone(), None).await;

        let mut forked_db = ForkedDatabase::new(backend.clone(), db.clone());

        let db_data = forked_db.inner().accounts().read().is_empty();
        assert!(db_data);

        let address: B160 = "0xd8da6bf26964af9d7eed9e03e53415d37aa96045"
            .parse()
            .unwrap();

        let snapshot = forked_db.create_snapshot();
        let idx = rU256::from(0u64);
        let account = forked_db.basic(address).unwrap().unwrap();
        let mem_acc = db.accounts().read().get(&address).unwrap().clone();
        let snap_shot_acc = snapshot.get_storage(address.clone(), idx.clone());

        // test snapshot
        assert!(!snap_shot_acc.is_some());
        // test fork db and blockchain db sync
        assert_eq!(account.balance, mem_acc.balance);
        assert_eq!(account.nonce, mem_acc.nonce);

        // create a random account for testing writing to cache
        let mut account_delta: Map<B160, Account> = Map::new();
        let rand_account = B160::random();
        account_delta.insert(rand_account.clone(), Account::from(account.clone()));
        DatabaseCommit::commit(&mut forked_db.clone(), account_delta);

        let inner_account = forked_db.inner().accounts().read();
        let account_detail = inner_account.get(&rand_account);
        // test writing to cache
        // the db should have the randomly created account data
        assert!(!account_detail.is_some());
        drop(inner_account);

        forked_db
            .reset(Some(mainnet_blast_url.clone()), block_num - U64::from(1))
            .unwrap();
        let cleared_account = forked_db.inner().accounts();
        // test reset
        assert_eq!(cleared_account.read().is_empty(), true);
    }
}
