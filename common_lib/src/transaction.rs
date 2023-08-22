//! transaction.rs
//!
//! display ongoing transactions from the database fn_transaction() function


use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use crate::db::DbMsg;
use crate::trade_struct::TradeSide;

#[derive(Debug, Deserialize, Serialize)]
pub struct Transaction{
    pub symbol:String,
    pub side:TradeSide,
    pub profit:Option<BigDecimal>,
    pub elapsed_sec:Option<BigDecimal>,
    pub posn_age_sec:Option<BigDecimal>,
    pub qty_transaction:BigDecimal,
    pub price_transaction:BigDecimal,
    pub amt_transaction:BigDecimal,
    pub qty_posn:BigDecimal,
    pub price_posn_entry:BigDecimal,
    pub posn_basis:BigDecimal,
    pub dtg:DateTime<Utc>,
    pub dtg_pacific:NaiveDateTime,
}

// impl Transaction{
    pub async fn get(symbol:Option<String>, tx_db:crossbeam_channel::Sender<DbMsg>)->Vec<Transaction>{
        let (tx, rx) = oneshot::channel();
        match tx_db.send(DbMsg::TransactionGet{symbol:symbol, sender:tx}){
            Ok(_)=>{
                match rx.await{
                    Ok(vec_transaction)=> vec_transaction,
                    Err(e)=>{
                        tracing::error!("[get] recv error: {:?}", &e);
                        vec!()
                    }
                }
            },
            Err(e)=>{
                tracing::error!("[get] send error: {:?}", &e);
                vec!()
            }
        }
    }
// }