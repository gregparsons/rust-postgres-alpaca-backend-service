//! error.rs
//!
//!
//!

#[derive(Debug, PartialEq)]
pub enum TradeWebError {
    ReqwestError,
    JsonError,
    SqlxError,
    ChannelError,
    Alpaca422,
    Alpaca429,
    Alpaca403,
    TransactionNotFound,
    BuyOrderExists,
    PositionExists,
    DeleteFailed,
    NoSharesFound,
    SqlInjectionRisk,
}

#[derive(Debug, Clone)]
pub enum PollerError {
    DifferentSizeSnapshots,
    SnapshotsNotOkayFromDatabase,
    Sqlx
}
