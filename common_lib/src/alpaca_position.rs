//! alpaca_position
//!
//!
//!

use crate::settings::Settings;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgQueryResult;
use sqlx::PgPool;
use std::fmt;

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
///   "asset_id": "904837e3-3b76-47ec-b432-046db621571b",
///   "symbol": "AAPL ",
///   "exchange": "NASDAQ",
///   "asset_class": "us_equity",
///   "avg_entry_price": "100.0",
///   "qty": "5",
///   "qty_available": "4",
///   "side": "long",
///   "market_value": "600.0",
///   "cost_basis": "500.0",
///   "unrealized_pl": "100.0",
///   "unrealized_plpc": "0.20",
///   "unrealized_intraday_pl": "10.0",
///   "unrealized_intraday_plpc": "0.0084",
///   "current_price": "120.0",
///   "lastday_price": "119.0",
///   "change_today": "0.0084"
/// }
///
///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Position {
    pub dtg: DateTime<Utc>,
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
    pub dtg_updated: DateTime<Utc>,
}

use chrono::{DateTime, Utc};
use std::fmt::{Debug, Display};

/// From the web API deserialize to this then convert to Position
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TempPosition {
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

impl Position {
    fn from_temp(dt_updated_now: DateTime<Utc>, t: TempPosition) -> Position {
        Position {
            // TODO: alpaca doesn't provide a timestamp of when the position started; it needs to be gleaned
            // from the Activity API; but it'd be useful to be able to know how old a position is
            dtg: dt_updated_now,
            asset_id: t.asset_id,
            symbol: t.symbol,
            exchange: t.exchange,
            asset_class: t.asset_class,
            avg_entry_price: t.avg_entry_price,
            qty: t.qty,
            qty_available: t.qty_available,
            side: t.side,
            market_value: t.market_value,
            cost_basis: t.cost_basis,
            unrealized_pl: t.unrealized_pl,
            unrealized_plpc: t.unrealized_plpc,
            unrealized_intraday_pl: t.unrealized_intraday_pl,
            unrealized_intraday_plpc: t.unrealized_intraday_plpc,
            current_price: t.current_price,
            lastday_price: t.lastday_price,
            change_today: t.change_today,
            dtg_updated: dt_updated_now
        }
    }

    /// Get the most recent list of positions from the database
    pub async fn get_open_positions_from_db(pool: &PgPool) -> Result<Vec<Position>, sqlx::Error> {
        // Assume the latest batch was inserted at the same time; get the most recent timestamp, get the most recent matching positions
        // https://docs.rs/sqlx/0.4.2/sqlx/macro.query.html#type-overrides-bind-parameters-postgres-only
        let result_vec = sqlx::query_as!(
            Position,
            r#"
                select
                    dtg as "dtg!"
                    ,symbol as "symbol!"
                    ,exchange as "exchange!"
                    ,asset_class as "asset_class!"
                    ,coalesce(avg_entry_price, 0.0) as "avg_entry_price!"
                    ,coalesce(qty,0.0) as "qty!"
                    ,coalesce(qty_available,0.0) as "qty_available!"
                    ,side as "side!:PositionSide"
                    ,coalesce(market_value,0.0) as "market_value!"
                    ,coalesce(cost_basis,0.0) as "cost_basis!"
                    ,coalesce(unrealized_pl,0.0) as "unrealized_pl!"
                    ,coalesce(unrealized_plpc,0.0) as "unrealized_plpc!"
                    ,coalesce(unrealized_intraday_pl,0.0) as "unrealized_intraday_pl!"
                    ,coalesce(unrealized_intraday_plpc,0.0) as "unrealized_intraday_plpc!"
                    ,coalesce(current_price,0.0) as "current_price!"
                    ,coalesce(lastday_price,0.0) as "lastday_price!"
                    ,coalesce(change_today,0.0) as "change_today!"
                    ,asset_id as "asset_id!"
                    ,dtg_updated as "dtg_updated!"
                from
                alpaca_position
                order by symbol
            "#
        )
        .fetch_all(pool)
        .await;

        result_vec
    }

    // Call the Alpaca API to get the remote position snapshot
    pub async fn get_remote(settings: &Settings) -> Result<Vec<Position>, reqwest::Error> {
        let mut headers = reqwest::header::HeaderMap::new();
        let api_key_id = settings.alpaca_paper_id.clone();
        let api_secret = settings.alpaca_paper_secret.clone();
        headers.insert("APCA-API-KEY-ID", api_key_id.parse().unwrap());
        headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());

        let client = reqwest::Client::new();
        let remote_positions: Vec<TempPosition> = client
            .get("https://paper-api.alpaca.markets/v2/positions")
            .headers(headers)
            .send()
            .await?
            .json()
            .await?;

        let now = Utc::now();
        let remote_positions = remote_positions
            .iter()
            .map(move |x| Position::from_temp(now, x.clone()))
            .collect();

        Ok(remote_positions)
    }

    /// delete_all_db
    pub async fn delete_all_db(pool: &PgPool) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query!(r#"delete from alpaca_position"#)
            .execute(pool)
            .await
    }

    /// save a single position to the database; not ideal to not insert the result of the alpaca api call as a bulk insert but not rocket science at the moment
    pub async fn save_to_db(
        &self,
        timestamp: DateTime<Utc>,
        pool: &PgPool,
    ) -> Result<PgQueryResult, sqlx::Error> {
        // tracing::debug!("[save_to_db]");
        let result = sqlx::query!(
            r#"
                insert into alpaca_position(dtg, symbol, exchange, asset_class, avg_entry_price, qty, qty_available, side, market_value
                    , cost_basis, unrealized_pl, unrealized_plpc, current_price, lastday_price, change_today, asset_id)
                values
                    (
                        $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11, $12, $13, $14, $15, $16
                    )"#,
            timestamp,
            self.symbol, self.exchange, self.asset_class, self.avg_entry_price, self.qty, self.qty_available,
            self.side.to_string(), self.market_value, self.cost_basis, self.unrealized_pl, self.unrealized_plpc, self.current_price,
            self.lastday_price, self.change_today, self.asset_id
        ).execute(pool).await;

        // tracing::debug!("[activity::save_to_db] insert result: {:?}", &result);
        result
    }
}

#[derive(sqlx::Type, Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum PositionSide {
    #[serde(rename = "long")]
    Long,
    #[serde(rename = "short")]
    Short,
}

impl Display for PositionSide {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

impl Display for Position {
    /// enable to_string()
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}\t{:?}\t{}\t{}\t{}",
            self.symbol, self.side, self.qty, self.cost_basis, self.asset_id
        )
    }
}

