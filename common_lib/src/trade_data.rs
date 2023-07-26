//! trade_data.rs

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeData {
    pub dtg: DateTime<Utc>,
    pub symbol: String,
    pub price: BigDecimal,
    // pub size: BigDecimal,
    // pub exchange: Option<i32>,
    pub current_time: DateTime<Utc>,

}