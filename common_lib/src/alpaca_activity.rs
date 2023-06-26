//! alpaca_activity.rs
//!
//! Get events via the activities API
//!
//! https://alpaca.markets/docs/api-references/trading-api/account-activities/
//!
//! curl -X GET \
//!     -H "APCA-API-KEY-ID: xxxx" \
//!     -H "APCA-API-SECRET-KEY: xxxx"\
//!     https://paper-api.alpaca.markets/v2/account/activities/FILL?date='2023-03-24'
//!

use crate::error::TradeWebError;
use crate::settings::Settings;
use crate::trade_struct::TradeSide;
use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgQueryResult;
use sqlx::PgPool;
use std::fmt;

/// load all the most recent activities
/// 1. get the most recent activity in the database
/// 2. get everything since
///
/// TODO: move this to the backend web service and run continuously (needs to be idempotent, no duplicates)
///
/// {
///   "activity_type": "FILL",
///   "cum_qty": "1",
///   "id": "20190524113406977::8efc7b9a-8b2b-4000-9955-d36e7db0df74",
///   "leaves_qty": "0",
///   "price": "1.63",
///   "qty": "1",
///   "side": "buy",
///   "symbol": "LPCN",
///   "transaction_time": "2019-05-24T15:34:06.977Z",
///   "order_id": "904837e3-3b76-47ec-b432-046db621571b",
///   "type": "fill"
/// }
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub activity_type: ActivityType,
    // fill or partial_fill
    #[serde(rename = "type")]
    pub activity_subtype: ActivitySubtype,
    pub transaction_time: DateTime<Utc>,
    pub symbol: String,
    pub side: TradeSide,
    pub qty: BigDecimal,
    pub price: BigDecimal,
    pub cum_qty: BigDecimal,
    pub leaves_qty: BigDecimal,
    pub order_id: String,
}

impl Activity {
    /// Get FILL activities
    ///
    /// https://alpaca.markets/docs/api-references/trading-api/account-activities/#properties
    ///
    pub async fn get_remote(settings: &Settings) -> Result<Vec<Activity>, TradeWebError> {
        let mut headers = HeaderMap::new();
        let api_key = settings.alpaca_paper_id.clone();
        let api_secret = settings.alpaca_paper_secret.clone();
        headers.insert("APCA-API-KEY-ID", api_key.parse().unwrap());
        headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());

        let url = format!("https://paper-api.alpaca.markets/v2/account/activities/FILL");

        tracing::debug!("[get_remote] calling API: {}", &url);
        let client = reqwest::Client::new();

        let http_result = client.get(url).headers(headers).send().await;

        // let result = parse_http_result_to_vec::<Activity>(http_result).await;
        // result

        let return_val = match http_result {
            Ok(response) => match &response.text().await {
                Ok(response_text) => match serde_json::from_str::<Vec<Activity>>(&response_text) {
                    Ok(results) => Ok(results),
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

    pub async fn save_to_db(&self, pool: &PgPool) -> Result<PgQueryResult, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            insert into alpaca_activity
                (
                id
                , activity_type
                , activity_subtype
                , transaction_time
                , symbol
                , side
                , qty
                , price
                , cum_qty
                , leaves_qty
                , order_id
                )
                values (
                    $1
                    ,$2
                    ,$3
                    ,$4
                    ,$5
                    ,lower($6)
                    ,$7
                    ,$8
                    ,$9
                    ,$10
                    ,$11
                    )"#,
            self.id,
            self.activity_type.to_string(),
            self.activity_subtype.to_string(),
            self.transaction_time,
            self.symbol,
            self.side.to_string(),
            self.qty,
            self.price,
            self.cum_qty,
            self.leaves_qty,
            self.order_id
        )
        .execute(pool)
        .await;

        // tracing::debug!("[get_remote] insert result: {:?}", &result);
        result
    }

    /// get a vec of alpaca trading activities from the postgres database (as a reflection of what's been
    /// synced from the Alpaca API)
    pub async fn get_activities_from_db(pool: &PgPool) -> Result<Vec<ActivityQuery>, sqlx::Error> {
        // https://docs.rs/sqlx/0.4.2/sqlx/macro.query.html#type-overrides-bind-parameters-postgres-only
        sqlx::query_as!(
            ActivityQuery,
            r#"
                select
                    transaction_time::timestamp as "dtg_utc!"
                    ,timezone('US/Pacific', transaction_time) as "dtg_pacific!"
                    ,symbol as "symbol!"
                    ,side as "side!:TradeSide"
                    ,qty as "qty!"
                    ,price as "price!"
                from alpaca_activity
                order by transaction_time desc
            "#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn get_activities_from_db_for_symbol(
        symbol: &str,
        pool: &PgPool,
    ) -> Result<Vec<ActivityQuery>, sqlx::Error> {
        // https://docs.rs/sqlx/0.4.2/sqlx/macro.query.html#type-overrides-bind-parameters-postgres-only

        sqlx::query_as!(
            ActivityQuery,
            r#"
                select
                    transaction_time::timestamp as "dtg_utc!"
                    ,timezone('US/Pacific', transaction_time) as "dtg_pacific!"
                    ,symbol as "symbol!"
                    ,side as "side!:TradeSide"
                    ,qty as "qty!"
                    ,price as "price!"
                from alpaca_activity
                where symbol = upper($1)
                order by transaction_time desc
            "#,
            symbol
        )
        .fetch_all(pool)
        .await
    }
}

/// https://alpaca.markets/docs/api-references/trading-api/account-activities/#properties
#[derive(sqlx::Type, Deserialize, Serialize, Debug)]
pub enum ActivityType {
    #[serde(rename = "FILL")]
    Fill,
}

impl fmt::Display for ActivityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// https://alpaca.markets/docs/api-references/trading-api/account-activities/#properties
#[derive(sqlx::Type, Deserialize, Serialize, Debug)]
pub enum ActivitySubtype {
    #[serde(rename = "fill")]
    Fill,
    #[serde(rename = "partial_fill")]
    PartialFill,
}
impl fmt::Display for ActivitySubtype {
    /// enable to_string()
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityQuery {
    // pub id: String,
    // pub activity_type: ActivityType,
    // fill or partial_fill
    // #[serde(rename="type")]
    // pub activity_subtype: ActivitySubtype,
    pub dtg_utc: NaiveDateTime,
    pub dtg_pacific: NaiveDateTime,
    pub symbol: String,
    pub side: TradeSide,
    pub qty: BigDecimal,
    pub price: BigDecimal,
    // pub cum_qty: BigDecimal,
    // pub leaves_qty: BigDecimal,
    // pub order_id: String,
}
