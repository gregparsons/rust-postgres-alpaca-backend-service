//! account.rs
//!
//! https://alpaca.markets/docs/api-references/trading-api/account/

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool};
use crate::error::TradeWebError;
use crate::settings::Settings;

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub cash: BigDecimal,
    pub position_market_value: BigDecimal,
    pub equity: BigDecimal,
    pub last_equity: BigDecimal,
    pub daytrade_count: i32,
    pub balance_asof: String,
    pub pattern_day_trader: bool,
    pub id: String,
    pub account_number: String,
    pub status: String,
    // // crypto_status: String,
    pub currency: String,
    pub buying_power: BigDecimal,
    pub regt_buying_power: BigDecimal,
    pub daytrading_buying_power: BigDecimal,
    pub effective_buying_power: BigDecimal,
    pub non_marginable_buying_power: BigDecimal,
    pub bod_dtbp: String,
    pub accrued_fees: BigDecimal,
    pub pending_transfer_in: BigDecimal,
    pub portfolio_value: BigDecimal,    // deprecated (same as equity field)
    pub trading_blocked: bool,
    pub transfers_blocked: bool,
    pub account_blocked: bool,
    pub created_at: DateTime<Utc>,
    pub trade_suspended_by_user: bool,
    pub multiplier: BigDecimal,
    pub shorting_enabled: bool,
    pub long_market_value: BigDecimal,
    pub short_market_value: BigDecimal,
    pub initial_margin: BigDecimal,
    pub maintenance_margin: BigDecimal,
    pub last_maintenance_margin: BigDecimal,
    pub sma: String,
    // pub crypto_tier: i32,
}

impl Account {
    pub fn blank() -> Account {
        Account {
            cash: BigDecimal::from(0), // "".to_string(),
            position_market_value: BigDecimal::from(0), //"".to_string(),
            equity: BigDecimal::from(0), // "".to_string(),
            last_equity: BigDecimal::from(0), //"".to_string(),
            daytrade_count: 0,
            balance_asof: "".to_string(),
            pattern_day_trader: false,
            id: "".to_string(),
            account_number: "".to_string(),
            status: "".to_string(),
            currency: "".to_string(),
            buying_power: BigDecimal::from(0), //"".to_string(),
            regt_buying_power: BigDecimal::from(0), //"".to_string(),
            daytrading_buying_power: BigDecimal::from(0), //"".to_string(),
            effective_buying_power: BigDecimal::from(0), //"".to_string(),
            non_marginable_buying_power: BigDecimal::from(0), //"".to_string(),
            bod_dtbp: "".to_string(),
            accrued_fees: BigDecimal::from(0), //"".to_string(),
            pending_transfer_in: BigDecimal::from(0), //"".to_string(),
            portfolio_value: BigDecimal::from(0), //"".to_string(),
            trading_blocked: false,
            transfers_blocked: false,
            account_blocked: false,
            created_at: Utc::now(), //"".to_string(),
            trade_suspended_by_user: false,
            multiplier: BigDecimal::from(0), //"".to_string(),
            shorting_enabled: false,
            long_market_value: BigDecimal::from(0), //"".to_string(),
            short_market_value: BigDecimal::from(0), //"".to_string(),
            initial_margin: BigDecimal::from(0), //"".to_string(),
            maintenance_margin: BigDecimal::from(0), //"".to_string(),
            last_maintenance_margin: BigDecimal::from(0), //"".to_string(),
            sma: "".to_string(),
            // crypto_tier: "".to_string()

        }
    }

    /// latest from database
    pub async fn get(pool:&PgPool) -> Result<AccountWithDate, TradeWebError> {
        match sqlx::query_as!(AccountWithDate, r#"
            select
                dtg as "dtg!"
                ,cash as "cash!"
                ,position_market_value as "position_market_value!"
                ,equity as "equity!"
                ,last_equity as "last_equity!"
                ,daytrade_count as "daytrade_count!"
                ,balance_asof as "balance_asof!"
                ,pattern_day_trader as "pattern_day_trader!"
                ,id as "id!"
                ,account_number as "account_number!"
                ,status as "status!"
                -- crypto_status as "!"
                ,currency as "currency!"
                            ,buying_power as "buying_power!"
                ,regt_buying_power as "regt_buying_power!"
                ,daytrading_buying_power as "daytrading_buying_power!"
                ,effective_buying_power as "effective_buying_power!"
                ,non_marginable_buying_power as "non_marginable_buying_power!"

                   ,bod_dtbp as "bod_dtbp!"
                ,accrued_fees as "accrued_fees!"
                ,pending_transfer_in as "pending_transfer_in!"
                --,portfolio_value as "portfolio_value!"    --deprecated (same as equity field)
                ,trading_blocked as "trading_blocked!"
                ,transfers_blocked as "transfers_blocked!"
                ,account_blocked as "account_blocked!"
                ,created_at as "created_at!"
                ,trade_suspended_by_user as "trade_suspended_by_user!"
                ,multiplier as "multiplier!"
                ,shorting_enabled as "shorting_enabled!"
                ,long_market_value as "long_market_value!"
                ,short_market_value as "short_market_value!"
                ,initial_margin as "initial_margin!"
                ,maintenance_margin as "maintenance_margin!"
                ,last_maintenance_margin as "last_maintenance_margin!"
                ,sma as "sma!"
            from alpaca_account
            order by dtg desc
            limit 1
        "#).fetch_one(pool).await{
            Ok(acct) => Ok(acct),
            Err(_e) => Err(TradeWebError::SqlxError)
        }
    }

