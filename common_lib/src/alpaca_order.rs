//! alpaca_order

//!
//! remote_position.rs
//!
//! Sync positions and orders to a local data structure from the remote.
//!
//! Why?
//! 1. If want to buy, check here before buying. If there's already an outstanding order OR position, don't buy.
//! 2. If want to sell, check here, there needs to be an outstanding position to actually sell. (or really
//! make sure the settings to disallow short sales is turned off.
//!
//! TODO: make database calls non-blocking (either with dedicated db thread and crossbeam or tokio spawn)
//!

//! alpaca_order
//!

use std::fmt;
use serde::{Serialize, Deserialize};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use crossbeam_channel::Sender;
use reqwest::header::HeaderMap;
use sqlx::{Error, PgPool};
use sqlx::postgres::PgQueryResult;
use crate::db::DbMsg;
use crate::error::TradeWebError;
use crate::settings::Settings;
use crate::trade_struct::{OrderType, TimeInForce, TradeSide};

#[derive(Deserialize)]
struct OrderCount{
    order_count:i64,
}

///
///
/// curl -X GET \
///     -H "APCA-API-KEY-ID: key" \
///     -H "APCA-API-SECRET-KEY: secret" \
///     https://paper-api.alpaca.markets/v2/orders
///
/// Response:
///
/// {"id":"4d1e0470-875c-4a03-888c-ef40cc971741","client_order_id":"d886653b-2bbf-423b-a1dd-ebf3e334d214","created_at":"2023-03-06T21:29:41.010557404Z","updated_at":"2023-03-06T21:29:41.010557404Z","submitted_at":"2023-03-06T21:29:41.009706244Z","filled_at":null,"expired_at":null,"canceled_at":null,"failed_at":null,"replaced_at":null,"replaced_by":null,"replaces":null,"asset_id":"b0b6dd9d-8b9b-48a9-ba46-b9d54906e415","symbol":"AAPL","asset_class":"us_equity","notional":null,"qty":"1","filled_qty":"0","filled_avg_price":null,"order_class":"","order_type":"market","type":"market","side":"buy","time_in_force":"gtc","limit_price":null,"stop_price":null,"status":"accepted","extended_hours":false,"legs":null,"trail_percent":null,"trail_price":null,"hwm":null,"subtag":null,"source":null}
///
///
/// https://alpaca.markets/docs/api-references/trading-api/orders/
///
///
/// {
///   "id": "61e69015-8549-4bfd-b9c3-01e75843f47d",
///   "client_order_id": "eb9e2aaa-f71a-4f51-b5b4-52a6c565dad4",
///   "created_at": "2021-03-16T18:38:01.942282Z",
///   "updated_at": "2021-03-16T18:38:01.942282Z",
///   "submitted_at": "2021-03-16T18:38:01.937734Z",
///   "filled_at": null,
///   "expired_at": null,
///   "canceled_at": null,
///   "failed_at": null,
///   "replaced_at": null,
///   "replaced_by": null,
///   "replaces": null,
///   "asset_id": "b0b6dd9d-8b9b-48a9-ba46-b9d54906e415",
///   "symbol": "AAPL",
///   "asset_class": "us_equity",
///   "notional": "500",
///   "qty": null,
///   "filled_qty": "0",
///   "filled_avg_price": null,
///   "order_class": "",
///   "order_type": "market",
///   "type": "market",
///   "side": "buy",
///   "time_in_force": "day",
///   "limit_price": null,
///   "stop_price": null,
///   "status": "accepted",
///   "extended_hours": false,
///   "legs": null,
///   "trail_percent": null,
///   "trail_price": null,
///   "hwm": null
/// }
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    pub id: String,
    pub client_order_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub submitted_at: DateTime<Utc>,
    pub filled_at: Option<DateTime<Utc>>,
    pub expired_at: Option<DateTime<Utc>>,
    pub canceled_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub replaced_at: Option<DateTime<Utc>>,
    pub replaced_by: Option<DateTime<Utc>>,
    pub replaces: Option<String>,
    pub asset_id: Option<String>,
    pub symbol: String,
    pub asset_class: Option<String>,
    pub notional: Option<BigDecimal>,
    pub qty: BigDecimal,
    pub filled_qty: Option<BigDecimal>,
    pub filled_avg_price: Option<BigDecimal>,
    pub order_class: Option<String>,
    // deprecated (so they could use "type" screw up every programming language reserved keyword SERDE)
    // order_type: String,
    #[serde(rename = "type")]
    pub order_type_v2: OrderType,
    pub side: TradeSide,
    pub time_in_force: TimeInForce,
    pub limit_price: Option<BigDecimal>,
    pub stop_price: Option<BigDecimal>,
    pub status: String,
    pub extended_hours: bool,
    // pub legs: Option<String>,
    pub trail_percent: Option<BigDecimal>,
    pub trail_price: Option<BigDecimal>,
    pub hwm: Option<BigDecimal>,

}

