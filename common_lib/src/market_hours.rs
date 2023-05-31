//! market_hours.rs


use chrono::NaiveTime;
use once_cell::sync::Lazy;

// prod
// pub static MARKET_OPEN_TIME:Lazy<NaiveTime> = Lazy::new(||{ NaiveTime::from_hms_opt(9, 30, 0).unwrap() }); // 4am Eastern
// pub static MARKET_CLOSE_TIME:Lazy<NaiveTime> = Lazy::new(||{ NaiveTime::from_hms_opt(16, 0, 0).unwrap() }); // 4pm

// debug
pub static MARKET_OPEN_TIME:Lazy<NaiveTime> = Lazy::new(||{ NaiveTime::from_hms_opt(0, 01, 0).unwrap() }); // 4am Eastern
pub static MARKET_CLOSE_TIME:Lazy<NaiveTime> = Lazy::new(||{ NaiveTime::from_hms_opt(23, 59, 0).unwrap() }); // 4pm
