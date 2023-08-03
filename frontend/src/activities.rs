//! symbols.rs
//!
//! Render a list of all Alpaca trade activities. Can be filtered by symbol. (TODO: paging)
//!

use actix_session::Session;
use actix_web::{web, HttpResponse};
use crossbeam_channel::Sender;
use common_lib::alpaca_activity::Activity;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::http::redirect_home;
use common_lib::symbol_list::SymbolList;
use handlebars::Handlebars;
use serde_json::json;
use common_lib::db::DbMsg;

/// GET /activity
pub async fn get_activities(tx_db: web::Data<Sender<DbMsg>>, hb: web::Data<Handlebars<'_>>, session: Session) -> HttpResponse {
    let tx_db = tx_db.into_inner().as_ref().clone();
    get_activities_with_message(tx_db, hb, session, "").await
}

async fn get_activities_with_message(tx_db: Sender<DbMsg>, hb: web::Data<Handlebars<'_>>, session: Session, message: &str) -> HttpResponse {

    // let tx_db = tx_db.into_inner().as_ref().clone();

    // require login
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {

        let activity_vec = Activity::get_activities_from_db(tx_db.clone()).await;

        let symbol_list = match SymbolList::get_all_symbols(tx_db.clone()).await {
            Ok(symbol_list) => symbol_list,
            Err(e) => {
                tracing::debug!("[get_dashboard] error getting symbols {:?}", &e);
                vec!()
            }
        };

        tracing::debug!("[get_activities_with_message] activity_vec: {:?}", &activity_vec);

        let data = json!({
            "title": "Activity",
            "parent": "base0",
            "is_logged_in": true,
            "session_username": &session_username,
            "data": &activity_vec,
            "message": message,
            "symbols": symbol_list,
        });
        let body = hb.render("activity_table", &data).unwrap();
        HttpResponse::Ok().append_header(("cache-control", "no-store")).body(body)

    } else {
        redirect_home().await
    }
}

/// GET /activity/{symbol}
pub async fn get_activity_for_symbol(symbol: web::Path<String>, tx_db: web::Data<Sender<DbMsg>>, hb: web::Data<Handlebars<'_>>, session: Session) -> HttpResponse {

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

        let activity_vec = Activity::get_activities_from_db_for_symbol(&symbol, tx_db.clone()).await;

        tracing::debug!("[get_activities_with_message] activity_vec: {:?}",&activity_vec);

        let data = json!({
            "title": "Activity",
            "parent": "base0",
            "is_logged_in": true,
            "session_username": &session_username,
            "data": &activity_vec,
            "message": message,
            "symbols": symbol_list,
        });
        let body = hb.render("activity_table", &data).unwrap();
        HttpResponse::Ok().append_header(("cache-control", "no-store")).body(body)

    } else {
        redirect_home().await
    }
}
