//! symbols.rs
//!
//! Render a list of all Alpaca trade transactions. Can be filtered by symbol. (TODO: paging)
//!

use actix_session::Session;
use actix_web::{web, HttpResponse};
use crossbeam_channel::Sender;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::http::redirect_home;
use common_lib::symbol_list::SymbolList;
use handlebars::Handlebars;
use serde_json::json;
use common_lib::db::DbMsg;
use common_lib::transaction;

/// GET /transaction
pub async fn get_transactions(tx_db: web::Data<Sender<DbMsg>>, hb: web::Data<Handlebars<'_>>, session: Session) -> HttpResponse {
    let tx_db = tx_db.into_inner().as_ref().clone();
    get_transactions_with_message(tx_db, hb, session, "").await
}

async fn get_transactions_with_message(tx_db: Sender<DbMsg>, hb: web::Data<Handlebars<'_>>, session: Session, message: &str) -> HttpResponse {

    // let tx_db = tx_db.into_inner().as_ref().clone();

    // require login
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {

        let transaction_vec = transaction::get(None, tx_db.clone()).await;

        let symbol_list = match SymbolList::get_all_symbols(tx_db.clone()).await {
            Ok(symbol_list) => symbol_list,
            Err(e) => {
                tracing::debug!("[get_dashboard] error getting symbols {:?}", &e);
                vec!()
            }
        };

        tracing::debug!("[get_transactions_with_message] transaction_vec: {:?}", &transaction_vec);

        let data = json!({
            "title": "Transactions",
            "parent": "base0",
            "is_logged_in": true,
            "session_username": &session_username,
            "data": &transaction_vec,
            "message": message,
            "symbols": symbol_list,
        });
        let body = hb.render("transactions", &data).unwrap();
        HttpResponse::Ok().append_header(("cache-control", "no-store")).body(body)

    } else {
        redirect_home().await
    }
}

/// GET /transaction/{symbol}
pub async fn get_transaction_for_symbol(symbol: web::Path<String>, tx_db: web::Data<Sender<DbMsg>>, hb: web::Data<Handlebars<'_>>, session: Session) -> HttpResponse {

    let message = "";
    let tx_db = tx_db.into_inner().as_ref().clone();
    let symbol = symbol.into_inner();

    // require login
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {
        let symbol_list = match SymbolList::get_all_symbols(tx_db.clone()).await {
            Ok(symbol_list) => symbol_list,
            Err(e) => {
                tracing::debug!("[get_dashboard] error getting symbols {:?}", &e);
                vec![]
            }
        };

        let transaction_vec = transaction::get(Some(symbol.clone()), tx_db.clone()).await;

        tracing::debug!("[get_transactions_with_message] transaction_vec: {:?}",&transaction_vec);

        let data = json!({
            "title": "Transactions",
            "parent": "base0",
            "is_logged_in": true,
            "session_username": &session_username,
            "data": &transaction_vec,
            "message": message,
            "symbols": symbol_list,
        });
        let body = hb.render("transactions", &data).unwrap();
        HttpResponse::Ok().append_header(("cache-control", "no-store")).body(body)

    } else {
        redirect_home().await
    }
}
