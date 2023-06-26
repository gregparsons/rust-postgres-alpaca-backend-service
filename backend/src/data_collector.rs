//! backend

/*

    Market

    Starts both REST and websocket services.

    Starts a DB thread to store results of rest/ws tickers.

    Performs analysis on incoming tickers.


*/

use crate::db::DbActor;
use crate::websocket_service::{AlpacaStream, AlpacaWebsocket};
use crate::ws_finnhub::FinnhubWebsocket;
use common_lib::finnhub::FinnhubStream;
use common_lib::settings::Settings;
use common_lib::symbol_list::SymbolList;
use sqlx::PgPool;
use std::str::FromStr;
use crate::rest_client;

/// Spawn threads to collect Alpaca and Finnhub websocket feeds into a Postgresql database
pub struct DataCollector {}

// associated functions v methods
// https://doc.rust-lang.org/book/ch05-03-method-syntax.html
impl DataCollector {

    pub async fn run(pool: PgPool, settings: &Settings) {

        /****** alpaca websocket ******/
        let tx_db = DbActor::start().await;
        tracing::debug!("[Market::start] db start() complete");
        let alpaca_ws_on = bool::from_str(std::env::var("ALPACA_WEBSOCKET_ON").unwrap_or_else(|_| "true".to_owned()).as_str()).unwrap_or(false);
        tracing::info!("ALPACA_WEBSOCKET_ON is: {}", &alpaca_ws_on);
        if alpaca_ws_on {
            tracing::debug!("Starting alpaca text websocket service in new thread...");
            // spawn long-running text thread
            let tx_db_ws = tx_db.clone();
            let ws_pool = pool.clone();
            let settings2 = (*settings).clone();
            match SymbolList::get_active_symbols(&ws_pool).await {
                Ok(symbols) => {
                    let _jh = std::thread::spawn(move || {  AlpacaWebsocket::run(tx_db_ws, &AlpacaStream::TextData, symbols, settings2); });
                },
                Err(e) => tracing::debug!("[start] error getting symbols for websocket: {:?}", &e),
            }
        }

        /****** finnhub websocket ******/
        let finnhub_on = bool::from_str(std::env::var("FINNHUB_ON").unwrap_or_else(|_| "true".to_owned()).as_str()).unwrap_or(true);
        tracing::info!("FINNHUB_ON is: {}", &finnhub_on);
        if finnhub_on {
            tracing::debug!("Starting Finnhub text websocket service in new thread...");
            // spawn long-running text thread
            let tx_db_ws = tx_db.clone();
            let ws_pool = pool.clone();
            let settings2 = (*settings).clone();
            // this symbol call is repeated because eventually finnhub may use a different symbol list
            match SymbolList::get_active_symbols(&ws_pool).await {
                Ok(symbols) => {
                    let _jh = std::thread::spawn(|| async move { FinnhubWebsocket::run(tx_db_ws, &FinnhubStream::TextData, symbols, settings2).await; });
                },
                Err(e) => tracing::debug!("[start] error getting symbols for websocket: {:?}", &e),
            }
        }

        /****** alpaca rest polling ******/
        // Rest HTTP Service (in/out)
        let alpaca_rest_on = bool::from_str(std::env::var("ALPACA_REST_ON").unwrap_or_else(|_| "false".to_owned()).as_str()).unwrap_or(false);
        tracing::info!("ALPACA_REST_ON is: {}", alpaca_rest_on);
        if alpaca_rest_on {
            rest_client::run().await;
        }


        // infinite loop to keep child threads alive
        loop {
            // TODO: join all thread handles instead
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }
}
