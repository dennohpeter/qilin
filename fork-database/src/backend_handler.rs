// ported from foundry's executor with some modifications
// https://github.com/foundry-rs/foundry/blob/master/evm/src/executor/fork/backend.rs
use super::{
    blockchain_db::BlockchainDb,
    errors::{DatabaseError, DatabaseResult},
    utils::{b256_to_h256, h160_to_b160, ru256_to_u256, u256_to_ru256},
};
use ethers::{
    core::abi::ethereum_types::BigEndianHash,
    providers::Middleware,
    types::{Address, Block, BlockId, Bytes, Transaction, H256, U256},
    utils::keccak256,
};
use futures::{
    channel::mpsc::Receiver,
    stream::Stream,
    task::{Context, Poll},
    Future, FutureExt,
};
use revm::primitives::{bytes, AccountInfo, Bytecode, KECCAK_EMPTY, U256 as rU256};
use std::{
    collections::{hash_map::Entry, HashMap, VecDeque},
    pin::Pin,
    sync::{mpsc::Sender as OneshotSender, Arc},
};
use tracing::{error, trace, warn};

type AccountFuture<Err> =
    Pin<Box<dyn Future<Output = (Result<(U256, U256, Bytes), Err>, Address)> + Send>>;
type StorageFuture<Err> = Pin<Box<dyn Future<Output = (Result<U256, Err>, Address, U256)> + Send>>;
type BlockHashFuture<Err> = Pin<Box<dyn Future<Output = (Result<H256, Err>, u64)> + Send>>;
type FullBlockFuture<Err> = Pin<
    Box<
        dyn Future<
                Output = (
                    FullBlockSender,
                    Result<Option<Block<Transaction>>, Err>,
                    BlockId,
                ),
            > + Send,
    >,
>;
type TransactionFuture<Err> = Pin<
    Box<dyn Future<Output = (TransactionSender, Result<Option<Transaction>, Err>, H256)> + Send>,
>;

type AccountInfoSender = OneshotSender<DatabaseResult<AccountInfo>>;
type StorageSender = OneshotSender<DatabaseResult<U256>>;
type BlockHashSender = OneshotSender<DatabaseResult<H256>>;
type FullBlockSender = OneshotSender<DatabaseResult<Block<Transaction>>>;
type TransactionSender = OneshotSender<DatabaseResult<Transaction>>;

/// Request variants that are executed by the provider
enum ProviderRequest<Err> {
    Account(AccountFuture<Err>),
    Storage(StorageFuture<Err>),
    BlockHash(BlockHashFuture<Err>),
    FullBlock(FullBlockFuture<Err>),
    Transaction(TransactionFuture<Err>),
}

/// The Request type the Backend listens for
#[derive(Debug)]
pub enum BackendRequest {
    /// Fetch the account info
    Basic(Address, AccountInfoSender),
    /// Fetch a storage slot
    Storage(Address, U256, StorageSender),
    /// Fetch a block hash
    BlockHash(u64, BlockHashSender),
    /// Fetch an entire block with transactions
    FullBlock(BlockId, FullBlockSender),
    /// Fetch a transaction
    Transaction(H256, TransactionSender),
    /// Sets the pinned block to fetch data from
    SetPinnedBlock(BlockId),
}

/// Handles an internal provider and listens for requests.
///
/// This handler will remain active as long as it is reachable (request channel still open) and
/// requests are in progress.
#[must_use = "BackendHandler does nothing unless polled."]
pub struct BackendHandler<M: Middleware> {
    provider: M,
    /// Stores all the data.
    db: BlockchainDb,
    /// Requests currently in progress
    pending_requests: Vec<ProviderRequest<M::Error>>,
    /// Listeners that wait for a `get_account` related response
    account_requests: HashMap<Address, Vec<AccountInfoSender>>,
    /// Listeners that wait for a `get_storage_at` response
    storage_requests: HashMap<(Address, U256), Vec<StorageSender>>,
    /// Listeners that wait for a `get_block` response
    block_requests: HashMap<u64, Vec<BlockHashSender>>,
    /// Incoming commands.
    incoming: Receiver<BackendRequest>,
    /// unprocessed queued requests
    queued_requests: VecDeque<BackendRequest>,
    /// The block to fetch data from.
    // This is an `Option` so that we can have less code churn in the functions below
    block_id: Option<BlockId>,
}

