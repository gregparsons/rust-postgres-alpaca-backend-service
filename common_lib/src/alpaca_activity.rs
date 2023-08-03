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




use std::fmt;
use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDateTime, Utc};
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use crate::db::DbMsg;
use crate::error::TradeWebError;
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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Activity {
    pub id: String,
    pub activity_type: ActivityType,
    // fill or partial_fill
    #[serde(rename = "type")]
    pub activity_subtype: ActivitySubtype,
    #[serde(rename="transaction_time")]
    pub dtg: DateTime<Utc>,
    pub symbol: String,
    pub side: TradeSide,
    pub qty: BigDecimal,
    pub price: BigDecimal,
    pub cum_qty: BigDecimal,
    pub leaves_qty: BigDecimal,
    pub order_id: String,
}

#[derive(Deserialize)]
pub struct ActivityLatest{
    pub dtg:DateTime<Utc>
}

impl Activity {

    /// latest_dtg: get the date of the most recent activity; used to filter the activity API
    pub fn latest_dtg(tx_db:crossbeam_channel::Sender<DbMsg>) -> Result<DateTime<Utc>, TradeWebError> {
        let (resp_tx, resp_rx) = crossbeam_channel::unbounded();
        let _ = tx_db.send(DbMsg::ActivityLatestDtg { resp_tx });
        match resp_rx.recv(){
            Ok(result)=>Ok(result),
            Err(_e)=>Err(TradeWebError::ChannelError),
        }

    }

    /// Get FILL activities
    ///
    /// https://alpaca.markets/docs/api-references/trading-api/account-activities/#properties
    ///
    /// curl --request GET \
    ///      --url 'https://paper-api.alpaca.markets/v2/account/activities/FILL?after=2023-07-17T19%3A57%3A00.0Z' \
    ///     --header 'APCA-API-KEY-ID: ' \
    ///      --header 'APCA-API-SECRET-KEY: ' \
    ///      --header 'accept: application/json'
    ///
    pub fn get_remote(since_filter:Option<DateTime<Utc>>, settings: &Settings, tx_db:crossbeam_channel::Sender<DbMsg>)
        -> Result<Vec<Activity>, TradeWebError> {
        let (resp_tx, resp_rx) = crossbeam_channel::unbounded();
        let _ = tx_db.send(DbMsg::ActivityGetRemote { since_option:since_filter, settings:settings.clone(), resp_tx});
        match resp_rx.recv(){
            Ok(result)=>Ok(result),
            Err(_e)=>Err(TradeWebError::ChannelError),
        }

    }

    pub fn save_to_db(&self, tx_db:crossbeam_channel::Sender<DbMsg>) {
        let _ = tx_db.send(DbMsg::ActivitySaveToDb { activity: (*self).clone() });
    }

    /// get a vec of alpaca trading activities from the postgres database (as a reflection of what's been
    /// synced from the Alpaca API)
    pub async fn get_activities_from_db(tx_db:Sender<DbMsg>) -> Vec<ActivityQuery> {
        let (tx, rx) = oneshot::channel();
        match tx_db.send(DbMsg::ActivityGetAll{ sender:tx}){
            Ok(_)=>{
                match rx.await{
                    Ok(vec_activity)=> vec_activity,
                    Err(e)=>{
                        tracing::error!("[get_activities_from_db_for_symbol] recv error: {:?}", &e);
                        vec!()
                    }
                }
            },
            Err(e)=>{
                tracing::error!("[get_activities_from_db_for_symbol] send error: {:?}", &e);
                vec!()
            }
        }
    }

    pub async fn get_activities_from_db_for_symbol(symbol: &str, tx_db:Sender<DbMsg>) -> Vec<ActivityQuery> {
        let (tx, rx) = oneshot::channel();
        match tx_db.send(DbMsg::ActivityGetForSymbol{symbol:symbol.to_string(), sender:tx}){
            Ok(_)=>{
                match rx.await{
                    Ok(vec_activity)=> vec_activity,
                    Err(e)=>{
                        tracing::error!("[get_activities_from_db_for_symbol] recv error: {:?}", &e);
                        vec!()
                    }
                }
            },
            Err(e)=>{
                tracing::error!("[get_activities_from_db_for_symbol] send error: {:?}", &e);
                vec!()
            }

        }
    }

}

/// https://alpaca.markets/docs/api-references/trading-api/account-activities/#properties
#[derive(sqlx::Type, Deserialize, Serialize, Debug, Clone)]
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
#[derive(sqlx::Type, Deserialize, Serialize, Debug, Clone)]
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
    #[serde(rename="order_id")]
    pub client_order_id:String,
}
