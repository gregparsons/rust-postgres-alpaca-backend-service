//! settings.rs
//!
//! model for settings store in postgres db

use std::sync::mpsc::Sender;
use crate::trade_setting_profile::TradeSettingsProfile;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Settings {
    pub dtg: DateTime<Utc>,
    pub alpaca_paper_id: String,
    pub alpaca_paper_secret: String,
    pub alpaca_live_id: String,
    pub alpaca_live_secret: String,
    pub trade_size: BigDecimal,
    pub trade_enable_buy: bool,
    pub trade_ema_small_size: i32,
    pub trade_ema_large_size: i32,
    pub trade_sell_high_per_cent_multiplier: BigDecimal,
    pub trade_sell_high_upper_limit_cents: BigDecimal,
    pub finnhub_key: String,
    pub account_start_value:BigDecimal,
    pub max_position_age_minute:BigDecimal,
    pub upgrade_min_profit:BigDecimal,
    pub upgrade_sell_elapsed_minutes_min:BigDecimal,
    pub upgrade_posn_max_elapsed_minutes:BigDecimal,
    pub upgrade_posn_loss_allowed_dollars:BigDecimal,
    pub acct_max_position_market_value: BigDecimal,
    pub acct_min_cash_dollars: BigDecimal,

}

impl Settings {
    ///
    /// TODO: encrypt alpaca credentials in database and decrypt here using .env
    ///
    pub async fn load_with_secret(tx_db: Sender<DbMsg>) -> Result<Settings, sqlx::Error> {

        tx_db.send(DbMsg::GetSettingsWithSecret).unwrap();


        settings_result
    }

    /// return blank secret for front-end type uses
    pub async fn load_no_secret(pool: &PgPool) -> Result<Settings, sqlx::Error> {
        let settings_result = sqlx::query_as!(
            Settings,
            r#"
                SELECT
                    dtg as "dtg!",
                    alpaca_paper_id as "alpaca_paper_id!:String",
                    '' as "alpaca_paper_secret!:String",
                    alpaca_live_id as "alpaca_live_id!:String",
                    '' as "alpaca_live_secret!:String",
                    trade_size as "trade_size!",
                    trade_enable_buy as "trade_enable_buy!",
                    trade_ema_small_size as "trade_ema_small_size!",
                    trade_ema_large_size as "trade_ema_large_size!",
                    trade_sell_high_per_cent_multiplier as "trade_sell_high_per_cent_multiplier!",
                    trade_sell_high_upper_limit_cents as "trade_sell_high_upper_limit_cents!"
                    ,finnhub_key as "finnhub_key!:String"
                    ,coalesce(account_start_value,0.0) as "account_start_value!"
                    ,coalesce(max_position_age_minute,0.0) as "max_position_age_minute!"
                    ,coalesce(upgrade_min_profit,0.0) as "upgrade_min_profit!"
                    ,coalesce(upgrade_sell_elapsed_minutes_min,60.0) as "upgrade_sell_elapsed_minutes_min!"
                    ,coalesce(upgrade_posn_max_elapsed_minutes,60.0) as "upgrade_posn_max_elapsed_minutes!"
                    ,coalesce(upgrade_posn_loss_allowed_dollars,10.0) as "upgrade_posn_loss_allowed_dollars!"
                    ,coalesce(acct_max_position_market_value,10.0) as "acct_max_position_market_value!"
                    ,coalesce(acct_min_cash_dollars,10.0) as "acct_min_cash_dollars!"
                FROM t_settings
                ORDER BY t_settings.dtg DESC
                LIMIT 1
            "#
        )
        .fetch_one(pool)
        .await;
        settings_result
    }

    /// change the settings and return blank secret for front-end type uses
    pub async fn change_trade_profile(trade_settings_profile: &TradeSettingsProfile, pool: &PgPool, ) -> Result<Settings, sqlx::Error> {
        let ts = trade_settings_profile.clone();
        let ts = ts.to_string(); // .as_str();

        // SQL injection is avoided here by using an enum; failure upon parsing would've happened at the api level
        let settings_result = sqlx::query_as!(
            Settings,
            r#"
                select
                    dtg as "dtg!"
                    , alpaca_paper_id as "alpaca_paper_id!"
                    , '' as "alpaca_paper_secret!"
                    , alpaca_live_id as "alpaca_live_id!"
                    , '' as "alpaca_live_secret!"
                    , trade_size as "trade_size!"
                    , trade_enable_buy as "trade_enable_buy!"
                    , trade_ema_small_size as "trade_ema_small_size!"
                    , trade_ema_large_size as "trade_ema_large_size!"
                    , trade_sell_high_per_cent_multiplier as "trade_sell_high_per_cent_multiplier!"
                    , trade_sell_high_upper_limit_cents as "trade_sell_high_upper_limit_cents!"
                    , finnhub_key as "finnhub_key!"
                    ,coalesce(account_start_value,0.0) as "account_start_value!"
                    ,coalesce(max_position_age_minute,0.0) as "max_position_age_minute!"
                    ,coalesce(upgrade_min_profit,0.0) as "upgrade_min_profit!"
                    ,coalesce(upgrade_sell_elapsed_minutes_min,60.0) as "upgrade_sell_elapsed_minutes_min!"
                    ,coalesce(upgrade_posn_max_elapsed_minutes,60.0) as "upgrade_posn_max_elapsed_minutes!"
                    ,coalesce(upgrade_posn_loss_allowed_dollars,10.0) as "upgrade_posn_loss_allowed_dollars!"
                    ,coalesce(acct_max_position_market_value,60.0) as "acct_max_position_market_value!"
                    ,coalesce(acct_min_cash_dollars,60.0) as "acct_min_cash_dollars!"
                from fn_set_trade_settings($1);
            "#,
            &ts
        )
        .fetch_one(pool)
        .await;
        settings_result
    }
}
