//! error.rs
//!
//!
//!

#[derive(Debug)]
pub enum TradeWebError {
    ReqwestError,
    JsonError,
    SqlxError,
    ChannelError,
}
