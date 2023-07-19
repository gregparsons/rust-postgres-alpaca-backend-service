//! alpaca_transaction_status


use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use crate::alpaca_position::Position;

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
    pub async fn start_sell(symbol:&str, pool:&PgPool)->Result<BigDecimal, TransactionError>{
        match sqlx::query_as!(AlpacaTransaction, r#"select dtg as "dtg!", symbol as "symbol!", posn_shares as "posn_shares!" from alpaca_transaction_status where symbol=$1 and posn_shares > 0.0"#, symbol.to_lowercase()).fetch_one(pool).await{
            Ok(transaction)=> Ok(transaction.posn_shares),
            Err(_e)=>Err(TransactionError::NoSharesFound), // or db error
        }
    }

    /// insert a new transaction if one doesn't currently exist, otherwise error
    pub async fn delete(symbol:&str, pool:&PgPool)->Result<(), TransactionError>{
        match sqlx::query!(r#"delete from alpaca_transaction_status where symbol=$1"#, symbol.to_lowercase()).execute(pool).await{
            Ok(_)=>Ok(()),
            Err(_e)=>Err(TransactionError::DeleteFailed), // or db error
        }
    }

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

    /// start the order slate blank
    pub async fn delete_all(pool:&PgPool)->Result<(), TransactionError>{
        match sqlx::query!(
            r#"
                delete from alpaca_transaction_status
            "#
        ).execute(pool).await{
            Ok(_)=>Ok(()),
            Err(_e)=>Err(TransactionError::DeleteFailed), // or db error
        }
    }


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
    pub async fn insert_existing_position(position:&Position, pool:&PgPool)->Result<(), TransactionError>{
        match sqlx::query!(
            r#"
                insert into alpaca_transaction_status(dtg, symbol, posn_shares)
                values(now()::timestamptz, $1, $2)
                ON CONFLICT (symbol) DO UPDATE
                    SET posn_shares = $2, dtg=now()::timestamptz;
            "#,
            position.symbol.to_lowercase(),
            position.qty
        ).execute(pool).await{
            Ok(_)=>Ok(()),
            Err(_e)=>Err(TransactionError::DeleteFailed), // or db error
        }
    }

}