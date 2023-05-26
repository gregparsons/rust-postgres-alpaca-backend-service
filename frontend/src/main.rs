//! main.rs
#![forbid(unsafe_code)]

mod activities;
mod account;
mod configuration;
mod edit_settings;
mod login;
mod metrics;
mod positions;
mod profit;
mod signup;
mod symbols;
mod utils;
mod web_server;
mod dashboard;
pub mod order;

use common_lib::init::init;

use chrono::NaiveTime;
use once_cell::sync::Lazy;
use crate::web_server::WebServer;

// https://alpaca.markets/learn/investing-basics/what-is-extended-hours-trading/
pub static MARKET_OPEN:Lazy<NaiveTime> = Lazy::new(||{ NaiveTime::from_hms_opt(9, 30, 0).unwrap() }); // 4am Eastern
pub static MARKET_CLOSE:Lazy<NaiveTime> = Lazy::new(||{ NaiveTime::from_hms_opt(16, 0, 0).unwrap() }); // 8pm

/// main
fn main() {
    // this was useful in another cargo workspace where the path was specific to the crate inside the workspace
    // init(concat!(env!("CARGO_MANIFEST_DIR"), "/.env"));
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
        WebServer::run().await;
    });
}
