//! backend

/*

    Market

    Starts both REST and websocket services.

    Starts a DB thread to store results of rest/ws tickers.

    Performs analysis on incoming tickers.


*/

use crate::db::DbActor;
use crate::alpaca_websocket::{AlpacaWebsocket};
use crate::finnhub_websocket::FinnhubWebsocket;
use common_lib::settings::Settings;
use common_lib::symbol_list::SymbolList;
use sqlx::PgPool;
use std::str::FromStr;
use common_lib::alpaca_api_structs::WebsocketMessageFormat;
use crate::alpaca_rest::AlpacaRest;
use crate::stock_rating;

/// Spawn threads to collect Alpaca and Finnhub websocket feeds into a Postgresql database
pub struct DataCollector {}

// associated functions v methods
// https://doc.rust-lang.org/book/ch05-03-method-syntax.html
impl DataCollector {

    pub async fn run(pool: PgPool, settings: &Settings) {

        let mut handles = vec![];

        /****** database actor thread ******/
        // use this crossbeam channel to transmit messages to the database
        let tx_db = DbActor::start().await;

        /****** alpaca websocket ******/
        tracing::debug!("[Market::start] db start() complete");
        let alpaca_ws_on = bool::from_str(std::env::var("ALPACA_WEBSOCKET_ON").unwrap_or_else(|_| "true".to_owned()).as_str()).unwrap_or(false);
        tracing::info!("ALPACA_WEBSOCKET_ON is: {}", &alpaca_ws_on);
        if alpaca_ws_on {
            tracing::debug!("Starting alpaca text websocket service in new thread...");
            // spawn long-running text thread
            let tx_db_ws = tx_db.clone();
            let ws_pool = pool.clone();
            match SymbolList::get_active_symbols(&ws_pool).await {

                Ok(symbols) => {
                    let tx_db_ws2 = tx_db_ws.clone();
                    let symbols2 = symbols.clone();
                    let settings2 = settings.clone();
                    let settings3 = settings.clone();

                    // stock data websocket thread
                    let join_handle = std::thread::spawn(|| {
                        tracing::debug!("[run] starting text data websocket");
                        AlpacaWebsocket::run(tx_db_ws, &WebsocketMessageFormat::TextData, symbols, settings3);
                        // AlpacaWebsocket::run(tx_db_ws.clone(), &AlpacaData::BinaryUpdates, symbols.clone(), settings2.clone());
                    });
                    handles.push(join_handle);

                    // account and order update websocket thread
                    let join_handle = std::thread::spawn(|| {
                        tracing::debug!("[run] starting binary data for 'trade_updates'");
                        // AlpacaWebsocket::run(tx_db_ws, &AlpacaData::TextData, symbols, settings2);
                        AlpacaWebsocket::run(tx_db_ws2, &WebsocketMessageFormat::BinaryUpdates, symbols2, settings2);
                    });
                    handles.push(join_handle);

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
                    let join_handle = std::thread::spawn(|| {
                        FinnhubWebsocket::run(tx_db_ws, symbols, settings2);
                    });
                    handles.push(join_handle);
                },
                Err(e) => tracing::debug!("[start] error getting symbols for websocket: {:?}", &e),
            }
        }

        /****** alpaca rest polling ******/
        // Rest HTTP Service (in/out)
        let alpaca_rest_on = bool::from_str(std::env::var("ALPACA_REST_ON").unwrap_or_else(|_| "false".to_owned()).as_str()).unwrap_or(false);
        tracing::info!("ALPACA_REST_ON is: {}", alpaca_rest_on);
        if alpaca_rest_on {
            AlpacaRest::run().await;
        }



        // start the stock rating system
        // default on
        let stock_rating_on = bool::from_str(std::env::var("STOCK_RATING_ON").unwrap_or_else(|_| "true".to_owned()).as_str()).unwrap_or(true);
        tracing::info!("STOCK_RATING_ON is: {}", stock_rating_on);
        let tx_db2 = tx_db.clone();
        if stock_rating_on {
            let join_handle = std::thread::spawn(|| {
                stock_rating::run(tx_db2);
            });
            handles.push(join_handle);
        }



        // collect all the threads (which will never happen unless they all crash)
        for h in handles {
            h.join().expect("thread stopped: {:?}");
        }
    }
}