impl<M> BackendHandler<M>
where
    M: Middleware + Clone + 'static,
{
    pub fn new(
        provider: M,
        db: BlockchainDb,
        rx: Receiver<BackendRequest>,
        block_id: Option<BlockId>,
    ) -> Self {
        Self {
            provider,
            db,
            pending_requests: Default::default(),
            account_requests: Default::default(),
            storage_requests: Default::default(),
            block_requests: Default::default(),
            queued_requests: Default::default(),
            incoming: rx,
            block_id,
        }
    }

    /// handle the request in queue in the future.
    ///
    /// We always check:
    ///  1. if the requested value is already stored in the cache, then answer the sender
    ///  2. otherwise, fetch it via the provider but check if a request for that value is already in
    /// progress (e.g. another Sender just requested the same account)
    fn on_request(&mut self, req: BackendRequest) {
        match req {
            BackendRequest::Basic(addr, sender) => {
                trace!(target: "backendhandler", "received request basic address={:?}", addr);
                let acc = self.db.accounts().read().get(&h160_to_b160(addr)).cloned();
                if let Some(basic) = acc {
                    let _ = sender.send(Ok(basic));
                } else {
                    self.request_account(addr, sender);
                }
            }
            BackendRequest::BlockHash(number, sender) => {
                let hash = self
                    .db
                    .block_hashes()
                    .read()
                    .get(&rU256::from(number))
                    .cloned();
                if let Some(hash) = hash {
                    let _ = sender.send(Ok(hash.into()));
                } else {
                    self.request_hash(number, sender);
                }
            }
            BackendRequest::FullBlock(number, sender) => {
                self.request_full_block(number, sender);
            }
            BackendRequest::Transaction(tx, sender) => {
                self.request_transaction(tx, sender);
            }
            BackendRequest::Storage(addr, idx, sender) => {
                // account is already stored in the cache
                let value = self
                    .db
                    .storage()
                    .read()
                    .get(&h160_to_b160(addr))
                    .and_then(|acc| acc.get(&u256_to_ru256(idx)).copied());
                if let Some(value) = value {
                    let _ = sender.send(Ok(ru256_to_u256(value)));
                } else {
                    // account present but not storage -> fetch storage
                    self.request_account_storage(addr, idx, sender);
                }
            }
            BackendRequest::SetPinnedBlock(block_id) => {
                self.block_id = Some(block_id);
            }
        }
    }

    /// process a request for account's storage
    fn request_account_storage(&mut self, address: Address, idx: U256, listener: StorageSender) {
        match self.storage_requests.entry((address, idx)) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push(listener);
            }
            Entry::Vacant(entry) => {
                trace!(target: "backendhandler", "preparing storage request, address={:?}, idx={}", address, idx);
                entry.insert(vec![listener]);
                let provider = self.provider.clone();
                let block_id = self.block_id;
                let fut = Box::pin(async move {
                    // serialize & deserialize back to U256
                    let idx_req = H256::from_uint(&idx);
                    let storage = provider.get_storage_at(address, idx_req, block_id).await;
                    let storage = storage.map(|storage| storage.into_uint());
                    (storage, address, idx)
                });
                self.pending_requests.push(ProviderRequest::Storage(fut));
            }
        }
    }

    /// returns the future that fetches the account data
    fn get_account_req(&self, address: Address) -> ProviderRequest<M::Error> {
        trace!(target: "backendhandler", "preparing account request, address={:?}", address);
        let provider = self.provider.clone();
        let block_id = self.block_id;
        let fut = Box::pin(async move {
            let balance = provider.get_balance(address, block_id);
            let nonce = provider.get_transaction_count(address, block_id);
            let code = provider.get_code(address, block_id);
            let resp = tokio::try_join!(balance, nonce, code);
            (resp, address)
        });
        ProviderRequest::Account(fut)
    }

    /// process a request for an account
    fn request_account(&mut self, address: Address, listener: AccountInfoSender) {
        match self.account_requests.entry(address) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push(listener);
            }
            Entry::Vacant(entry) => {
                entry.insert(vec![listener]);
                self.pending_requests.push(self.get_account_req(address));
            }
        }
    }

    /// process a request for an entire block
    fn request_full_block(&mut self, number: BlockId, sender: FullBlockSender) {
        let provider = self.provider.clone();
        let fut = Box::pin(async move {
            let block = provider.get_block_with_txs(number).await;
            (sender, block, number)
        });

        self.pending_requests.push(ProviderRequest::FullBlock(fut));
    }

    /// process a request for a transactions
    fn request_transaction(&mut self, tx: H256, sender: TransactionSender) {
        let provider = self.provider.clone();
        let fut = Box::pin(async move {
            let block = provider.get_transaction(tx).await;
            (sender, block, tx)
        });

        self.pending_requests
            .push(ProviderRequest::Transaction(fut));
    }

    /// process a request for a block hash
    fn request_hash(&mut self, number: u64, listener: BlockHashSender) {
        match self.block_requests.entry(number) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push(listener);
            }
            Entry::Vacant(entry) => {
                trace!(target: "backendhandler", "preparing block hash request, number={}", number);
                entry.insert(vec![listener]);
                let provider = self.provider.clone();
                let fut = Box::pin(async move {
                    let block = provider.get_block(number).await;

                    let block_hash = match block {
                        Ok(Some(block)) => Ok(block
                            .hash
                            .expect("empty block hash on mined block, this should never happen")),
                        Ok(None) => {
                            warn!(target: "backendhandler", ?number, "block not found");
                            // if no block was returned then the block does not exist, in which case
                            // we return empty hash
                            Ok(b256_to_h256(KECCAK_EMPTY))
                        }
                        Err(err) => {
                            error!(target: "backendhandler", ?err, ?number, "failed to get block");
                            Err(err)
                        }
                    };
                    (block_hash, number)
                });
                self.pending_requests.push(ProviderRequest::BlockHash(fut));
            }
        }
    }
}

