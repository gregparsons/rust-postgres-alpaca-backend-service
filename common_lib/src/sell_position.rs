//! position_sell
//!
//! Get a position snapshot with market value and purchase price. Get a list of positions recommended
//! for sale if they can be sold higher than purchase price per share by a certain number of cents.
//!


use bigdecimal::BigDecimal;
use crossbeam_channel::Sender;
use crate::db::DbMsg;
use crate::error::{TradeWebError};

/// used to receive the output of the SQL function fn_positions_to_sell
#[derive(Debug, serde::Deserialize)]
pub struct SellPosition {
    pub symbol:String,
    pub avg_entry_price:BigDecimal,
    pub qty:BigDecimal,
    // pub qty_available:BigDecimal,
    pub unrealized_pl_per_share:BigDecimal,
    pub cost_basis:BigDecimal,
    pub unrealized_pl_total:BigDecimal,
    pub trade_size:BigDecimal,
    pub age_minute:BigDecimal,

}

impl SellPosition {

    /// positions with a market value per share higher than their purchase price per share.
    pub async fn list_showing_profit(pl_filter:BigDecimal, sender_tx:Sender<DbMsg>) -> Result<Vec<SellPosition>, TradeWebError> {
        let (resp_tx, resp_rx) = crossbeam_channel::unbounded();
        sender_tx.send(DbMsg::PositionListShowingProfit { pl_filter, sender_tx: resp_tx}).unwrap();
        match resp_rx.recv(){
            Ok(sell_list)=>Ok(sell_list),
            Err(_e)=>Err(TradeWebError::ChannelError),
        }
    }

    /// old positions
    pub async fn list_showing_age(sender_tx:Sender<DbMsg>) -> Result<Vec<SellPosition>, TradeWebError> {
        let (resp_tx, resp_rx) = crossbeam_channel::unbounded();
        sender_tx.send(DbMsg::PositionListShowingAge {sender_tx: resp_tx}).unwrap();
        match resp_rx.recv(){
            Ok(sell_list)=>Ok(sell_list),
            Err(_e)=>Err(TradeWebError::ChannelError),
        }
    }

}