    /*



     */


    pub async fn load_account(pool:&PgPool, settings:&Settings){

        match Account::get_remote(settings).await{
            Ok(account)=>{
                let _ = account.save_to_db(pool).await;
            },
            Err(e)=>{
                tracing::debug!("[load_account] error: {:?}", &e);
            }
        }


    }

    /// GET https://paper-api.alpaca.markets/v2/account
    pub async fn get_remote(settings:&Settings)-> Result<Account,TradeWebError> {

        let api_key = settings.alpaca_paper_id.clone();
        let api_secret = settings.alpaca_paper_secret.clone();
        let mut headers = HeaderMap::new();
        headers.insert("APCA-API-KEY-ID", api_key.parse().unwrap());
        headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());
        let url = format!("https://paper-api.alpaca.markets/v2/account");

        let client = reqwest::Client::new();
        let http_result = client.get(url).headers(headers).send().await;
        match http_result {
            Ok(resp) => {
                let json_text = &resp.text().await.unwrap();
                tracing::debug!("json: {}", &json_text);
                match serde_json::from_str::<Account>(&json_text) {
                    Ok(account) => Ok(account),
                    Err(e) => {
                        tracing::debug!("[get_account] json: {}", &json_text);
                        tracing::debug!("[get_account] json error: {:?}", &e);
                        Err(TradeWebError::JsonError)
                    }
                }
            }
            Err(e) => {
                tracing::debug!("[get_account] reqwest error: {:?}", &e);
                format!("reqwest error: {:?}", &e);
                Err(TradeWebError::SqlxError)
            }
        }



    }


    /// TODO: change all these long-running async db calls (except that it's okay these rest results are serial for now)
    pub async fn save_to_db(&self, pool:&PgPool) -> Result<(), TradeWebError> {

        /*


         */

        match sqlx::query!(
            r#"
                insert into alpaca_account(
                    dtg,
                    cash,
                    position_market_value,
                    equity,
                    last_equity,
                    daytrade_count,
                    balance_asof,
                    pattern_day_trader,
                    id,
                    account_number,
                    status,
                    -- crypto_status,
                    currency,
                    buying_power,
                    regt_buying_power,
                    daytrading_buying_power,
                    effective_buying_power,
                    non_marginable_buying_power,
                    bod_dtbp,
                    accrued_fees,
                    pending_transfer_in,
                    -- portfolio_value,    -- deprecated (same as equity)
                    trading_blocked,
                    transfers_blocked,
                    account_blocked,
                    created_at,
                    trade_suspended_by_user,
                    multiplier,
                    shorting_enabled,
                    long_market_value,
                    short_market_value,
                    initial_margin,
                    maintenance_margin,
                    last_maintenance_margin,
                    sma
                    -- crypto_tier: usize,

                ) values(now()::timestamptz, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32)

            "#,
            self.cash,self.position_market_value,self.equity,self.last_equity, self.daytrade_count,
            self.balance_asof, self.pattern_day_trader, self.id, self.account_number, self.status,
            self.currency, self.buying_power, self.regt_buying_power,self.daytrading_buying_power, self.effective_buying_power,
            self.non_marginable_buying_power, self.bod_dtbp, self.accrued_fees, self.pending_transfer_in, self.trading_blocked,
            self.transfers_blocked, self.account_blocked, self.created_at, self.trade_suspended_by_user, self.multiplier, self.shorting_enabled,
            self.long_market_value, self.short_market_value, self.initial_margin, self.maintenance_margin, self.last_maintenance_margin,
            self.sma

        ).execute(pool).await {
            Ok(_) => Ok(()),
            Err(_e) => Err(TradeWebError::SqlxError)
        }



    }


}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountWithDate {
    pub dtg: DateTime<Utc>,
    pub cash: BigDecimal,
    pub position_market_value: BigDecimal,
    pub equity: BigDecimal,
    pub last_equity: BigDecimal,
    pub daytrade_count: i32,
    pub balance_asof: String,
    pub pattern_day_trader: bool,
    pub id: String,
    pub account_number: String,
    pub status: String,
    // crypto_status: String,
    pub currency: String,
    pub buying_power: BigDecimal,
    pub regt_buying_power: BigDecimal,
    pub daytrading_buying_power: BigDecimal,
    pub effective_buying_power: BigDecimal,
    pub non_marginable_buying_power: BigDecimal,
    pub bod_dtbp: String,
    pub accrued_fees: BigDecimal,
    pub pending_transfer_in: BigDecimal,
    // pub portfolio_value: BigDecimal,    // deprecated (same as equity field)
    pub trading_blocked: bool,
    pub transfers_blocked: bool,
    pub account_blocked: bool,
    pub created_at: DateTime<Utc>,
    pub trade_suspended_by_user: bool,
    pub multiplier: BigDecimal,
    pub shorting_enabled: bool,
    pub long_market_value: BigDecimal,
    pub short_market_value: BigDecimal,
    pub initial_margin: BigDecimal,
    pub maintenance_margin: BigDecimal,
    pub last_maintenance_margin: BigDecimal,
    pub sma: String,
    // crypto_tier: i32,
}

