use collectors::types::{
    BlockPayload,
    NewTx,
};

/// Core Event implementation for the strategies
#[derive(Debug, Clone)]
pub enum Event {
    NewBlock(BlockPayload),
    NewMempoolTx(NewTx),
}

/// Core Action implementation for the strategies
#[derive(Debug, Clone)]
pub enum Action {
    SubmitTx,
}
