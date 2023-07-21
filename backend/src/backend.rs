//! backend

/*

    Market

    Starts both REST and websocket services.

    Starts a DB thread to store results of rest/ws tickers.

    Performs analysis on incoming tickers.


*/

use crate::db::{DbActor, DbMsg};
use crate::alpaca_websocket::{AlpacaWebsocket};
use crate::finnhub_websocket::FinnhubWebsocket;
use common_lib::settings::Settings;
use common_lib::symbol_list::SymbolList;
use sqlx::PgPool;
use std::str::FromStr;
use std::time::Duration;
use crossbeam_channel::Sender;
use tokio::runtime::Handle;
use common_lib::alpaca_api_structs::WebsocketMessageFormat;
use crate::alpaca_rest::AlpacaRest;
use crate::stock_rating;

/// Spawn threads to collect Alpaca and Finnhub websocket feeds into a Postgresql database
pub struct Backend {}

// associated functions v methods
// https://doc.rust-lang.org/book/ch05-03-method-syntax.html
impl Backend {
    pub async fn run(settings: &Settings, tokio_handle: Handle) {
        let mut handles = vec![];

        /****** database actor thread ******/
        // use this crossbeam channel to transmit messages to the database
        // let pool_db = pool.clone();
        let db_actor = DbActor::new().await;
        let db_join_handle = tokio_handle.spawn(async {
            db_actor.run().await
        });

        let tx_db_result = db_join_handle.await;

        if let Ok(tx_db) = tx_db_result {

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
                tracing::debug!("[run] alpaca_ws_on: {}", alpaca_ws_on);
            } else {
                tracing::debug!("[run] alpaca_ws_on: {}", alpaca_ws_on);
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
                tracing::debug!("[run] finnhub_on: {}", finnhub_on);
            } else {
                tracing::debug!("[run] finnhub_on: {}", finnhub_on);
            }

            /****** alpaca rest polling ******/
            // Rest HTTP Service (in/out)
            let alpaca_rest_on = bool::from_str(std::env::var("ALPACA_REST_ON").unwrap_or_else(|_| "false".to_owned()).as_str()).unwrap_or(false);
            tracing::info!("ALPACA_REST_ON is: {}", alpaca_rest_on);

            let pool3 = pool.clone();
            let tx_db_rest = tx_db.clone();

            if alpaca_rest_on {
                // AlpacaRest::run().await;

                let join_handle = std::thread::spawn(move || {
                    // let tokio_handle = Handle::current();
                    AlpacaRest::run(tx_db_rest, txpool3, tokio_handle);
                });
                handles.push(join_handle);
                tracing::debug!("[run] alpaca_rest_on: {}", finnhub_on);
            } else {
                tracing::debug!("[run] alpaca_rest_on: {}", finnhub_on);
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
                tracing::debug!("[run] stock_rating_on: {}", finnhub_on);
            } else {
                tracing::debug!("[run] stock_rating_on: {}", finnhub_on);
            }


            loop {
                std::thread::sleep(Duration::from_secs(10));
            }

            // collect all the threads (which will never happen unless they all crash)
            // for h in handles {
            //     h.join().expect("thread stopped: {:?}");
            // }

            // tracing::debug!("[data_collector::run] all handles joined, exiting");
        }
    }
}
