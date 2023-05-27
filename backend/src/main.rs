//! main.rs
#![forbid(unsafe_code)]

pub mod data_collector;
pub mod db;
pub mod web_client_service;
pub mod websocket_service;

use crate::data_collector::DataCollector;
use common_lib::init::init;
use common_lib::settings::Settings;
use common_lib::sqlx_pool::create_sqlx_pg_pool;
use chrono::NaiveTime;
use once_cell::sync::Lazy;

// https://alpaca.markets/learn/investing-basics/what-is-extended-hours-trading/
// pub static MARKET_OPEN_TIME:Lazy<NaiveTime> = Lazy::new(||{ NaiveTime::from_hms_opt(0, 01, 0).unwrap() }); // 4am Eastern
pub static MARKET_OPEN_TIME:Lazy<NaiveTime> = Lazy::new(||{ NaiveTime::from_hms_opt(9, 30, 0).unwrap() }); // 4am Eastern
pub static MARKET_CLOSE_TIME:Lazy<NaiveTime> = Lazy::new(||{ NaiveTime::from_hms_opt(16, 0, 0).unwrap() }); // 4pm
// pub static MARKET_CLOSE_TIME:Lazy<NaiveTime> = Lazy::new(||{ NaiveTime::from_hms_opt(23, 59, 0).unwrap() }); // 8pm pacific


/// main
fn main() {
    // this was useful in another cargo workspace where the path was specific to the crate inside the workspace

    init(env!("CARGO_MANIFEST_DIR"));

    // if you care to see what your macros are doing...
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .on_thread_start(|| {})
        .on_thread_stop(|| {})
        .thread_name("alpaca")
        .enable_all()
        .build()
        .expect("Tokio runtime didn't start");

    tokio_runtime.block_on(async {

        let pool = create_sqlx_pg_pool().await;
        match Settings::load(&pool).await{
            Ok(settings) =>{
                DataCollector::start(pool, &settings).await;
            },
            Err(e) => {
                tracing::debug!("[main] could not load settings: {:?}", &e);
            }
        }
    });
}
