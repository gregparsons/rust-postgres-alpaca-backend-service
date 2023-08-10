//! position_local.rs
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::oneshot;
use crate::db::DbMsg;

#[derive(Debug, Serialize)]
pub struct PositionLocal{
    pub symbol: String,
    pub profit_closed: BigDecimal,
    pub pl_posn: BigDecimal,
    pub pl_posn_share: BigDecimal,
    // pub elapsed_sec:BigDecimal,
    pub posn_age_sec: BigDecimal,
    pub qty_posn: BigDecimal,
    pub price_posn_entry: BigDecimal,
    pub basis: BigDecimal,
    pub price_market: BigDecimal,
    pub market_value: BigDecimal,
    pub dtg: DateTime<Utc>,


}

impl PositionLocal{
    pub async fn get_all(tx_db:crossbeam_channel::Sender<DbMsg>)->Vec<PositionLocal>{
        let (tx, rx) = oneshot::channel();
        tx_db.send(DbMsg::PositionLocalGet { sender: tx}).unwrap();
        match rx.await{
            Ok(vec)=>vec,
            Err(_e) => vec!(),
        }
    }
}