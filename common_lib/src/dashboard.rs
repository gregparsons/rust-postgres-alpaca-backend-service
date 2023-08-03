//! dashboard container for frontend display

use crossbeam_channel::Sender;
use serde::Serialize;
use tokio::sync::oneshot;
use crate::db::DbMsg;

/// frontend dashboard row
#[derive(Debug, Serialize)]
pub struct Dashboard{
    pub k:String,
    pub v:String,
}

/// returns a vec of k/v entries for the dashboard page; returns an empty vec on errors
pub async fn get(tx_db:Sender<DbMsg>)->Vec<Dashboard>{

    let (tx, rx) = oneshot::channel();
    match tx_db.send(DbMsg::AcctDashboardGet {sender:tx}){
        Ok(_)=>{
            match rx.await{
                Ok(vec)=>vec,
                Err(e)=>{
                    tracing::error!("[get] recv error: {:?}", &e);
                    vec!()
                }
            }
        },
        Err(e) => {
            tracing::error!("[get] send error: {:?}", &e);
            vec!()
        }
    }



}