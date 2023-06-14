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

//! alpaca_order
//!

use crate::error::TradeWebError;
use crate::settings::Settings;
use crate::trade_struct::{OrderType, TimeInForce, TradeSide};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgQueryResult;
use sqlx::PgPool;
use std::fmt;

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
    pub async fn get_remote(settings: &Settings) -> Result<Vec<Order>, TradeWebError> {
        let mut headers = HeaderMap::new();
        let api_key = settings.alpaca_paper_id.clone();
        let api_secret = settings.alpaca_paper_secret.clone();
        headers.insert("APCA-API-KEY-ID", api_key.parse().unwrap());
        headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());
        let client = reqwest::Client::new();

        let http_result = client
            .get("https://paper-api.alpaca.markets/v2/orders")
            .headers(headers)
            .send()
            .await;

        // parse_http_result_to_vec::<Order>(http_result).await

        let return_val = match http_result {
            Ok(response) => match &response.text().await {
                Ok(response_text) => match serde_json::from_str::<Vec<Order>>(&response_text) {
                    Ok(orders) => Ok(orders),
                    Err(e) => {
                        tracing::debug!(
                            "[get_remote] deserialization to json vec failed: {:?}",
                            &e
                        );
                        Err(TradeWebError::JsonError)
                    }
                },
                Err(e) => {
                    tracing::debug!("[get_remote] deserialization to json text failed: {:?}", &e);
                    Err(TradeWebError::JsonError)
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
    pub async fn get_unfilled_orders_from_db(pool: &PgPool) -> Result<Vec<Order>, sqlx::Error> {
        // Assume the latest batch was inserted at the same time; get the most recent timestamp, get the most recent matching positions
        // https://docs.rs/sqlx/0.4.2/sqlx/macro.query.html#type-overrides-bind-parameters-postgres-only
        let result_vec = sqlx::query_as!(
            Order,
            r#"
                select
                    id as "id!"
                    , client_order_id as "client_order_id!"
                    , created_at as "created_at!"
                    , updated_at as "updated_at!"
                    , submitted_at as "submitted_at!"
                    , filled_at
                    , expired_at
                    , canceled_at
                    , failed_at
                    , replaced_at
                    , replaced_by
                    , replaces
                    , asset_id as "asset_id!:Option<String>"
                    , symbol as "symbol!:String"
                    , asset_class as "asset_class!:Option<String>"
                    , notional as "notional!:Option<BigDecimal>"
                    , coalesce(qty, 0.0) as "qty!:BigDecimal"
                    , filled_qty as "filled_qty!:Option<BigDecimal>"
                    , filled_avg_price as "filled_avg_price!:Option<BigDecimal>"
                    , order_class as "order_class!:Option<String>"
                    , order_type_v2 as "order_type_v2!:OrderType"
                    , side as "side!:TradeSide"
                    , time_in_force as "time_in_force!:TimeInForce"
                    , limit_price as "limit_price!:Option<BigDecimal>"
                    , stop_price as "stop_price!:Option<BigDecimal>"
                    , status as "status!"
                    , coalesce(extended_hours, false) as "extended_hours!"
                    , trail_percent
                    , trail_price
                    , hwm
                from alpaca_order
                where filled_at is null
                order by updated_at desc
            "#
        )
        .fetch_all(pool)
        .await;

        result_vec
    }

    /// Save a single order to the database
    pub async fn save_to_db(&self, pool: &sqlx::PgPool) {
        /*

            [{"id":"2412874c-45a4-4e47-b0eb-98c00c1f05eb","client_order_id":"b6f91215-4e78-400d-b2ac-1bb546f86237","created_at":"2023-03-17T06:02:42.552044Z","updated_at":"2023-03-17T06:02:42.552044Z","submitted_at":"2023-03-17T06:02:42.551444Z","filled_at":null,"expired_at":null,"canceled_at":null,"failed_at":null,"replaced_at":null,"replaced_by":null,"replaces":null,"asset_id":"8ccae427-5dd0-45b3-b5fe-7ba5e422c766","symbol":"TSLA","asset_class":"us_equity","notional":null,"qty":"1","filled_qty":"0","filled_avg_price":null,"order_class":"","order_type":"market","type":"market","side":"buy","time_in_force":"day","limit_price":null,"stop_price":null,"status":"accepted","extended_hours":false,"legs":null,"trail_percent":null,"trail_price":null,"hwm":null,"subtag":null,"source":null}]

        */

        let result = sqlx::query!(
            r#"insert into alpaca_order(
                id,
                client_order_id,
                created_at,
                updated_at,
                submitted_at,
                filled_at,

                -- expired_at,
                -- canceled_at,
                -- failed_at,
                -- replaced_at,
                -- replaced_by,
                -- replaces,
                -- asset_id,

                symbol,
                -- asset_class,
                -- notional,
                qty,
                filled_qty,
                filled_avg_price,
                -- order_class,
                order_type_v2,
                side,
                time_in_force,
                limit_price,
                stop_price,
                status
                -- extended_hours,
                -- trail_percent,
                -- trail_price,
                -- hwm
                )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, lower($11), lower($12), lower($13), $14, $15, $16)

            "#,
            self.id, self.client_order_id, self.created_at, self.updated_at, self.submitted_at,
            self.filled_at, // $7
            self.symbol,
            self.qty, self.filled_qty, self.filled_avg_price,
            self.order_type_v2.to_string(), // $11
            self.side.to_string(),          // $12
            self.time_in_force.to_string(), // $13
            self.limit_price,
            self.stop_price,
            self.status
        ).execute(pool).await;
        tracing::debug!("[save_to_db] result: {:?}", result);
    }

    pub async fn delete_all_db(pool: &PgPool) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query!(r#"delete from alpaca_order"#)
            .execute(pool)
            .await
    }
}

impl fmt::Display for Order {
    /// enable to_string() for Order
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        //fmt::Debug::fmt(self, f)
        match self.limit_price.as_ref() {
            Some(limit_p) => {
                write!(
                    f,
                    "{}\t{}\t{}\t{}\t{}\t{:?}\t{}\t",
                    self.symbol,
                    self.side,
                    self.qty,
                    self.order_type_v2,
                    limit_p,
                    self.filled_at,
                    self.id
                )
            }
            None => {
                write!(
                    f,
                    "{}\t{}\t{}\t{}\t{}\t{:?}\t{}\t",
                    self.symbol,
                    self.side,
                    self.qty,
                    self.order_type_v2,
                    0.0,
                    self.filled_at,
                    self.id
                )
            }
        }
    }
}
