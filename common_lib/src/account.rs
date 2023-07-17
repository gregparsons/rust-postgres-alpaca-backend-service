//! account.rs
//!
//! https://alpaca.markets/docs/api-references/trading-api/account/

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub cash: String,
    pub position_market_value: String,
    pub equity: String,
    pub last_equity: String,
    pub daytrade_count: usize,
    pub balance_asof: String,
    pub pattern_day_trader: bool,
    pub id: String,
    pub account_number: String,
    pub status: String,
    // crypto_status: String,
    pub currency: String,
    pub buying_power: String,
    pub regt_buying_power: String,
    pub daytrading_buying_power: String,
    pub effective_buying_power: String,
    pub non_marginable_buying_power: String,
    pub bod_dtbp: String,
    pub accrued_fees: String,
    pub pending_transfer_in: String,
    pub portfolio_value: String,    // deprecated (same as equity field)
    pub trading_blocked: bool,
    pub transfers_blocked: bool,
    pub account_blocked: bool,
    pub created_at: String,
    pub trade_suspended_by_user: bool,
    pub multiplier: String,
    pub shorting_enabled: bool,
    pub long_market_value: String,
    pub short_market_value: String,
    pub initial_margin: String,
    pub maintenance_margin: String,
    pub last_maintenance_margin: String,
    pub sma: String,
    // crypto_tier: usize,
}

impl Account {
    pub fn blank() -> Account {
        Account {
            cash: "".to_string(),
            position_market_value: "".to_string(),
            equity: "".to_string(),
            last_equity: "".to_string(),
            daytrade_count: 0,
            balance_asof: "".to_string(),
            pattern_day_trader: false,
            id: "".to_string(),
            account_number: "".to_string(),
            status: "".to_string(),
            currency: "".to_string(),
            buying_power: "".to_string(),
            regt_buying_power: "".to_string(),
            daytrading_buying_power: "".to_string(),
            effective_buying_power: "".to_string(),
            non_marginable_buying_power: "".to_string(),
            bod_dtbp: "".to_string(),
            accrued_fees: "".to_string(),
            pending_transfer_in: "".to_string(),
            portfolio_value: "".to_string(),
            trading_blocked: false,
            transfers_blocked: false,
            account_blocked: false,
            created_at: "".to_string(),
            trade_suspended_by_user: false,
            multiplier: "".to_string(),
            shorting_enabled: false,
            long_market_value: "".to_string(),
            short_market_value: "".to_string(),
            initial_margin: "".to_string(),
            maintenance_margin: "".to_string(),
            last_maintenance_margin: "".to_string(),
            sma: "".to_string(),
        }
    }
}
