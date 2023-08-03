//! dashboard.rs
//!
//! Under construction

use actix_session::Session;
use actix_web::{web, HttpResponse};
use crossbeam_channel::Sender;
use common_lib::alpaca_activity::Activity;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::http::redirect_home;
use common_lib::symbol_list::SymbolList;
use handlebars::Handlebars;
use serde_json::json;
use common_lib::dashboard;
use common_lib::db::DbMsg;

/// GET /dashboard
pub async fn get_dashboard(tx_db: web::Data<Sender<DbMsg>>, hb: web::Data<Handlebars<'_>>, session: Session) -> HttpResponse {
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {
        let message = "".to_string();
        let tx_db = tx_db.into_inner().as_ref().clone();
        let symbol_list = match SymbolList::get_active_symbols(tx_db.clone()).await {
            Ok(symbol_list) => symbol_list,
            Err(e) => {
                tracing::debug!("[get_dashboard] error getting symbols {:?}", &e);
                vec![]
            }
        };
        let vec_dashboard = dashboard::get(tx_db.clone()).await;
        let data = json!({
            "title": "",
            "parent": "base0",
            "is_logged_in": true,
            "session_username": session_username,
            "message": message,
            "dashboard": vec_dashboard,
            "symbols":symbol_list,
        });
        let body = hb.render("dashboard", &data).unwrap();
        HttpResponse::Ok().append_header(("Cache-Control", "no-store")).body(body)

    } else {
        redirect_home().await
    }
}

/// GET /dashboard/{symbol}
pub async fn get_dashboard_with_symbol(symbol: web::Path<String>, tx_db: web::Data<Sender<DbMsg>>, hb: web::Data<Handlebars<'_>>, session: Session) -> HttpResponse {
    let symbol = symbol.into_inner();

    if let Ok(Some(_session_username)) = session.get::<String>(SESSION_USERNAME) {
        let message = format!("TBD dashboard for symbol: {}", &symbol);
        let tx_db = tx_db.into_inner().as_ref().clone();
        match SymbolList::get_active_symbols(tx_db.clone()).await {
            Ok(symbol_list) => render_dashboard(&symbol, &symbol_list, message, tx_db.clone(), hb, session).await,
            Err(e) => {
                tracing::debug!("[get_dashboard_with_symbol] {:?}", &e);
                render_dashboard(&symbol, &vec![], message, tx_db.clone(), hb, session).await
            }
        }
    } else {
        redirect_home().await
    }
}

/// render dashboard
async fn render_dashboard(symbol: &str, symbol_list: &Vec<String>, message: String, tx_db: Sender<DbMsg>, hb: web::Data<Handlebars<'_>>, session: Session) -> HttpResponse {

    let activities = Activity::get_activities_from_db_for_symbol(symbol, tx_db.clone()).await;

    let vec_dashboard = dashboard::get(tx_db.clone()).await;

    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {
        // pass username if logged in;
        let data = json!({
            "title": "",
            "parent": "base0",
            "is_logged_in": true,
            "session_username": session_username,
            "message": message,
            "dashboard": vec_dashboard,
            "symbols":symbol_list,
            "activities":activities,
        });
        let body = hb.render("dashboard", &data).unwrap();
        HttpResponse::Ok()
            .append_header(("Cache-Control", "no-store"))
            .body(body)
    } else {
        // not logged in
        redirect_home().await
    }
}

