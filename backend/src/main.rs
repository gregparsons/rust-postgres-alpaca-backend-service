//! main.rs
#![forbid(unsafe_code)]

pub mod backend;
pub mod alpaca_rest;
pub mod alpaca_websocket;
pub mod finnhub_websocket;
mod stock_rating;

use tokio::runtime::Handle;
use common_lib::init::init;

/// main
fn main() {
    // this was useful in another cargo workspace where the path was specific to the crate inside the workspace

    init(env!("CARGO_MANIFEST_DIR"));

    // if you care to see what your macros are doing...
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        // default to number of cores
        // .worker_threads(8)
        // .on_thread_start(|| {})
        // .on_thread_stop(|| {})
        .thread_name("alpaca")
        .enable_all()
        .build()
        .expect("Tokio runtime didn't start");

    // spawn an non-worker thread as the main runtime thread
    // https://ryhl.io/blog/async-what-is-blocking/
    tokio_runtime.block_on(async {
        let tokio_handle = Handle::current();
        let _ = backend::run(tokio_handle).await;

    });
}
