//! alpaca_transaction_status


use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use crossbeam_channel::Sender;
use sqlx::PgPool;
use tokio::sync::oneshot;
use crate::alpaca_position::Position;
use crate::db::DbMsg;
use crate::error::TradeWebError;

// poller includes
//
// use bigdecimal::BigDecimal;
// use chrono::{DateTime, Utc};
// use crossbeam_channel::Sender;
// use sqlx::PgPool;
// use crate::common::alpaca_position::Position;
// use crate::common::db::DbMsg;
// use crate::common::error::TradeWebError;

pub enum TransactionStatus{
    Found{shares:BigDecimal},
    NotFound
}

// #[derive(Debug)]
// pub enum TransactionError{
//     TransactionNotFound,
//     BuyOrderExists,
//     PositionExists,
//     DeleteFailed,
//     NoSharesFound,
// }

#[derive(PartialEq)]
pub enum TransactionNextStep {
    Continue,
    DeleteTransaction,

}

/// reflect an entry in the alpaca_transaction_status table
#[allow(dead_code)]
pub struct AlpacaTransaction{
    dtg:DateTime<Utc>,
    symbol:String,
    posn_shares:BigDecimal,
}

#[derive(Debug, PartialEq)]
pub enum BuyResult{
    Allowed,
    NotAllowed{error:TradeWebError},
}

impl AlpacaTransaction{

    /// insert a new transaction if one doesn't currently exist, otherwise error


    // TODO: combine this with Account::buy_decision_cash_available
    pub async fn buy_check(symbol:&str, tx_db:Sender<DbMsg>) -> BuyResult {
        let (tx, rx) = oneshot::channel();
        tx_db.send(DbMsg::TransactionStartBuy { symbol:symbol.to_string(), sender: tx}).unwrap();
        match rx.await {
            Ok(buy_result) => buy_result,
            Err(_) => BuyResult::NotAllowed { error: TradeWebError::ChannelError },
        }

    }

    // /// sell if a buy order previously created an entry in this table and subsequently the count of shares is greater than zero
    // /// TODO: not currently used
    // pub async fn start_sell(symbol:&str, tx_db:Sender<DbMsg>) {
    //     tx_db.send(TransactionStartSell {symbol:symbol.to_string()}).unwrap()
    // }


    /// start the order slate blank
    pub fn delete_one(symbol:&str, tx_db: Sender<DbMsg>) {
        tx_db.send(DbMsg::TransactionDeleteOne{ symbol:symbol.to_string()}).unwrap()
    }

    /// start the order slate blank
    pub fn delete_all(tx_db: Sender<DbMsg>){
        tx_db.send(DbMsg::TransactionDeleteAll).unwrap();
    }

    /// TODO: move to database; for now only called from within database crossbeam message anyway
    /// delete all "orders" without positions; start_buy() relies on the non-existence of a symbol to start an order
    pub async fn clean(pool:&PgPool)->Result<(), TradeWebError>{
        match sqlx::query!(
            r#"
                delete from alpaca_transaction_status
                where posn_shares <= 0.0
            "#
        ).execute(pool).await{
            Ok(_)=>Ok(()),
            Err(_e)=>Err(TradeWebError::DeleteFailed), // or db error
        }
    }

    /// TODO: move to database; for now only called from within database crossbeam message anyway
    /// insert a new transaction if one doesn't currently exist, otherwise error
    pub async fn decrement(symbol:&str, shares_to_decrement:BigDecimal, pool:&PgPool)->Result<(), TradeWebError>{
        match sqlx::query!(
            r#"
                update alpaca_transaction_status
                set posn_shares = posn_shares - $1 where symbol=$2
            "#,
            shares_to_decrement,
            symbol.to_lowercase()
        ).execute(pool).await{
            Ok(_)=>Ok(()),
            Err(_e)=>Err(TradeWebError::DeleteFailed), // or db error
        }
    }

    /// create a new entry or update the position's shares with the current timestamp
    pub fn insert_existing_position(position:&Position, tx_db:crossbeam_channel::Sender<DbMsg>){
        let _ = tx_db.send(DbMsg::TransactionInsertPosition { position: position.clone() });
    }

}