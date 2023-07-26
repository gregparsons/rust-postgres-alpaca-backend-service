//!

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use crossbeam_channel::Sender;
use sqlx::types::Uuid;
use crate::db::DbMsg;
use crate::trade_struct::TradeSide;

#[derive(Debug, Clone)]
pub struct OrderLogEntry {
    pub id:Uuid,
    pub id_group:Uuid,
    pub dtg:DateTime<Utc>,
    pub symbol:String,
    pub side:TradeSide,
    pub qty:BigDecimal,

}

impl OrderLogEntry{

    pub fn new(symbol:String, side:TradeSide, qty:BigDecimal)->OrderLogEntry{

        // generate a client ID
        // https://docs.alpaca.markets/reference/submit-an-order
        // https://alpaca.markets/docs/trading/orders/
        let id = Uuid::new_v4();

        // TODO: let this be assignable since we want to assign the same group to sales subsequent to buys
        let id_group = Uuid::new_v4();
        let dtg = Utc::now();

        tracing::debug!("[new] group: {}, uuid: {}", &id_group, &id);

        OrderLogEntry{
            id,
            id_group,
            dtg,
            symbol:symbol.to_lowercase(),
            side,
            qty,
        }

    }

    /// Save a single order to the database
    pub fn save(&self, tx_db:Sender<DbMsg>) {
        let entry = (*self).clone();
        let _ = tx_db.send(DbMsg::OrderLogEntrySave{ entry });
    }

    /// concatenate the group and id to form a unique order_client_id to appease alpaca but to enable
    pub fn id_client(&self) ->String{
        format!("{}---{}", &self.id_group, &self.id)
    }

    pub fn symbol(&self)->String{
        self.symbol.to_uppercase()
    }
}
