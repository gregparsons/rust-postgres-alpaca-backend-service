//! alpaca_transaction_status


use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use crossbeam_channel::Sender;
use sqlx::PgPool;
use crate::alpaca_position::Position;
use crate::db::DbMsg;

pub enum TransactionStatus{
    Found{shares:BigDecimal},
    NotFound
}

#[derive(Debug)]
pub enum TransactionError{
    TransactionNotFound,
    BuyOrderExists,
    PositionExists,
    DeleteFailed,
    NoSharesFound,
}

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

impl AlpacaTransaction{

    /// insert a new transaction if one doesn't currently exist, otherwise error
    pub async fn start_buy(symbol:&str, pool:&PgPool)->Result<(), TransactionError>{
        // if an entry exists then a buy order exists; otherwise create one with default 0 shares
        match sqlx::query!(r#"

                insert into alpaca_transaction_status(dtg, symbol, posn_shares)
                values(now()::timestamptz, $1, 0.0

            )"#, symbol.to_lowercase()).execute(pool).await{
                Ok(_)=>{
                    Ok(())
                },
                Err(_e)=>Err(TransactionError::PositionExists), // or db error
            }
    }


    /// sell if a buy order previously created an entry in this table and subsequently the count of shares is greater than zero
    /// TODO: not currently used
    pub async fn start_sell(symbol:&str, tx_db:Sender<DbMsg>) {
        tx_db.send(TransactionStartSell {symbol:symbol.to_string()}).unwrap()
    }

    /// start the order slate blank
    pub fn delete_one(symbol:&str, pool:&PgPool) ->Result<(), TransactionError>{
        tx_db.send(DbMsg::TransactionDeleteOne).unwrap();
    }

    /// start the order slate blank
    pub fn delete_all(tx_db: Sender<DbMsg>){
        tx_db.send(DbMsg::TransactionDeleteAll).unwrap();
    }

    /// TODO: move to database; for now only called from within database crossbeam message anyway
    /// delete all "orders" without positions; start_buy() relies on the non-existence of a symbol to start an order
    pub async fn clean(pool:&PgPool)->Result<(), TransactionError>{
        match sqlx::query!(
            r#"
                delete from alpaca_transaction_status
                where posn_shares <= 0.0
            "#
        ).execute(pool).await{
            Ok(_)=>Ok(()),
            Err(_e)=>Err(TransactionError::DeleteFailed), // or db error
        }
    }


    /// TODO: move to database; for now only called from within database crossbeam message anyway
    /// insert a new transaction if one doesn't currently exist, otherwise error
    pub async fn decrement(symbol:&str, shares_to_decrement:BigDecimal, pool:&PgPool)->Result<(), TransactionError>{
        match sqlx::query!(
            r#"
                update alpaca_transaction_status
                set posn_shares = posn_shares - $1 where symbol=$2
            "#,
            shares_to_decrement,
            symbol.to_lowercase()
        ).execute(pool).await{
            Ok(_)=>Ok(()),
            Err(_e)=>Err(TransactionError::DeleteFailed), // or db error
        }
    }

    /// create a new entry or update the position's shares with the current timestamp
    pub fn insert_existing_position(position:&Position, tx_db:crossbeam_channel::Sender<DbMsg>){
        let _ = tx_db.send(DbMsg::TransactionInsertPosition { position: position.clone() });
    }

}