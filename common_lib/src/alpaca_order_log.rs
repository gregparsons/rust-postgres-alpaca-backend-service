//! alpaca_order_log


use chrono::{DateTime, Utc};
use crate::alpaca_order::Order;

#[derive(Debug)]
pub struct AlpacaOrderLogEvent{
    pub dtg:DateTime<Utc>,
    pub event: String,
    pub order: Order,

}

impl AlpacaOrderLogEvent{

    // /// Save a single order to the database
    // pub async fn save_to_db(&self, tx_db: crossbeam_channel::Sender<DbMsg>) {
    //     let msg = DbMsg::OrderLogInsert {
    //         order: self.order.clone()
    //     };
    //     tx_db.send(msg).unwrap();
    // }

}