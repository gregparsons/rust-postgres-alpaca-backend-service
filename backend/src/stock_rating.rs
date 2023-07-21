//!
//! stock_rating.rs
//!
//! Super simple timer to run a query that checks the recent sales and current positions and adjusts
//! the amount of shares traded for each stock according to it's immediate recent success. At present
//! all of the logic is in SQL, but roughly the idea is there's a rating range of 1-10. A successful
//! recent trade (positive profit and low elapsed time between buy/sell) then the rating for that
//! stock gets bumped +1. A position that's getting long in the tooth or has a previous sale resulting
//! in a loss or a really long buy/sell period gets degraded -1.
//!


use std::time::Duration;
use crossbeam_channel::{Sender, tick};
use common_lib::db::DbMsg;
use common_lib::market_hours::MarketHours;

const RATING_REFRESH_SECS:u64=30;

pub fn run(tx: Sender<DbMsg>){

    // tracing::debug!("[stock_rating::run]");
    let ticker = tick(Duration::from_secs(RATING_REFRESH_SECS));

    loop {

        if MarketHours::is_open(){
            tracing::debug!("[run] sending DbMsg::RefreshRating");
            let _send_result = tx.send(DbMsg::RefreshRating);
        }else{
            tracing::debug!("[run] not sending DbMsg::RefreshRating, market closed");
        }

        ticker.recv().unwrap();
    }
}

