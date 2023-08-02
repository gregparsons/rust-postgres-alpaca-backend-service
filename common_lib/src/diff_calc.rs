//! diff_calc.rs
//!
//! Used by poller_buy
//!

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use crate::db::DbMsg;
use crate::error::{PollerError, TradeWebError};

/// "Diff" corresponds to the difference between two market chart lines. When the diff is zero they're
/// crossing. The only way I know to tell, over time, whether a cross is occuring is to sample two
/// diff snapshots and compare their sign. If they're not the same then a cross is happening, negative
/// to positive indicates the faster moving of the two lines has swung upward, soon to drag the slower
/// one upward (hopefully) as well. A cross downward is a positive value in a diff in a snapshot followed
/// by a negative value in the subsequent sampling.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DiffCalc{
    pub dtg:DateTime<Utc>,
    pub dtg_pacific:DateTime<Utc>,
    pub symbol:String,
    pub diff_30s_1:BigDecimal,
    pub diff_30s_3:BigDecimal,
    pub diff_30s_5:BigDecimal,
    pub price_last:BigDecimal,
    pub price_30s:BigDecimal,
    pub price_1m:BigDecimal,
    pub price_3m:BigDecimal,
    pub price_5m:BigDecimal,
    pub dtg_last:DateTime<Utc>,
    pub dtg_last_pacific:DateTime<Utc>,
}

#[allow(dead_code)]
impl DiffCalc{

    /// Get the grid provided by v_alpaca_diff
    pub fn get(tx_db:crossbeam_channel::Sender<DbMsg>)->Result<Result<Vec<DiffCalc>, PollerError>, TradeWebError> {
        let (tx, resp_rx) = crossbeam_channel::unbounded();
        tx_db.send(DbMsg::DiffCalcGet{sender:tx}).unwrap();
        match resp_rx.recv(){
            Ok(result)=>{
                // tracing::debug!("[get] result: {:?}", &result);
                Ok(result)
            },
            Err(_e)=>{
                tracing::error!("[get] recv error: {:?}", &_e);
                Err(TradeWebError::ChannelError)
            },
        }
    }
}

