//! main.rs
#![forbid(unsafe_code)]

pub mod data_collector;
pub mod db;
pub mod alpaca_rest;
pub mod alpaca_websocket;
pub mod finnhub_websocket;
mod stock_rating;

use tokio::runtime::Handle;
use crate::data_collector::DataCollector;
use common_lib::init::init;
use common_lib::settings::Settings;
use common_lib::sqlx_pool::create_sqlx_pg_pool;

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
        match Settings::load_with_secret(&pool).await {
            Ok(settings) => {
                let tokio_handle = Handle::current();
                DataCollector::run(pool, &settings, tokio_handle).await;
            }
            Err(e) => {
                tracing::error!("[main] could not load settings: {:?}", &e);
            }
        }
    });
}
