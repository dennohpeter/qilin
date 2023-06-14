// ported from foundry's executor with some modifications
// https://github.com/foundry-rs/foundry/blob/master/evm/src/executor/fork/backend.rs
use super::{
    backend_handler::{BackendHandler, BackendRequest},
    blockchain_db::{BlockchainDb, FlushJsonBlockCacheDB},
    errors::{DatabaseError, DatabaseResult},
    utils::{b160_to_h160, b256_to_h256, h256_to_b256},
};
use ethers::{
    providers::Middleware,
    types::{Address, Block, BlockId, Transaction, H256, U256},
};
use futures::channel::mpsc::{channel, Sender};
use revm::{
    db::DatabaseRef,
    primitives::{AccountInfo, Bytecode, B160, B256, KECCAK_EMPTY, U256 as rU256},
};
use std::sync::{mpsc::channel as oneshot_channel, Arc};
use tracing::{error, trace};

#[derive(Debug, Clone)]
pub struct SharedBackend {
    /// channel used for sending commands related to database operations
    backend: Sender<BackendRequest>,
    /// Ensures that the underlying cache gets flushed once the last `SharedBackend` is dropped.
    ///
    /// There is only one instance of the type, so as soon as the last `SharedBackend` is deleted,
    /// `FlushJsonBlockCacheDB` is also deleted and the cache is flushed.
    #[allow(unused_variables)]
    cache: Arc<FlushJsonBlockCacheDB>,
}

impl SharedBackend {
    /// _Spawns_ a new `BackendHandler` on a `tokio::task` that listens for requests from any
    /// `SharedBackend`. Missing values get inserted in the `db`.
    ///
    /// The spawned `BackendHandler` finishes once the last `SharedBackend` connected to it is
    /// dropped.
    ///
    /// NOTE: this should be called with `Arc<Provider>`
    pub async fn spawn_backend<M>(provider: M, db: BlockchainDb, pin_block: Option<BlockId>) -> Self
    where
        M: Middleware + Unpin + 'static + Clone,
    {
        let (shared, handler) = Self::new(provider, db, pin_block);
        // spawn the provider handler to a task
        trace!(target: "backendhandler", "spawning Backendhandler task");
        tokio::spawn(handler);
        shared
    }

    /// Same as `Self::spawn_backend` but spawns the `BackendHandler` on a separate `std::thread` in
    /// its own `tokio::Runtime`
    pub fn spawn_backend_thread<M>(
        provider: M,
        db: BlockchainDb,
        pin_block: Option<BlockId>,
    ) -> Self
    where
        M: Middleware + Unpin + 'static + Clone,
    {
        let (shared, handler) = Self::new(provider, db, pin_block);

        // spawn a light-weight thread with a thread-local async runtime just for
        // sending and receiving data from the remote client
        let _ = std::thread::Builder::new()
            .name("fork-backend-thread".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to create fork-backend-thread tokio runtime");

                rt.block_on(handler);
            })
            .expect("failed to spawn backendhandler thread");
        trace!(target: "backendhandler", "spawned Backendhandler thread");

        shared
    }

    /// Returns a new `SharedBackend` and the `BackendHandler`
    pub fn new<M>(
        provider: M,
        db: BlockchainDb,
        pin_block: Option<BlockId>,
    ) -> (Self, BackendHandler<M>)
    where
        M: Middleware + Unpin + 'static + Clone,
    {
        let (backend, backend_rx) = channel(1);
        let cache = Arc::new(FlushJsonBlockCacheDB(Arc::clone(db.cache())));
        let handler = BackendHandler::new(provider, db, backend_rx, pin_block);
        (Self { backend, cache }, handler)
    }

    /// Updates the pinned block to fetch data from
    pub fn set_pinned_block(&self, block: impl Into<BlockId>) -> eyre::Result<()> {
        let req = BackendRequest::SetPinnedBlock(block.into());
        self.backend
            .clone()
            .try_send(req)
            .map_err(|e| eyre::eyre!("{:?}", e))
    }

    /// Returns the full block for the given block identifier
    pub fn get_full_block(&self, block: impl Into<BlockId>) -> DatabaseResult<Block<Transaction>> {
        tokio::task::block_in_place(|| {
            let (sender, rx) = oneshot_channel();
            let req = BackendRequest::FullBlock(block.into(), sender);
            self.backend.clone().try_send(req)?;
            rx.recv()?
        })
    }

    /// Returns the transaction for the hash
    pub fn get_transaction(&self, tx: H256) -> DatabaseResult<Transaction> {
        tokio::task::block_in_place(|| {
            let (sender, rx) = oneshot_channel();
            let req = BackendRequest::Transaction(tx, sender);
            self.backend.clone().try_send(req)?;
            rx.recv()?
        })
    }

    fn do_get_basic(&self, address: Address) -> DatabaseResult<Option<AccountInfo>> {
        tokio::task::block_in_place(|| {
            let (sender, rx) = oneshot_channel();
            let req = BackendRequest::Basic(address, sender);
            self.backend.clone().try_send(req)?;
            rx.recv()?.map(Some)
        })
    }

    fn do_get_storage(&self, address: Address, index: U256) -> DatabaseResult<U256> {
        tokio::task::block_in_place(|| {
            let (sender, rx) = oneshot_channel();
            let req = BackendRequest::Storage(address, index, sender);
            self.backend.clone().try_send(req)?;
            rx.recv()?
        })
    }

    fn do_get_block_hash(&self, number: u64) -> DatabaseResult<H256> {
        tokio::task::block_in_place(|| {
            let (sender, rx) = oneshot_channel();
            let req = BackendRequest::BlockHash(number, sender);
            self.backend.clone().try_send(req)?;
            rx.recv()?
        })
    }

    /// Flushes the DB to disk if caching is enabled
    #[allow(dead_code)]
    pub(crate) fn flush_cache(&self) {
        self.cache.0.flush();
    }
}

impl DatabaseRef for SharedBackend {
    type Error = DatabaseError;

    fn basic(&self, address: B160) -> Result<Option<AccountInfo>, Self::Error> {
        trace!( target: "sharedbackend", "request basic {:?}", address);
        self.do_get_basic(b160_to_h160(address)).map_err(|err| {
            error!(target: "sharedbackend",  ?err, ?address,  "Failed to send/recv `basic`");
            err
        })
    }

    fn code_by_hash(&self, hash: B256) -> Result<Bytecode, Self::Error> {
        Err(DatabaseError::MissingCode(b256_to_h256(hash)))
    }

    fn storage(&self, address: B160, index: rU256) -> Result<rU256, Self::Error> {
        trace!( target: "sharedbackend", "request storage {:?} at {:?}", address, index);
        match self.do_get_storage(b160_to_h160(address), index.into()).map_err(|err| {
            error!( target: "sharedbackend", ?err, ?address, ?index, "Failed to send/recv `storage`");
          err
        }) {
            Ok(val) => Ok(val.into()),
            Err(err) => Err(err),
        }
    }

    fn block_hash(&self, number: rU256) -> Result<B256, Self::Error> {
        if number > rU256::from(u64::MAX) {
            return Ok(KECCAK_EMPTY);
        }
        let number: U256 = number.into();
        let number = number.as_u64();
        trace!( target: "sharedbackend", "request block hash for number {:?}", number);
        match self.do_get_block_hash(number).map_err(|err| {
            error!(target: "sharedbackend",?err, ?number, "Failed to send/recv `block_hash`");
            err
        }) {
            Ok(val) => Ok(h256_to_b256(val)),
            Err(err) => Err(err),
        }
    }
}
