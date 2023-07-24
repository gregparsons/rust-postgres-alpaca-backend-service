//! main.rs
#![forbid(unsafe_code)]

mod account;
mod activities;
mod configuration;
mod dashboard;
mod edit_settings;
mod login;
mod metrics;
pub mod order;
mod positions;
mod profit;
mod signup;
mod symbols;
mod utils;
mod web_server;

use common_lib::init::init;

use crate::web_server::WebServer;
use chrono::NaiveTime;
use once_cell::sync::Lazy;
use common_lib::db::{DbActor};

// https://alpaca.markets/learn/investing-basics/what-is-extended-hours-trading/
pub static MARKET_OPEN: Lazy<NaiveTime> = Lazy::new(|| NaiveTime::from_hms_opt(9, 30, 0).unwrap()); // 4am Eastern
pub static MARKET_CLOSE: Lazy<NaiveTime> = Lazy::new(|| NaiveTime::from_hms_opt(16, 0, 0).unwrap()); // 8pm

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

        let db_actor = DbActor::new().await;
        let tr = tokio::runtime::Handle::current();
        let _ = db_actor.run(tr);
        let tx_db = db_actor.tx.clone();
        // tracing::debug!("[main] tx_db: {:?}", &tx_db);
        WebServer::run(tx_db).await;

        // let tick = crossbeam::channel::tick(Duration::from_secs(2));
        // for _ in 0..5 {
        //     tracing::debug!("[main] ping db result: {:?}", tx_db.send(DbMsg::PingDb));
        //     tick.recv().unwrap();
        // }

    });
}
