//! dashboard.rs

use actix_session::Session;
use actix_web::{web, HttpResponse};
use common_lib::alpaca_activity::Activity;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::http::redirect_home;
use common_lib::symbol_list::SymbolList;
use handlebars::Handlebars;
use serde_json::json;
use sqlx::PgPool;

/// GET /dashboard
pub async fn get_dashboard(
    pool: web::Data<PgPool>,
    hb: web::Data<Handlebars<'_>>,
    session: Session,
) -> HttpResponse {
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {
        let message = "TBD dashboard (no symbol specified)".to_string();

        let symbol_list = match SymbolList::get_all_symbols(&pool).await {
            Ok(symbol_list) => symbol_list,
            Err(e) => {
                tracing::debug!("[get_dashboard] error getting symbols {:?}", &e);
                vec![]
            }
        };

        let data = json!({
            "title": "",
            "parent": "base0",
            "is_logged_in": true,
            "session_username": session_username,
            "message": message,
            "symbols":symbol_list,
        });
        let body = hb.render("dashboard", &data).unwrap();
        HttpResponse::Ok()
            .append_header(("Cache-Control", "no-store"))
            .body(body)
    } else {
        redirect_home().await
    }
}

/// GET /dashboard/{symbol}
pub async fn get_dashboard_with_symbol(
    symbol: web::Path<String>,
    pool: web::Data<PgPool>,
    hb: web::Data<Handlebars<'_>>,
    session: Session,
) -> HttpResponse {
    let symbol = symbol.into_inner();

    if let Ok(Some(_session_username)) = session.get::<String>(SESSION_USERNAME) {
        let message = format!("TBD dashboard for symbol: {}", &symbol);
        match SymbolList::get_all_symbols(&pool).await {
            Ok(symbol_list) => {
                render_dashboard(&symbol, &symbol_list, message, pool, hb, session).await
            }
            Err(e) => {
                tracing::debug!("[get_dashboard_with_symbol] {:?}", &e);
                render_dashboard(&symbol, &vec![], message, pool, hb, session).await
            }
        }
    } else {
        redirect_home().await
    }
}

/// render dashboard
async fn render_dashboard(
    symbol: &str,
    symbol_list: &Vec<String>,
    message: String,
    pool: web::Data<PgPool>,
    hb: web::Data<Handlebars<'_>>,
    session: Session,
) -> HttpResponse {
    let activities = Activity::get_activities_from_db_for_symbol(symbol, &pool)
        .await
        .unwrap_or(vec![]);
    // tracing::debug!("[render_dashboard] got activities: {:?}", &activities);

    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {
        // pass username if logged in;
        let data = json!({
            "title": "",
            "parent": "base0",
            "is_logged_in": true,
            "session_username": session_username,
            "message": message,
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

// /// POST /dashboard?symbol_select=aapl
// pub async fn post_dashboard_with_symbol(symbol_form: web::Form<DashboardForm>, _pool: web::Data<PgPool>, _hb: web::Data<Handlebars<'_>>, _session:Session) -> impl Responder {
//     tracing::debug!("[post_dashboard_with_symbol] symbol: {}", &symbol_form.symbol);
//     format!("post_dashboard_with_symbol: {}", symbol_form.into_inner().symbol)
//
// }
