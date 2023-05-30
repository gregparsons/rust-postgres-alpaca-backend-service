//! trade_struct.rs
use std::fmt;
use bigdecimal::BigDecimal;
use serde::{Serialize, Deserialize};
use strum::Display;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonTrade{
    pub(crate) symbol:String,
    pub(crate) side:TradeSide,
    pub(crate) time_in_force:TimeInForce,
    pub(crate) qty:usize,
    #[serde(rename = "type")]
    pub(crate) order_type:OrderType,
    pub(crate) limit_price: Option<BigDecimal>,
    pub(crate) extended_hours: Option<bool>,
}

/*

warning: use of deprecated function `sqlx::_rename`: `#[sqlx(rename = "...")]` is now `#[sqlx(type_name = "...")`

ref: https://github.com/cockroachdb/cockroach/issues/57411

 */

// https://docs.rs/sqlx/0.4.2/sqlx/macro.query.html#type-overrides-bind-parameters-postgres-only
#[derive(sqlx::Type, Debug, Serialize, Deserialize, Clone, PartialEq, Display)]
#[strum(serialize_all = "snake_case")]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")] // TODO: deprecated but works
pub enum TradeSide{
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
    #[serde(rename = "sell_short")]
    #[sqlx(rename="sell_short")]
    SellShort,
}

//
// impl fmt::Display for TradeSide {
//     /// enable to_string()
//     // fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//     //     fmt::Debug::fmt(self, f)
//     // }
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", format!("{:?}", self).to_lowercase())
//     }
// }

#[derive(sqlx::Type, Debug, Serialize, Deserialize, PartialEq, Clone, Display)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[strum(serialize_all = "snake_case")]
pub enum TimeInForce{
    #[serde(rename = "gtc")]
    Gtc,
    #[serde(rename = "day")]
    Day,
    // Immediate or Cancel
    #[serde(rename = "ioc")]
    Ioc,
}

// impl fmt::Display for TimeInForce {
//     // fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//     //     fmt::Debug::fmt(self, f)
//     // }
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", format!("{:?}", self).to_lowercase())
//     }
//
// }


#[derive(sqlx::Type, Debug, Serialize, Deserialize, PartialEq, Clone, Display)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[strum(serialize_all = "snake_case")]
pub enum OrderType{
    #[serde(rename = "market")]
    Market,
    #[serde(rename = "limit")]
    Limit,
}

// enable to_string(); print enum in lowercase
// impl fmt::Display for OrderType {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", format!("{:?}", self).to_lowercase())
//     }
// }
