// ported from foundry's executor with some modifications
// https://github.com/foundry-rs/foundry/blob/master/evm/src/executor/fork/cache.rs
use super::snapshot::StateSnapshot;
use hashbrown::HashMap as Map;
use parking_lot::RwLock;
use revm::{
    primitives::{Account, AccountInfo, B160, B256, KECCAK_EMPTY, U256},
    DatabaseCommit,
};
use serde::{ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};
use std::{collections::BTreeSet, fs, io::BufWriter, path::PathBuf, sync::Arc};
use tracing::{trace, warn};

use url::Url;

pub type StorageInfo = Map<U256, U256>;

/// A shareable Block database
#[derive(Clone, Debug)]
pub struct BlockchainDb {
    /// Contains all the data
    db: Arc<MemDb>,
    /// metadata of the current config
    meta: Arc<RwLock<BlockchainDbMeta>>,
    /// the cache that can be flushed
    cache: Arc<JsonBlockCacheDB>,
}

impl BlockchainDb {
    /// Creates a new instance of the [BlockchainDb]
    ///
    /// if a `cache_path` is provided it attempts to load a previously stored [JsonBlockCacheData]
    /// and will try to use the cached entries it holds.
    ///
    /// This will return a new and empty [MemDb] if
    ///   - `cache_path` is `None`
    ///   - the file the `cache_path` points to, does not exist
    ///   - the file contains malformed data, or if it couldn't be read
    ///   - the provided `meta` differs from [BlockchainDbMeta] that's stored on disk
    pub fn new(meta: BlockchainDbMeta, cache_path: Option<PathBuf>) -> Self {
        Self::new_db(meta, cache_path, false)
    }

    /// Creates a new instance of the [BlockchainDb] and skips check when comparing meta
    /// This is useful for offline-start mode when we don't want to fetch metadata of `block`.
    ///
    /// if a `cache_path` is provided it attempts to load a previously stored [JsonBlockCacheData]
    /// and will try to use the cached entries it holds.
    ///
    /// This will return a new and empty [MemDb] if
    ///   - `cache_path` is `None`
    ///   - the file the `cache_path` points to, does not exist
    ///   - the file contains malformed data, or if it couldn't be read
    ///   - the provided `meta` differs from [BlockchainDbMeta] that's stored on disk
    pub fn new_skip_check(meta: BlockchainDbMeta, cache_path: Option<PathBuf>) -> Self {
        Self::new_db(meta, cache_path, true)
    }

    fn new_db(meta: BlockchainDbMeta, cache_path: Option<PathBuf>, skip_check: bool) -> Self {
        trace!(target : "forge::cache", cache=?cache_path, "initialising blockchain db");
        // read cache and check if metadata matches
        let cache = cache_path
            .as_ref()
            .and_then(|p| {
                JsonBlockCacheDB::load(p).ok().filter(|cache| {
                    if skip_check {
                        return true;
                    }
                    let mut existing = cache.meta().write();
                    existing.hosts.extend(meta.hosts.clone());
                    if meta != *existing {
                        warn!(target : "cache", "non-matching block metadata");
                        false
                    } else {
                        true
                    }
                })
            })
            .unwrap_or_else(|| JsonBlockCacheDB::new(Arc::new(RwLock::new(meta)), cache_path));

        Self {
            db: Arc::clone(cache.db()),
            meta: Arc::clone(cache.meta()),
            cache: Arc::new(cache),
        }
    }

    /// Returns the map that holds the account related info
    pub fn accounts(&self) -> &RwLock<Map<B160, AccountInfo>> {
        &self.db.accounts
    }

    /// Returns the map that holds the storage related info
    pub fn storage(&self) -> &RwLock<Map<B160, StorageInfo>> {
        &self.db.storage
    }

    /// Returns the map that holds all the block hashes
    pub fn block_hashes(&self) -> &RwLock<Map<U256, B256>> {
        &self.db.block_hashes
    }

    /// Returns the [revm::Env] related metadata
    pub fn meta(&self) -> &Arc<RwLock<BlockchainDbMeta>> {
        &self.meta
    }

    /// Returns the inner cache
    pub fn cache(&self) -> &Arc<JsonBlockCacheDB> {
        &self.cache
    }

    /// Returns the underlying storage
    pub fn db(&self) -> &Arc<MemDb> {
        &self.db
    }
}

/// relevant identifying markers in the context of [BlockchainDb]
#[derive(Debug, Clone, Eq, Serialize)]
pub struct BlockchainDbMeta {
    pub cfg_env: revm::primitives::CfgEnv,
    pub block_env: revm::primitives::BlockEnv,
    /// all the hosts used to connect to
    pub hosts: BTreeSet<String>,
}

impl BlockchainDbMeta {
    /// Creates a new instance
    pub fn new(env: revm::primitives::Env, url: String) -> Self {
        let host = Url::parse(&url)
            .ok()
            .and_then(|url| url.host().map(|host| host.to_string()))
            .unwrap_or(url);

        BlockchainDbMeta {
            cfg_env: env.cfg.clone(),
            block_env: env.block,
            hosts: BTreeSet::from([host]),
        }
    }
}

