//! symbol
//!
//! get the active symbols from the database

use bigdecimal::BigDecimal;
use sqlx::PgPool;
use serde::{Deserialize, Serialize};

use crate::db::DbMsg;
use crate::error::{PollerError, TradeWebError};

#[derive(Debug, Deserialize, Serialize)]
pub struct Symbol {
    pub symbol:String,
    pub active:bool,
    pub trade_size:BigDecimal,
}

impl Symbol {
    /// get a stock symbol
    pub fn load_one(symbol:String, tx_db: crossbeam_channel::Sender<DbMsg>) -> Result<Symbol, TradeWebError>{

        let (resp_tx, resp_rx) = crossbeam_channel::unbounded();
        let msg = DbMsg::SymbolLoadOne{ symbol, sender_tx:resp_tx };
        tx_db.send(msg).unwrap();
        match resp_rx.recv() {
            Ok(symbol)=>Ok(symbol),
            Err(_e)=>Err(TradeWebError::ChannelError),
        }
    }

/*    /// get a vec of stock symbols
    pub async fn load_all(pool: &PgPool) -> Result<Vec<String>, PollerError> {
        match sqlx::query_as!(Symbol,
        r#"
            select
                symbol as "symbol!"
                ,active as "active!"
                ,coalesce(trade_size,0.0) as "trade_size!"
            from t_symbol"#
    ).fetch_all(pool).await {
            Ok(symbol_list) => {
                tracing::debug!("[get_symbols] symbol_list: {:?}", &symbol_list);
                let s = symbol_list.iter().map(|x| { x.symbol.clone() }).collect();
                Ok(s)
            },
            Err(e) => {
                tracing::debug!("[get_symbols] error: {:?}", &e);
                Err(PollerError::Sqlx)
            }
        }
    }*/

    /// get a vec of active stock symbols
    pub async fn load_active(pool: &PgPool) -> Result<Vec<String>, PollerError> {
        match sqlx::query_as!(Symbol,
        r#"
            select
                symbol as "symbol!"
                ,active as "active!"
                ,coalesce(trade_size,0.0) as "trade_size!"
            from t_symbol where active=true"#
    ).fetch_all(pool).await {
            Ok(symbol_list) => {
                tracing::debug!("[get_symbols] symbol_list: {:?}", &symbol_list);
                let s = symbol_list.iter().map(|x| { x.symbol.clone() }).collect();
                Ok(s)
            },
            Err(e) => {
                tracing::debug!("[get_symbols] error: {:?}", &e);
                Err(PollerError::Sqlx)
            }
        }
    }
}