//! position_local.rs
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::oneshot;
use crate::db::DbMsg;

#[derive(Debug, Serialize)]
pub struct PositionLocal{
    pub symbol: String,
    pub qty: BigDecimal,
    pub pl: BigDecimal,
    pub pl_per_share: BigDecimal,
    pub basis: BigDecimal,
    pub market_value: BigDecimal,
    pub price: BigDecimal,
    pub dtg: DateTime<Utc>,
    pub posn_age_sec: BigDecimal

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