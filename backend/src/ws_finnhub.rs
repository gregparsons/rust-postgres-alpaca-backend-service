//! ws_finnhub.rs




/**
    Websocket client for Alpaca

    Current hard-coded stocks:
    aapl, tsla, plug, aal, nio, bac
*/
// use crossbeam_channel::{after, select, tick};
use crate::db::DbMsg;
use crossbeam::channel::Sender;
use serde_json::json;
use tungstenite::client::IntoClientRequest;
use tungstenite::{Message};
use common_lib::settings::Settings;
use common_lib::finnhub::{FinnhubPacket, FinnhubPing, FinnhubStream, FinnhubSubscribe};
use std::time::{Duration};

fn stock_list_to_uppercase(lower_stock:&Vec<String>)-> Vec<String>{
    lower_stock.iter().map(|x| x.to_uppercase() ).collect()
}

pub struct WsFinnhub;

impl WsFinnhub {
    pub async fn run(tx_db: Sender<DbMsg>, stream_type:&FinnhubStream, symbols:Vec<String>, settings:Settings) {
        tracing::debug!("[WsFinnhub::run]");
        WsFinnhub::connect(tx_db, stream_type, symbols, &settings).await;
    }

    async fn connect(tx_db: Sender<DbMsg>, stream_type:&FinnhubStream, symbols:Vec<String>, settings:&Settings) {

        // wss://ws.finnhub.io?token=xxxxxxxx
        // .env includes everything except the api key value (xxxxxx); called token here
        // TODO: binary not needed for finnhub
        let ws_url = match stream_type{
            FinnhubStream::TextData => std::env::var("FINNHUB_URL").expect("FINNHUB_URL not found"),
            FinnhubStream::BinaryUpdates => std::env::var("FINNHUB_URL").expect("FINNHUB_URL not found"),
        };

        let ws_url = format!("{}{}", ws_url, settings.finnhub_key);

        // websocket restart loop
        loop {

            let url = url::Url::parse(&ws_url).unwrap();
            let request = (&url).into_client_request().unwrap();

            // commence websocket connection
            match tungstenite::connect(request) {
                Err(e) => tracing::debug!("[WsFinnhub::connect] websocket connect error: {:?}", e),

                Ok((mut ws, _response)) => {
                    tracing::debug!("[WsFinnhub::connect] successful websocket connection; response: {:?}", _response);

                    /*
                    2023-05-31T21:23:42.256121Z DEBUG backend::ws_finnhub: [WsFinnhub::connect] successful websocket connection; response: Response { status: 101, version: HTTP/1.1, headers: {"date": "Wed, 31 May 2023 21:23:43 GMT", "connection": "upgrade", "upgrade": "websocket", "sec-websocket-accept": "XLvDaH0hCELNbMnjEJFm/AZcf8I=", "cf-cache-status": "DYNAMIC", "report-to": "{\"endpoints\":[{\"url\":\"https:\/\/a.nel.cloudflare.com\/report\/v3?s=0B5jxuyY0Bc%2FaXpEeJ67xAOdM%2B4GMmAXGJpSdZuGlpB%2FzOVJLibsbfUL3Mf%2F1yZkFUAs%2BKX3KXRzpYmdq%2B%2FgXoRE81lt4TaesP1aUtcsP0eyDfrjMEL9yImHrXWfQzeU\"}],\"group\":\"cf-nel\",\"max_age\":604800}", "nel": "{\"success_fraction\":0,\"report_to\":\"cf-nel\",\"max_age\":604800}", "server": "cloudflare", "cf-ray": "7d0247931bbece94-SJC", "alt-svc": "h3=\":443\"; ma=86400"}, body: None }
                     */

                    // Subscribe to all symbols
                    for symbol in stock_list_to_uppercase(&symbols){
                        // {"type":"subscribe","symbol":"TSLA"}
                        let subscribe = json!(FinnhubSubscribe{
                            websocket_message_type: "subscribe".to_string(),
                            symbol
                        });
                        tracing::debug!("[WsFinnhub] subscribe: {}", &subscribe.to_string());
                        let _ = ws.write_message(Message::Text(subscribe.to_string()));
                    }

                    loop {
                        // tracing::debug!("[ws_connect] reading websocket...");
                         match ws.read_message() {

                             Ok(msg) => {
                                 tracing::debug!("[WsFinnhub::connect] read websocket...");
                                 match msg {
                                     Message::Ping(t) => {
                                         // not used by finnhub; they use json over Text
                                         tracing::debug!("[WsFinnhub::connect][ping] {:?}", &t);
                                     },
                                     Message::Binary(b_msg) => {
                                         tracing::debug!("[WsFinnhub::connect][binary] {:?}", &b_msg);
                                     }
                                     Message::Text(t_msg) => {
                                         tracing::debug!("[WsFinnhub::connect][text] {}",&t_msg);

                                         match serde_json::from_str::<FinnhubPacket>(&t_msg){
                                             Ok(FinnhubPacket::Trade(trades))=>{
                                                 tracing::debug!("[deserialize] {:?}", &trades);

                                                 for trade in &trades {
                                                     let _ = tx_db.send(DbMsg::FhTrade(trade.clone()));
                                                 }
                                             },
                                             Ok(FinnhubPacket::Ping)=>{
                                                 tracing::info!("[Finnhub] ping");
                                                 let _ = tx_db.send(DbMsg::FhPing(FinnhubPing{ dtg: chrono::Utc::now() }));

                                             },
                                             Err(e)=>{
                                                 tracing::debug!("[deserialize] FinnhubPacket json error {:?}", &e);
                                             }
                                         }
                                     }
                                     _ => {
                                         tracing::debug!("[WsFinnhub::connect] websocket non-text, non-binary data: {:?}", &msg);
                                     }
                                 }
                             },
                             Err(e)=>{
                                 tracing::debug!("[ws_finnhub::connect] error reading message: {:?}", &e);
                             }
                        }
                    }
                }
            };

            // 5 second delay if the websocket goes down, then retry
            std::thread::sleep(Duration::from_millis(5000));

        }
    }



}