impl<M> Future for BackendHandler<M>
where
    M: Middleware + Clone + Unpin + 'static,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let pin = self.get_mut();
        loop {
            // Drain queued requests first.
            while let Some(req) = pin.queued_requests.pop_front() {
                pin.on_request(req)
            }

            // receive new requests to delegate to the underlying provider
            loop {
                match Pin::new(&mut pin.incoming).poll_next(cx) {
                    Poll::Ready(Some(req)) => {
                        pin.queued_requests.push_back(req);
                    }
                    Poll::Ready(None) => {
                        trace!(target: "backendhandler", "last sender dropped, ready to drop (&flush cache)");
                        return Poll::Ready(());
                    }
                    Poll::Pending => break,
                }
            }

            // poll all requests in progress
            for n in (0..pin.pending_requests.len()).rev() {
                let mut request = pin.pending_requests.swap_remove(n);
                match &mut request {
                    ProviderRequest::Account(fut) => {
                        if let Poll::Ready((resp, addr)) = fut.poll_unpin(cx) {
                            // get the response
                            let (balance, nonce, code) = match resp {
                                Ok(res) => res,
                                Err(err) => {
                                    let err = Arc::new(eyre::Error::new(err));
                                    if let Some(listeners) = pin.account_requests.remove(&addr) {
                                        listeners.into_iter().for_each(|l| {
                                            let _ = l.send(Err(DatabaseError::GetAccount(
                                                addr,
                                                Arc::clone(&err),
                                            )));
                                        })
                                    }
                                    continue;
                                }
                            };

                            // convert it to revm-style types
                            let (code, code_hash) = if !code.0.is_empty() {
                                (Some(code.0.clone()), keccak256(&code).into())
                            } else {
                                (Some(bytes::Bytes::default()), KECCAK_EMPTY)
                            };

                            // update the cache
                            let acc = AccountInfo {
                                nonce: nonce.as_u64(),
                                balance: balance.into(),
                                code: code.map(|bytes| Bytecode::new_raw(bytes).to_checked()),
                                code_hash,
                            };
                            pin.db.accounts().write().insert(addr.into(), acc.clone());

                            // notify all listeners
                            if let Some(listeners) = pin.account_requests.remove(&addr) {
                                listeners.into_iter().for_each(|l| {
                                    let _ = l.send(Ok(acc.clone()));
                                })
                            }
                            continue;
                        }
                    }
                    ProviderRequest::Storage(fut) => {
                        if let Poll::Ready((resp, addr, idx)) = fut.poll_unpin(cx) {
                            let value = match resp {
                                Ok(value) => value,
                                Err(err) => {
                                    // notify all listeners
                                    let err = Arc::new(eyre::Error::new(err));
                                    if let Some(listeners) =
                                        pin.storage_requests.remove(&(addr, idx))
                                    {
                                        listeners.into_iter().for_each(|l| {
                                            let _ = l.send(Err(DatabaseError::GetStorage(
                                                addr,
                                                idx,
                                                Arc::clone(&err),
                                            )));
                                        })
                                    }
                                    continue;
                                }
                            };

                            // update the cache
                            pin.db
                                .storage()
                                .write()
                                .entry(addr.into())
                                .or_default()
                                .insert(idx.into(), value.into());

                            // notify all listeners
                            if let Some(listeners) = pin.storage_requests.remove(&(addr, idx)) {
                                listeners.into_iter().for_each(|l| {
                                    let _ = l.send(Ok(value));
                                })
                            }
                            continue;
                        }
                    }
                    ProviderRequest::BlockHash(fut) => {
                        if let Poll::Ready((block_hash, number)) = fut.poll_unpin(cx) {
                            let value = match block_hash {
                                Ok(value) => value,
                                Err(err) => {
                                    let err = Arc::new(eyre::Error::new(err));
                                    // notify all listeners
                                    if let Some(listeners) = pin.block_requests.remove(&number) {
                                        listeners.into_iter().for_each(|l| {
                                            let _ = l.send(Err(DatabaseError::GetBlockHash(
                                                number,
                                                Arc::clone(&err),
                                            )));
                                        })
                                    }
                                    continue;
                                }
                            };

                            // update the cache
                            pin.db
                                .block_hashes()
                                .write()
                                .insert(rU256::from(number), value.into());

                            // notify all listeners
                            if let Some(listeners) = pin.block_requests.remove(&number) {
                                listeners.into_iter().for_each(|l| {
                                    let _ = l.send(Ok(value));
                                })
                            }
                            continue;
                        }
                    }
                    ProviderRequest::FullBlock(fut) => {
                        if let Poll::Ready((sender, resp, number)) = fut.poll_unpin(cx) {
                            let msg = match resp {
                                Ok(Some(block)) => Ok(block),
                                Ok(None) => Err(DatabaseError::BlockNotFound(number)),
                                Err(err) => {
                                    let err = Arc::new(eyre::Error::new(err));
                                    Err(DatabaseError::GetFullBlock(number, err))
                                }
                            };
                            let _ = sender.send(msg);
                            continue;
                        }
                    }
                    ProviderRequest::Transaction(fut) => {
                        if let Poll::Ready((sender, tx, tx_hash)) = fut.poll_unpin(cx) {
                            let msg = match tx {
                                Ok(Some(tx)) => Ok(tx),
                                Ok(None) => Err(DatabaseError::TransactionNotFound(tx_hash)),
                                Err(err) => {
                                    let err = Arc::new(eyre::Error::new(err));
                                    Err(DatabaseError::GetTransaction(tx_hash, err))
                                }
                            };
                            let _ = sender.send(msg);
                            continue;
                        }
                    }
                }
                // not ready, insert and poll again
                pin.pending_requests.push(request);
            }

            // If no new requests have been queued, break to
            // be polled again later.
            if pin.queued_requests.is_empty() {
                return Poll::Pending;
            }
        }
    }
}
