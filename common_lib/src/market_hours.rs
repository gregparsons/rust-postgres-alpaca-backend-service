//! market_hours.rs

use chrono::{NaiveTime, Utc};
use once_cell::sync::Lazy;

// pub const OPERATE_API_AFTER_HOURS: bool = true;
// side effect of BUY_EXTENDED_HOURS is causing buy limit orders, which can cause orders to hang, not immediately accepted
pub const BUY_EXTENDED_HOURS: bool = false;
pub const SELL_EXTENDED_HOURS: bool = false;

// prod
pub static MARKET_OPEN_TIME: Lazy<NaiveTime> = Lazy::new(|| NaiveTime::from_hms_opt(9, 30, 0).unwrap()); // 4am Eastern
pub static MARKET_CLOSE_TIME: Lazy<NaiveTime> = Lazy::new(|| NaiveTime::from_hms_opt(16, 0, 0).unwrap()); // 4pm

pub static MARKET_OPEN_EXT: Lazy<NaiveTime> = Lazy::new(|| NaiveTime::from_hms_opt(4, 0, 0).unwrap()); // 4am Eastern
pub static MARKET_CLOSE_EXT: Lazy<NaiveTime> = Lazy::new(|| NaiveTime::from_hms_opt(20, 0, 0).unwrap()); // 4pm

pub struct MarketHours {
}


impl MarketHours{
    pub fn is_open() -> bool{

        let time_current_ny = Utc::now().with_timezone(&chrono_tz::America::New_York).time();

        let operate_after_hours = std::env::var("OPERATE_API_AFTER_HOURS").unwrap_or_else(|_| "false".to_string()).parse().unwrap_or_else(|_| false);

        let open_for_testing = std::env::var("PRETEND_TO_BE_OPEN").unwrap_or_else(|_| "false".to_string()).parse().unwrap_or_else(|_| false);
        if open_for_testing{
            return true;
        }

        let are_we_open = match operate_after_hours {
            false => {
                if time_current_ny > *MARKET_OPEN_TIME /* *MARKET_NORMAL_OPEN_TIME */ &&
                    time_current_ny < *MARKET_CLOSE_TIME /* *MARKET_NORMAL_CLOSE_TIME */ {
                    (true, "Operating inside normal hours (0930-1600)")
                } else {
                    (false, "Outside normal hours (0930-1600)")
                }
            },
            true =>{
                if time_current_ny > *MARKET_OPEN_EXT /* *MARKET_EXTENDED_OPEN_TIME */ &&
                    time_current_ny < *MARKET_CLOSE_EXT /* *MARKET_EXTENDED_CLOSE_TIME */ {
                    (true, "Operating inside extended hours (0400-2000)")
                } else {
                    (false, "Outside extended hours (0400-2000)")
                }
            }
        };
        // tracing::debug!("[are_we_open] Time in NYC: {:?}; OPERATE_API_AFTER_HOURS: {}; are_we_open: {}:{}", &time_current_ny, OPERATE_API_AFTER_HOURS, &are_we_open.0, &are_we_open.1);
        are_we_open.0
    }
}