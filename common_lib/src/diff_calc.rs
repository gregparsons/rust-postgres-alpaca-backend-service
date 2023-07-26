//! diff_calc.rs
//!
//! Used by poller_buy
//!

use std::iter::zip;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use crate::alpaca_api_structs::CrossStatus;
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

    /// Compare two containing rows of stocks and associated snapshot data. Compare two snapshots for each
    /// given stock to see if a cross up/down has occurred for that stock for that pair of lines.
    pub fn compare_snapshots(
        snapshot_0: &Result<Vec<DiffCalc>,PollerError>,
        snapshot_1: &Result<Vec<DiffCalc>,PollerError>
    ) -> Result<Vec<(String, CrossStatus)>, PollerError>{

        // Do error detection first
        if (snapshot_0.is_ok() && snapshot_1.is_ok()) == false {
            return Err(PollerError::SnapshotsNotOkayFromDatabase)
        }

        // otherwise continue on...(knowing they're both okay to unwrap); get contents as refs too.
        let v0 = snapshot_0.as_ref().unwrap();
        let v1 = snapshot_1 .as_ref().unwrap();
        if v0.len() != v1.len(){
            // These vectors should be sorted and should have the same number of entries
            return Err(PollerError::DifferentSizeSnapshots)
        }

        let vec_of_crosses /*:Vec<(String,CrossStatus)>*/ = zip(v0, v1)
            .into_iter()
            .map(|z| {
                let cross_30s_1m = {
                    //  negative then positive (including zero)...
                    if z.0.diff_30s_1 < BigDecimal::from(0) && z.1.diff_30s_1 >= BigDecimal::from(0) {
                        CrossStatus::Up
                    }
                    // positive (including zero) then negative...
                    else if z.0.diff_30s_1 >= BigDecimal::from(0) && z.1.diff_30s_1 < BigDecimal::from(0) {
                        CrossStatus::Down
                    }
                    else {
                        CrossStatus::None
                    }
                };
                // tracing::debug!("{}: is_cross_30s_1m: {}", z.0.symbol, is_cross_30s_1m);
                // let is_cross_30s_3m = if z.1.diff_30s_3 >= BigDecimal::from(0) && z.0.diff_30s_3 < BigDecimal::from(0) { true } else { false };
                // tracing::debug!("{}: is_cross_30s_3m: {}", z.0.symbol, is_cross_30s_3m);
                // let is_cross_30s_5m = if z.1.diff_30s_5 >= BigDecimal::from(0) && z.0.diff_30s_3 < BigDecimal::from(0) { true } else { false };
                // tracing::debug!("{}: is_cross_30s_5m: {}", z.0.symbol, is_cross_30s_5m);

                // tuple of the symbol and whether it's a cross
                (z.0.symbol.clone(), cross_30s_1m)
            }
            ).collect();

        Ok(vec_of_crosses)
    }

    /// Get the grid provided by v_alpaca_diff
    pub fn get(tx_db:crossbeam_channel::Sender<DbMsg>)->Result<Result<Vec<DiffCalc>, PollerError>, TradeWebError> {
        let (tx, resp_rx) = crossbeam_channel::unbounded();
        tx_db.send(DbMsg::DiffCalcGet{sender:tx}).unwrap();
        match resp_rx.recv(){
            Ok(result)=>Ok(result),
            Err(_e)=>Err(TradeWebError::ChannelError),
        }
    }
}

