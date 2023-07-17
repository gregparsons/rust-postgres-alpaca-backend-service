//! common_structs.rs

use bigdecimal::BigDecimal;
use chrono::{NaiveDateTime};
use serde::{Deserialize, Serialize};


pub static SESSION_USER_ID: &str = "session_user_id";
pub static SESSION_USERNAME: &str = "session_username";

#[derive(Serialize, Deserialize, Debug)]
pub struct FormData {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryAverage {
    pub dtg: NaiveDateTime,
    pub symbol: String,
    pub price: BigDecimal,
    // pub size: BigDecimal,
    // pub exchange: Option<i32>,
}



