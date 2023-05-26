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

use std::fmt;
use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::postgres::PgQueryResult;
use crate::settings::Settings;
use crate::trade_struct::TradeSide;



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
pub struct Activity{
    pub id: String,
    pub activity_type: ActivityType,
    // fill or partial_fill
    #[serde(rename="type")]
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

    pub async fn save_to_db(&self, pool: &PgPool)-> Result<PgQueryResult, sqlx::Error> {
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
                    ,$6
                    ,$7
                    ,$8
                    ,$9
                    ,$10
                    ,$11
                    )"#,
            self.id, self.activity_type.to_string(), self.activity_subtype.to_string(), self.transaction_time,
            self.symbol, self.side.to_string(), self.qty, self.price, self.cum_qty, self.leaves_qty, self.order_id
        ).execute(pool).await;

        tracing::debug!("[activity::save_to_db] insert result: {:?}", &result);
        result
    }

    // /// Vec<Activity> for a specific stock symbol
    // pub async fn get_activity_for_symbol(symbol:&str, pool:&PgPool)->Result<Vec<Activity>,sqlx::Error> {
    //
    //     let result_vec = sqlx::query_as!(
    //         Activity,
    //         r#"
    //             select
    //                 id as "id!"
    //                 , activity_type as "activity_type!:ActivityType"
    //                 , activity_subtype as "activity_subtype!:ActivitySubtype"
    //                 , transaction_time as "transaction_time!"
    //                 , symbol as "symbol!"
    //                 , side as "side!:TradeSide"
    //                 , qty as "qty!"
    //                 , price as "price!"
    //                 , cum_qty as "cum_qty!"
    //                 , leaves_qty as "leaves_qty!"
    //                 , order_id as "order_id!"
    //             from alpaca_activity
    //             where symbol = upper($1)
    //         "#,
    //         &symbol
    //     ).fetch_all(pool).await;
    //
    //     result_vec
    //
    // }

    /// get a vec of alpaca trading activities from the postgres database (as a reflection of what's been
    /// synced from the Alpaca API)
    ///
    /// TODO: probably already have a common function for this
    ///
    pub async fn get_activities_from_db(pool:&PgPool) -> Result<Vec<ActivityQuery>,sqlx::Error>{

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

    pub async fn get_activities_from_db_for_symbol(symbol:&str, pool:&PgPool) -> Result<Vec<ActivityQuery>,sqlx::Error>{

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

    /// Get FILL activities
    ///
    /// https://alpaca.markets/docs/api-references/trading-api/account-activities/#properties
    ///
    pub async fn get_remote_activities(pool: PgPool, settings:Settings) ->Result<(), reqwest::Error> {
        // 1. database call to get most recent activity
        // 2. web api call to get all activities since most recent stored locally

        let mut headers = HeaderMap::new();
        // let api_key_id = std::env::var("ALPACA_API_ID").expect("ALPACA_API_ID environment variable not found");
        // let api_secret = std::env::var("ALPACA_API_SECRET").expect("alpaca_secret environment variable not found");
        let api_key = settings.alpaca_paper_id.clone();
        let api_secret = settings.alpaca_paper_secret.clone();
        headers.insert("APCA-API-KEY-ID", api_key.parse().unwrap());
        headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());
        let url = format!("https://paper-api.alpaca.markets/v2/account/activities/FILL");

        tracing::debug!("[load_fill_activities] calling API: {}", &url);

        // get a single order
        let client = reqwest::Client::new();

        let http_result = client.get(url)
            .headers(headers)
            .send()
            .await;

        match http_result {
            Ok(resp) => {

                // i want to see what's in there dangit. json() gives away ownership and I can't get it back.
                let json_text = &resp.text().await.unwrap();
                tracing::debug!("json: {}", &json_text);

                match serde_json::from_str::<Vec<Activity>>(&json_text) {
                    Ok(activities) => {
                        tracing::debug!("[load_fill_activities] activity json: {:?}", &activities);
                        for a in activities {
                            tracing::debug!("[load_fill_activities] activity: {:?}", &a);

                            // 3. merge remote results to local database
                            let _result = a.save_to_db(&pool).await;
                        }
                    },
                    Err(e) => {
                        tracing::debug!("[load_fill_activities] json: {}", &json_text);
                        tracing::debug!("[load_fill_activities] json error: {:?}", &e);
                    }
                }
            },
            Err(e) => {
                tracing::debug!("[load_fill_activities] reqwest error: {:?}", &e);
            }
        }

        Ok(())

    }
}

/// https://alpaca.markets/docs/api-references/trading-api/account-activities/#properties
#[derive(sqlx::Type, Deserialize, Serialize, Debug)]
pub enum ActivityType{
    #[serde(rename="FILL")]
    Fill
}

impl fmt::Display for ActivityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// https://alpaca.markets/docs/api-references/trading-api/account-activities/#properties
#[derive(sqlx::Type, Deserialize, Serialize, Debug)]
pub enum ActivitySubtype{
    #[serde(rename="fill")]
    Fill,
    #[serde(rename="partial_fill")]
    PartialFill
}
impl fmt::Display for ActivitySubtype {

    /// enable to_string()
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

// #[derive(Deserialize, Serialize, Debug)]
// pub enum ActivityType{
//     #[serde(rename="FILL")]
//     Fill
// }
// #[derive(Deserialize, Serialize, Debug)]
// pub enum ActivitySubtype{
//     #[serde(rename="fill")]
//     Fill,
//     #[serde(rename="partial_fill")]
//     PartialFill
// }

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityQuery{
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