/*

{"id":"9f835ea3-69d8-4618-8ab6-fba69db64c00","admin_configurations":{"Configurations":{}},"user_configurations":{"fractional_trading":false,"no_shorting":true},"account_number":"PA3WZVD1N4KP","status":"ACTIVE","crypto_status":"ACTIVE","currency":"USD","buying_power":"382728.7921","regt_buying_power":"199586.9911","daytrading_buying_power":"382728.7921","effective_buying_power":"382728.7921","non_marginable_buying_power":"92754.59","bod_dtbp":"385136.0344","cash":"98584.6366","accrued_fees":"0","pending_transfer_in":"0","portfolio_value":"101002.3545","pattern_day_trader":true,"trading_blocked":false,"transfers_blocked":false,"account_blocked":false,"created_at":"2023-06-16T01:09:13.057141Z","trade_suspended_by_user":false,"multiplier":"4","shorting_enabled":false,"equity":"101002.3545","last_equity":"100753.8886","long_market_value":"2417.7179","short_market_value":"0","position_market_value":"2417.7179","initial_margin":"1208.85895","maintenance_margin":"727.08588","last_maintenance_margin":"4469.88","sma":"95216.04","daytrade_count":633,"balance_asof":"2023-07-18","crypto_tier":1}
2023-07-19T18:24:09.062223Z DEBUG common_lib::account: [get_account] json: {"id":"9f835ea3-69d8-4618-8ab6-fba69db64c00","admin_configurations":{"Configurations":{}},"user_configurations":{"fractional_trading":false,"no_shorting":true},"account_number":"PA3WZVD1N4KP","status":"ACTIVE","crypto_status":"ACTIVE","currency":"USD","buying_power":"382728.7921","regt_buying_power":"199586.9911","daytrading_buying_power":"382728.7921","effective_buying_power":"382728.7921","non_marginable_buying_power":"92754.59","bod_dtbp":"385136.0344","cash":"98584.6366","accrued_fees":"0","pending_transfer_in":"0","portfolio_value":"101002.3545","pattern_day_trader":true,"trading_blocked":false,"transfers_blocked":false,"account_blocked":false,"created_at":"2023-06-16T01:09:13.057141Z","trade_suspended_by_user":false,"multiplier":"4","shorting_enabled":false,"equity":"101002.3545","last_equity":"100753.8886","long_market_value":"2417.7179","short_market_value":"0","position_market_value":"2417.7179","initial_margin":"1208.85895","maintenance_margin":"727.08588","last_maintenance_margin":"4469.88","sma":"95216.04","daytrade_count":633,"balance_asof":"2023-07-18","crypto_tier":1}


 */

#[cfg(test)]
mod tests{
    use crate::account::Account;

    #[test]
    /// confirm parsing from json to struct
    fn parse_account(){
        // println!("test_account: {}", TEST_ACCOUNT);
        let account_result = serde_json::from_str::<Account>(TEST_ACCOUNT);
        println!("test_account: {:?}", &account_result);
        assert!(account_result.is_ok(), "account result was not okay");
    }

    const TEST_ACCOUNT:&str = r#"
        {"id":"9f835ea3-69d8-4618-8ab6-fba69db64c00","admin_configurations":{"Configurations":{}},"user_configurations":{"fractional_trading":false,"no_shorting":true},"account_number":"SOMETHINGFAKE","status":"ACTIVE","crypto_status":"ACTIVE","currency":"USD","buying_power":"384048.5321","regt_buying_power":"200916.9035","daytrading_buying_power":"384048.5321","effective_buying_power":"384048.5321","non_marginable_buying_power":"92758.87","bod_dtbp":"385136.0344","cash":"99905.9866","accrued_fees":"0","pending_transfer_in":"0","portfolio_value":"101010.9169","pattern_day_trader":true,"trading_blocked":false,"transfers_blocked":false,"account_blocked":false,"created_at":"2023-06-16T01:09:13.057141Z","trade_suspended_by_user":false,"multiplier":"4","shorting_enabled":false,"equity":"101010.9169","last_equity":"100753.8886","long_market_value":"1104.9303","short_market_value":"0","position_market_value":"1104.9303","initial_margin":"552.46515","maintenance_margin":"333.25296","last_maintenance_margin":"4469.88","sma":"95216.04","daytrade_count":633,"balance_asof":"2023-07-18","crypto_tier":1}
    "#;



}