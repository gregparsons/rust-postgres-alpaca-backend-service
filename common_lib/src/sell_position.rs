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

    // /// positions exceeding the age specified in the parameter filter
    // pub async fn list_showing_age(age_filter_minutes:BigDecimal, pool:&PgPool) ->Result<Vec<SellPosition>,PollerError>{
    //     let result = sqlx::query_as!(SellPosition,r#"
    //         select
    //             stock_symbol as "symbol!"
    //             , price as "avg_entry_price!"
    //             , sell_qty as "qty!"
    //             // , sell_qty_available as "qty_available!"
    //             , unrealized_pl_per_share as "unrealized_pl_per_share!"
    //             , cost as "cost_basis!"
    //             , unrealized_pl_total as "unrealized_pl_total!"
    //             , coalesce(trade_size,0.0) as "trade_size!"
    //             , coalesce(age_min,0.0) as "age_minute!"
    //         from fn_positions_to_sell_old($1) a
    //         left join t_symbol b on upper(a.stock_symbol) = upper(b.symbol)
    //     "#, age_filter_minutes).fetch_all(pool).await;
    //     match result {
    //         Ok(positions)=>Ok(positions),
    //         Err(_e)=> Err(PollerError::Sqlx)
    //     }
    // }
}