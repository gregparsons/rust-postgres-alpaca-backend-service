//! alpaca_order_log


use chrono::{DateTime, Utc};
use crate::alpaca_order::Order;

#[derive(Debug)]
pub struct AlpacaOrderLogEvent{
    pub dtg:DateTime<Utc>,
    pub event: String,
    pub order: Order,

}

