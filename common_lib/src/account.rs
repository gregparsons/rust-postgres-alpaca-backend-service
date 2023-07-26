//! account.rs
//!
//! https://alpaca.markets/docs/api-references/trading-api/account/

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use crossbeam_channel::SendError;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::RecvError;
use crate::db::DbMsg;
use crate::error::TradeWebError;
use crate::settings::Settings;

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    pub async fn get(tx_db: crossbeam_channel::Sender<DbMsg>) -> Result<AccountWithDate, RecvError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = DbMsg::AccountGet{ resp_tx };
        tx_db.send(msg).unwrap();
        let result = resp_rx.await;
        result

    }

    /// Return the cash in dollars available to trade with (not including day trade minimum)
    pub async fn available_cash(tx_db: crossbeam_channel::Sender<DbMsg>)->Result<BigDecimal, TradeWebError>{
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = DbMsg::AccountAvailableCash{ sender_tx:resp_tx };
        tx_db.send(msg).unwrap();
        match resp_rx.await {
            Ok(cash) => Ok(cash),
            Err(e) => {
                tracing::error!("[available_cash] channel error: {:?}", &e);
                Err(TradeWebError::ChannelError)
            }
        }
    }


    /// get account from the web API and save to the local database
    pub fn load_account(settings:&Settings, tx_db: crossbeam_channel::Sender<DbMsg>){

        match Account::get_remote(settings, tx_db.clone()) {
            Ok(account)=>{
                tracing::debug!("[load_account] got remote account, saving to db...");
                match account.save_to_db(tx_db.clone()){
                    Ok(_)=>{
                        tracing::debug!("[load_account] save to db ok");
                    },
                    Err(e) =>{
                        tracing::error!("[load_account] save to db failed: {:?}", &e);
                    }
                }
            },
            Err(e)=>{
                tracing::error!("[load_account] error: {:?}", &e);
            }
        }

    }

    /// GET https://paper-api.alpaca.markets/v2/account
    /// TODO: move the web api calls out of db.rs so they're not competing with that thread
    pub fn get_remote(settings:&Settings, tx_db: crossbeam_channel::Sender<DbMsg>) -> Result<Account, RecvError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let msg = DbMsg::AccountGetRemote{
            settings:settings.clone(),
            resp_tx
        };
        tx_db.send(msg).unwrap();
        let result = resp_rx.blocking_recv();
        result

    }

    pub fn save_to_db(&self, tx_db: crossbeam_channel::Sender<DbMsg>) -> Result<(), SendError<DbMsg>> {
        tx_db.send(DbMsg::AccountSaveToDb { account: (*self).clone() })
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