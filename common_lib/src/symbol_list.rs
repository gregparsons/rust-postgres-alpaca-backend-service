//! symbol_list.rs
//!
//! get the active symbols from the database

use serde::{Deserialize, Serialize};
use crate::db::DbMsg;
use crate::error::TradeWebError;

#[derive(Debug, Deserialize, Serialize)]
pub struct QrySymbol {
    pub  symbol: String,
}

pub struct SymbolList {}

impl SymbolList {
    /// get a vec of stock symbols
    pub async fn get_active_symbols(tx_db: crossbeam_channel::Sender<DbMsg>) -> Result<Vec<String>, TradeWebError> {

        let (resp_tx, resp_rx) = crossbeam_channel::unbounded();
        let msg = DbMsg::SymbolListGetActive { sender_tx: resp_tx };
        tx_db.send(msg).unwrap();
        match resp_rx.recv(){
            Ok(result)=>Ok(result),
            Err(_e)=>Err(TradeWebError::ChannelError),
        }
    }

    /// get a vec of stock symbols
    pub async fn get_all_symbols(tx_db: crossbeam_channel::Sender<DbMsg>) -> Result<Vec<String>, TradeWebError> {
        let (resp_tx, resp_rx) = crossbeam_channel::unbounded();
        let msg = DbMsg::SymbolListGetAll{ sender_tx: resp_tx };
        tx_db.send(msg).unwrap();
        match resp_rx.recv(){
            Ok(result)=>Ok(result),
            Err(_e)=>Err(TradeWebError::ChannelError),
        }
    }
}
