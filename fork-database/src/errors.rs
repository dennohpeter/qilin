// ported from foundry's executor
// // https://github.com/foundry-rs/foundry/blob/master/evm/src/executor/backend/error.rs
use ethers::types::{Address, BlockId, H256, U256};
use futures::channel::mpsc::{SendError, TrySendError};
use std::{
    convert::Infallible,
    sync::{mpsc::RecvError, Arc},
};

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Could should already be loaded: {0:?}")]
    MissingCode(H256),
    #[error("{0}")]
    Message(String),
    #[error("Failed to get account for {0:?}: {0:?}")]
    GetAccount(Address, Arc<eyre::Error>),
    #[error("Failed to get storage for {0:?} at {1:?}: {2:?}")]
    GetStorage(Address, U256, Arc<eyre::Error>),
    #[error("Failed to get block hash for {0}: {1:?}")]
    GetBlockHash(u64, Arc<eyre::Error>),
    #[error("Failed to get full block for {0:?}: {1:?}")]
    GetFullBlock(BlockId, Arc<eyre::Error>),
    #[error("Block {0:?} does not exist")]
    BlockNotFound(BlockId),
    #[error("Failed to get transaction {0:?}: {1:?}")]
    GetTransaction(H256, Arc<eyre::Error>),
    #[error("Transaction {0:?} not found")]
    TransactionNotFound(H256),
    #[error(transparent)]
    Recv(#[from] RecvError),
    #[error(transparent)]
    Send(#[from] SendError),
}

pub type DatabaseResult<T> = Result<T, DatabaseError>;

impl DatabaseError {
    /// Create a new error with a message
    pub fn msg(msg: impl Into<String>) -> Self {
        DatabaseError::Message(msg.into())
    }
}

impl<T> From<TrySendError<T>> for DatabaseError {
    fn from(err: TrySendError<T>) -> Self {
        err.into_send_error().into()
    }
}

impl From<Infallible> for DatabaseError {
    fn from(never: Infallible) -> Self {
        match never {}
    }
}