impl Order {

    /// Get all outstanding orders from Alpaca API
    pub async fn remote(settings: &Settings) -> Result<Vec<Order>, TradeWebError> {

        let mut headers = HeaderMap::new();
        let api_key = settings.alpaca_paper_id.clone();
        let api_secret = settings.alpaca_paper_secret.clone();
        headers.insert("APCA-API-KEY-ID", api_key.parse().unwrap());
        headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());
        let client = reqwest::Client::new();

        let http_result = client.get("https://paper-api.alpaca.markets/v2/orders")
            .headers(headers)
            .send()
            .await;

        // parse_http_result_to_vec::<Order>(http_result).await

        let return_val = match http_result {
            Ok(response) => {
                match &response.text().await{
                    Ok(response_text)=>{
                        match serde_json::from_str::<Vec<Order>>(&response_text){
                            Ok(orders)=> {
                                Ok(orders)
                            },
                            Err(e)=>{
                                tracing::debug!("[get_remote] deserialization to json vec failed: {:?}", &e);
                                Err(TradeWebError::JsonError)
                            }
                        }
                    },
                    Err(e)=>{
                        tracing::debug!("[get_remote] deserialization to json text failed: {:?}", &e);
                        Err(TradeWebError::JsonError)
                    }
                }
            },
            Err(e) => {
                tracing::debug!("[get_remote] reqwest error: {:?}", &e);
                Err(TradeWebError::ReqwestError)
            }
        };

        return_val

    }

    /// Get the most recent list of positions from the database
    ///
    /// TODO: make these simple inserts non-blocking and non-async
    pub fn local(tx_db:Sender<DbMsg>) -> Result<Vec<Order>, TradeWebError> {
        let (tx, rx) = crossbeam_channel::unbounded();
        tx_db.send(DbMsg::OrderLocal { sender_tx: tx }).unwrap();
        match rx.recv(){
            Ok(result)=>Ok(result),
            Err(_e) => Err(TradeWebError::ChannelError),
        }
    }

    /// Save a single order to the database
    pub fn save(&self, tx_db:Sender<DbMsg>) {
        let _ = tx_db.send(DbMsg::OrderSave{ order:self.clone() });
    }

    pub async fn clear(pool: &PgPool)->Result<PgQueryResult,sqlx::Error> {
        sqlx::query!(r#"delete from alpaca_order"#).execute(pool).await
    }

    pub async fn exists(symbol:&str, trade_side:TradeSide, pool:&PgPool) -> Result<u64, Error> {
        match sqlx::query_as!(OrderCount,
            r#"
                select count(*) as "order_count!:i64"
                from alpaca_order where upper(symbol)=upper($1) and side=$2
            "#,
            symbol,
            &trade_side.to_string()
        ).fetch_one(pool).await {
            Ok(oc) => {
                // return an unsigned int regardless of what the designers of Postgres think is best
                if oc.order_count <= 0 {
                    Ok(0)
                } else {
                    Ok(oc.order_count as u64)
                }

            },
            Err(e) => Err(e),
        }
    }
}

impl fmt::Display for Order {

    /// enable to_string() for Order
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        //fmt::Debug::fmt(self, f)
        match self.limit_price.as_ref(){
            Some(limit_p) => {
                write!(f, "{}\t{}\t{}\t{}\t{}\t{:?}\t{}\t", self.symbol, self.side, self.qty, self.order_type_v2, limit_p, self.filled_at, self.id)
            },
            None=>{
                write!(f, "{}\t{}\t{}\t{}\t{}\t{:?}\t{}\t", self.symbol, self.side, self.qty, self.order_type_v2, 0.0, self.filled_at, self.id)
            }
        }
    }
}







