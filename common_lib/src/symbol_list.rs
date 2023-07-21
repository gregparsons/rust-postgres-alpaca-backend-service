//! symbol_list.rs
//!
//! get the active symbols from the database

use crossbeam_channel::RecvError;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use crate::db::DbMsg;
use crate::settings::Settings;

#[derive(Debug, Deserialize, Serialize)]
pub struct QrySymbol {
    pub(crate) symbol: String,
}

pub struct SymbolList {}

impl SymbolList {
    /// get a vec of stock symbols
    pub async fn get_active_symbols(tx_db: crossbeam_channel::Sender<DbMsg>) -> Result<Vec<String>, RecvError> {

        let (resp_tx, resp_rx) = crossbeam_channel::unbounded();
        let msg = DbMsg::GetSymbolList{ resp_tx };
        tx_db.send(msg).unwrap();
        let result = resp_rx.recv();
        result

    }

    /// get a vec of stock symbols
    pub async fn get_all_symbols(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
        let result: Result<Vec<QrySymbol>, sqlx::Error> = sqlx::query_as!(
            QrySymbol,
            r#"select symbol as "symbol!" from t_symbol order by symbol"#
        )
        .fetch_all(pool)
        .await;

        match result {
            Ok(symbol_list) => {
                tracing::debug!("[get_symbols] symbol_list: {:?}", &symbol_list);
                let s = symbol_list.iter().map(|x| x.symbol.clone()).collect();
                Ok(s)
            }
            Err(e) => {
                tracing::debug!("[get_symbols] error: {:?}", &e);
                Err(e)
            }
        }
    }
}
