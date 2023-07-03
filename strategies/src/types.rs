use collectors::types::{BlockPayload, NewTx};

/// Core Event implementation for the strategies
#[derive(Debug, Clone)]
pub enum Event {
    NewBlock(BlockPayload),
    NewMempoolTx(NewTx),
}

impl From<BlockPayload> for Event {
    fn from(payload: BlockPayload) -> Self {
        Self::NewBlock(payload)
    }
}

impl From<NewTx> for Event {
    fn from(tx: NewTx) -> Self {
        Self::NewMempoolTx(tx)
    }
}

/// Core Action implementation for the strategies
#[derive(Debug, Clone)]
pub enum Action {
    SubmitTx,
}