// ignore hosts to not invalidate the cache when different endpoints are used, as it's commonly the
// case for http vs ws endpoints
impl PartialEq for BlockchainDbMeta {
    fn eq(&self, other: &Self) -> bool {
        self.cfg_env == other.cfg_env && self.block_env == other.block_env
    }
}

impl<'de> Deserialize<'de> for BlockchainDbMeta {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        /// A backwards compatible representation of [revm::CfgEnv]
        ///
        /// This prevents deserialization errors of cache files caused by breaking changes to the
        /// default [revm::CfgEnv], for example enabling an optional feature.
        /// By hand rolling deserialize impl we can prevent cache file issues
        struct CfgEnvBackwardsCompat {
            inner: revm::primitives::CfgEnv,
        }

        impl<'de> Deserialize<'de> for CfgEnvBackwardsCompat {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let mut value = serde_json::Value::deserialize(deserializer)?;

                // we check for breaking changes here
                if let Some(obj) = value.as_object_mut() {
                    // additional field `disable_eip3607` enabled by the `optional_eip3607` feature
                    let key = "disable_eip3607";
                    if !obj.contains_key(key) {
                        obj.insert(key.to_string(), true.into());
                    }
                    // additional field `disable_block_gas_limit` enabled by the
                    // `optional_block_gas_limit` feature
                    let key = "disable_block_gas_limit";
                    if !obj.contains_key(key) {
                        // keep default value
                        obj.insert(key.to_string(), false.into());
                    }
                    let key = "disable_base_fee";
                    if !obj.contains_key(key) {
                        // keep default value
                        obj.insert(key.to_string(), false.into());
                    }
                }

                let cfg_env: revm::primitives::CfgEnv =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(Self { inner: cfg_env })
            }
        }

        // custom deserialize impl to not break existing cache files
        #[derive(Deserialize)]
        struct Meta {
            cfg_env: CfgEnvBackwardsCompat,
            block_env: revm::primitives::BlockEnv,
            /// all the hosts used to connect to
            #[serde(alias = "host")]
            hosts: Hosts,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Hosts {
            Multi(BTreeSet<String>),
            Single(String),
        }

        let Meta {
            cfg_env,
            block_env,
            hosts,
        } = Meta::deserialize(deserializer)?;
        Ok(Self {
            cfg_env: cfg_env.inner,
            block_env,
            hosts: match hosts {
                Hosts::Multi(hosts) => hosts,
                Hosts::Single(host) => BTreeSet::from([host]),
            },
        })
    }
}

/// In Memory cache containing all fetched accounts and storage slots
/// and their values from RPC
#[derive(Debug, Default)]
pub struct MemDb {
    /// Account related data
    pub accounts: RwLock<Map<B160, AccountInfo>>,
    /// Storage related data
    pub storage: RwLock<Map<B160, StorageInfo>>,
    /// All retrieved block hashes
    pub block_hashes: RwLock<Map<U256, B256>>,
    // TODO: add a block number hashmap
}

impl MemDb {
    /// Clears all data stored in this db
    pub fn clear(&self) {
        self.accounts.write().clear();
        self.storage.write().clear();
        self.block_hashes.write().clear();
    }

    // Inserts the account, replacing it if it exists already
    pub fn do_insert_account(&self, address: B160, account: AccountInfo) {
        self.accounts.write().insert(address, account);
    }

    /// The implementation of [DatabaseCommit::commit()]
    pub fn do_commit(&self, changes: Map<B160, Account>) {
        let mut storage = self.storage.write();
        let mut accounts = self.accounts.write();
        for (add, mut acc) in changes {
            if acc.is_empty() || acc.is_destroyed {
                accounts.remove(&add);
                storage.remove(&add);
            } else {
                // insert account
                if let Some(code_hash) = acc
                    .info
                    .code
                    .as_ref()
                    .filter(|code| !code.is_empty())
                    .map(|code| code.hash())
                {
                    acc.info.code_hash = code_hash;
                } else if acc.info.code_hash.is_zero() {
                    acc.info.code_hash = KECCAK_EMPTY;
                }
                accounts.insert(add, acc.info);

                let acc_storage = storage.entry(add).or_default();
                if acc.storage_cleared {
                    acc_storage.clear();
                }
                for (index, value) in acc.storage {
                    if value.present_value() == U256::from(0) {
                        acc_storage.remove(&index);
                    } else {
                        acc_storage.insert(index, value.present_value());
                    }
                }
                if acc_storage.is_empty() {
                    storage.remove(&add);
                }
            }
        }
    }
}

impl Clone for MemDb {
    fn clone(&self) -> Self {
        Self {
            storage: RwLock::new(self.storage.read().clone()),
            accounts: RwLock::new(self.accounts.read().clone()),
            block_hashes: RwLock::new(self.block_hashes.read().clone()),
        }
    }
}

