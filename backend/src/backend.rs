//! backend

/*

    Market

    Starts both REST and websocket services.

    Starts a DB thread to store results of rest/ws tickers.

    Performs analysis on incoming tickers.


*/

use crate::alpaca_websocket::{AlpacaWebsocket};
use crate::finnhub_websocket::FinnhubWebsocket;
use common_lib::settings::Settings;
use common_lib::symbol_list::SymbolList;
use std::str::FromStr;
use std::time::Duration;
use tokio::runtime::Handle;
use common_lib::alpaca_api_structs::WebsocketMessageFormat;
use common_lib::db::{DbActor, DbMsg};
use crate::alpaca_rest::AlpacaRest;
use crate::stock_rating;

/// Spawn threads to collect Alpaca and Finnhub websocket feeds into a Postgresql database


// associated functions v methods
// https://doc.rust-lang.org/book/ch05-03-method-syntax.html

pub async fn run(tokio_handle: Handle) {

    tracing::debug!("[run]");

    /****** database actor thread ******/

    let db_actor = DbActor::new().await;
    let tx_db = db_actor.tx.clone();
    let rt = tokio_handle.clone();

    std::thread::spawn(move || {
        tracing::debug!("[backend] db thread");
        db_actor.run(rt);
        tracing::debug!("[backend] db thread done");
    });
    tracing::debug!("[backend] db thread spawned");

    // start threads for rest polling, alpaca websocket, and finnhub websocket
    let tx_db_1 = tx_db.clone();
    let settings_result = Settings::load_with_secret(tx_db_1).await;
    match settings_result{

        Ok(settings)=>{

            tracing::debug!("[run] loaded settings: {:?}", &settings);

            /****** alpaca rest polling ******/
            // Rest HTTP Service (in/out)
            let alpaca_rest_on = bool::from_str(std::env::var("ALPACA_REST_ON").unwrap_or_else(|_| "false".to_owned()).as_str()).unwrap_or(false);
            tracing::info!("ALPACA_REST_ON is: {}", alpaca_rest_on);
            let tx_db_rest = tx_db.clone();

            tracing::debug!("[run] alpaca_rest_on: {}", alpaca_rest_on);

            if alpaca_rest_on {
                std::thread::spawn(||{
                    tracing::debug!("[run] REST API thread started...");
                    AlpacaRest::run(tx_db_rest, tokio_handle);
                });
            }


            /****** alpaca websocket ******/
            tracing::debug!("[run] db start() complete");
            let alpaca_ws_on = bool::from_str(std::env::var("ALPACA_WEBSOCKET_ON").unwrap_or_else(|_| "true".to_owned()).as_str()).unwrap_or(false);
            tracing::info!("ALPACA_WEBSOCKET_ON is: {}", &alpaca_ws_on);
            if alpaca_ws_on {

                tracing::debug!("[run] Starting alpaca text websocket service in new thread...");

                let tx_db_2 = tx_db.clone();

                match SymbolList::get_active_symbols(tx_db_2).await {
                    Ok(symbols) => {
                        let symbols2 = symbols.clone();
                        let settings2 = settings.clone();
                        let settings3 = settings.clone();
                        let tx_db_3 = tx_db.clone();

                        // alpaca market data websocket
                        let _join_handle_1 = std::thread::spawn(|| {
                            tracing::debug!("[run] starting text data websocket");
                            AlpacaWebsocket::run(tx_db_3, &WebsocketMessageFormat::TextData, symbols, settings3);
                        });

                        // alpaca account data websocket
                        let tx_db_4 = tx_db.clone();
                        let _join_handle_1 = std::thread::spawn(|| {
                            tracing::debug!("[run] starting binary data for 'trade_updates'");
                            AlpacaWebsocket::run(tx_db_4, &WebsocketMessageFormat::BinaryUpdates, symbols2, settings2);
                        });
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

                let tx_db_symbols = tx_db.clone();
                match SymbolList::get_active_symbols(tx_db_symbols).await{
                    Ok(symbols)=>{
                        let tx_db_ws = tx_db.clone();
                        let settings2 = settings.clone();
                        let _join_handle = std::thread::spawn(|| {
                            tracing::debug!("[finnhub] inside spawned thread...");
                            FinnhubWebsocket::run(tx_db_ws, symbols, settings2);
                        });
                    },
                    Err(e)=>tracing::debug!("[finnhub] could not load symbols, finnhub not started: {:?}", &e),
                }

                // handles.push(join_handle);
                tracing::debug!("[run] finnhub_on: {}", finnhub_on);
            } else {
                tracing::debug!("[run] finnhub_on: {}", finnhub_on);
            }

            // start the stock rating system
            // default on
            let stock_rating_on = bool::from_str(std::env::var("STOCK_RATING_ON").unwrap_or_else(|_| "true".to_owned()).as_str()).unwrap_or(true);
            tracing::info!("STOCK_RATING_ON is: {}", stock_rating_on);
            let tx_db2 = tx_db.clone();
            if stock_rating_on {
                let _join_handle = std::thread::spawn(|| {
                    stock_rating::run(tx_db2);
                });
                // handles.push(join_handle);
                tracing::info!("[run] stock_rating_on: {}", stock_rating_on);
            } else {
                tracing::error!("[run] stock_rating_on: {}", stock_rating_on);
            }

        },
        Err(e)=> tracing::debug!("[run] error getting settings: {:?}", &e),

    }


    // for h in handles{
    //     let _ = h.join();
    // }


    loop {
        std::thread::sleep(Duration::from_secs(3));
        tracing::info!("[run] database: {:?}", tx_db.send(DbMsg::PingDb));
        println!("[backend]");
    }
}

