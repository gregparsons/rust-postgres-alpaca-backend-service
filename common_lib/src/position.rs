//! position.rs
//!
//!
//!

use std::fmt;
use serde::{Serialize, Deserialize};
use bigdecimal::BigDecimal;
use crate::settings::Settings;

///
/// curl -X GET \
///     -H "APCA-API-KEY-ID: {id}" \
///     -H "APCA-API-SECRET-KEY: {secret}" \
///     https://paper-api.alpaca.markets/v2/positions
///
/// Result:
/// [{"asset_id":"b0b6dd9d-8b9b-48a9-ba46-b9d54906e415","symbol":"AAPL","exchange":"NASDAQ","asset_class":"us_equity","asset_marginable":false,"qty":"4","avg_entry_price":"149.2625","side":"long","market_value":"616.48","cost_basis":"597.05","unrealized_pl":"19.43","unrealized_plpc":"0.0325433380788879","unrealized_intraday_pl":"12.36","unrealized_intraday_plpc":"0.0204595113553599","current_price":"154.12","lastday_price":"151.03","change_today":"0.0204595113553599","qty_available":"4"}]
///
/// Reference:
/// https://alpaca.markets/docs/api-references/trading-api/positions/
///
/// {
    //   "asset_id": "904837e3-3b76-47ec-b432-046db621571b",
    //   "symbol": "AAPL ",
    //   "exchange": "NASDAQ",
    //   "asset_class": "us_equity",
    //   "avg_entry_price": "100.0",
    //   "qty": "5",
    //   "qty_available": "4",
    //   "side": "long",
    //   "market_value": "600.0",
    //   "cost_basis": "500.0",
    //   "unrealized_pl": "100.0",
    //   "unrealized_plpc": "0.20",
    //   "unrealized_intraday_pl": "10.0",
    //   "unrealized_intraday_plpc": "0.0084",
    //   "current_price": "120.0",
    //   "lastday_price": "119.0",
    //   "change_today": "0.0084"
    // }
///
///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Position {
    pub asset_id: String,
    pub symbol: String,
    pub exchange: String,
    pub asset_class: String,
    pub avg_entry_price: BigDecimal,
    pub qty: BigDecimal,
    pub qty_available: BigDecimal,
    pub side: PositionSide,
    pub market_value: BigDecimal,
    pub cost_basis: BigDecimal,
    pub unrealized_pl: BigDecimal,
    pub unrealized_plpc: BigDecimal,
    pub unrealized_intraday_pl: BigDecimal,
    pub unrealized_intraday_plpc: BigDecimal,
    pub current_price: BigDecimal,
    pub lastday_price: BigDecimal,
    pub change_today: BigDecimal,
}

impl Position{
    /// Call the Alpaca API to get the remote position snapshot
    ///
    /// populate the positions hashmap
    pub async fn get_remote(settings:&Settings) -> Result<Vec<Position>, reqwest::Error> {

        let mut headers = reqwest::header::HeaderMap::new();

        // TODO: add a setting for USE_PAPER_OR_LIVE
        let api_key_id = settings.alpaca_paper_id.clone();
        let api_secret = settings.alpaca_paper_secret.clone();

        headers.insert("APCA-API-KEY-ID", api_key_id.parse().unwrap());
        headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());

        // get the position list from the positions API
        let client = reqwest::Client::new();
        let remote_positions:Vec<Position> = client.get("https://paper-api.alpaca.markets/v2/positions")
            .headers(headers)
            .send()
            .await?
            .json()
            .await?;

        Ok(remote_positions)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum PositionSide{
    #[serde(rename = "long")]
    Long,
    #[serde(rename = "short")]
    Short

}

impl fmt::Display for Position {

    /// enable to_string()
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\t{:?}\t{}\t{}\t{}", self.symbol, self.side, self.qty, self.cost_basis, self.asset_id)
    }
}