impl DatabaseCommit for MemDb {
    fn commit(&mut self, changes: Map<B160, Account>) {
        self.do_commit(changes)
    }
}

/// A [BlockCacheDB] that stores the cached content in a json file
#[derive(Debug)]
pub struct JsonBlockCacheDB {
    /// Where this cache file is stored.
    ///
    /// If this is a [None] then caching is disabled
    cache_path: Option<PathBuf>,
    /// Object that's stored in a json file
    data: JsonBlockCacheData,
}

impl JsonBlockCacheDB {
    /// Creates a new instance.
    fn new(meta: Arc<RwLock<BlockchainDbMeta>>, cache_path: Option<PathBuf>) -> Self {
        Self {
            cache_path,
            data: JsonBlockCacheData {
                meta,
                data: Arc::new(Default::default()),
            },
        }
    }

    /// Loads the contents of the diskmap file and returns the read object
    ///
    /// # Errors
    /// This will fail if
    ///   - the `path` does not exist
    ///   - the format does not match [JsonBlockCacheData]
    pub fn load(path: impl Into<PathBuf>) -> eyre::Result<Self> {
        let path = path.into();
        trace!(target : "cache", ?path, "reading json cache");
        let file = fs::File::open(&path).map_err(|err| {
            warn!(?err, ?path, "Failed to read cache file");
            err
        })?;
        let file = std::io::BufReader::new(file);
        let data = serde_json::from_reader(file).map_err(|err| {
            warn!(target : "cache", ?err, ?path, "Failed to deserialize cache data");
            err
        })?;
        Ok(Self {
            cache_path: Some(path),
            data,
        })
    }

    /// Returns the [MemDb] it holds access to
    pub fn db(&self) -> &Arc<MemDb> {
        &self.data.data
    }

    /// Metadata stored alongside the data
    pub fn meta(&self) -> &Arc<RwLock<BlockchainDbMeta>> {
        &self.data.meta
    }

    /// Returns `true` if this is a transient cache and nothing will be flushed
    pub fn is_transient(&self) -> bool {
        self.cache_path.is_none()
    }

    /// Flushes the DB to disk if caching is enabled
    pub fn flush(&self) {
        // writes the data to a json file
        if let Some(ref path) = self.cache_path {
            trace!(target: "cache", "saving json cache path={:?}", path);
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::File::create(path)
                .map_err(|e| warn!(target: "cache", "Failed to open json cache for writing: {}", e))
                .and_then(|f| {
                    serde_json::to_writer(BufWriter::new(f), &self.data)
                        .map_err(|e| warn!(target: "cache" ,"Failed to write to json cache: {}", e))
                });
            trace!(target: "cache", "saved json cache path={:?}", path);
        }
    }
}

/// The Data the [JsonBlockCacheDB] can read and flush
///
/// This will be deserialized in a JSON object with the keys:
/// `["meta", "accounts", "storage", "block_hashes"]`
#[derive(Debug)]
pub struct JsonBlockCacheData {
    pub meta: Arc<RwLock<BlockchainDbMeta>>,
    pub data: Arc<MemDb>,
}

impl Serialize for JsonBlockCacheData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(4))?;

        let meta = self.meta.read();
        map.serialize_entry("meta", &*meta)?;
        drop(meta);

        let accounts = self.data.accounts.read();
        map.serialize_entry("accounts", &*accounts)?;
        drop(accounts);

        let storage = self.data.storage.read();
        map.serialize_entry("storage", &*storage)?;
        drop(storage);

        let block_hashes = self.data.block_hashes.read();
        map.serialize_entry("block_hashes", &*block_hashes)?;
        drop(block_hashes);

        map.end()
    }
}

impl<'de> Deserialize<'de> for JsonBlockCacheData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Data {
            meta: BlockchainDbMeta,
            #[serde(flatten)]
            data: StateSnapshot,
        }

        let Data {
            meta,
            data:
                StateSnapshot {
                    accounts,
                    storage,
                    block_hashes,
                },
        } = Data::deserialize(deserializer)?;

        Ok(JsonBlockCacheData {
            meta: Arc::new(RwLock::new(meta)),
            data: Arc::new(MemDb {
                accounts: RwLock::new(accounts),
                storage: RwLock::new(storage),
                block_hashes: RwLock::new(block_hashes),
            }),
        })
    }
}

/// A type that flushes a `JsonBlockCacheDB` on drop
///
/// This type intentionally does not implement `Clone` since it's intended that there's only once
/// instance that will flush the cache.
#[derive(Debug)]
pub struct FlushJsonBlockCacheDB(pub Arc<JsonBlockCacheDB>);

impl Drop for FlushJsonBlockCacheDB {
    fn drop(&mut self) {
        trace!(target: "fork::cache", "flushing cache");
        self.0.flush();
        trace!(target: "fork::cache", "flushed cache");
    }
}
