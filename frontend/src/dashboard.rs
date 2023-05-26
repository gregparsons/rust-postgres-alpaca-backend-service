//! dashboard.rs

use actix_session::Session;
use actix_web::{HttpResponse, Responder, web};
use handlebars::Handlebars;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::http::redirect_home;


pub async fn get_dashboard(pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session:Session) -> impl Responder {
    let data = "TBD dashboard (no symbol specified)".to_string();
    dashboard_with_data(data, pool, hb, session).await
}

pub async fn get_dashboard_with_symbol(symbol: web::Path<String>, pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session:Session) -> impl Responder {
    let data = format!("TBD dashboard for symbol: {}", symbol.into_inner());
    dashboard_with_data(data, pool, hb, session).await
}


async fn dashboard_with_data(data:String, _pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session:Session) -> impl Responder{

    let symbol_list = vec!["aapl", "brds", "f"];

    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {
        // pass username if logged in;
        let data = json!({
            "title": "",
            "parent": "base0",
            "is_logged_in": true,
            "session_username": session_username,
            "data": data,
            "symbols":symbol_list,
        });
        let body = hb.render("dashboard", &data).unwrap();
        HttpResponse::Ok().append_header(("Cache-Control", "no-store")).body(body)

    } else {
        // not logged in
        redirect_home().await
    }
}

#[derive(Deserialize)]
pub struct DashboardForm {
    symbol:String,
}

/// POST /dashboard?symbol_select=aapl
pub async fn post_dashboard_with_symbol(symbol_form: web::Form<DashboardForm>, _pool: web::Data<PgPool>, _hb: web::Data<Handlebars<'_>>, _session:Session) -> impl Responder {
    tracing::debug!("[post_dashboard_with_symbol] symbol: {}", &symbol_form.symbol);
    format!("post_dashboard_with_symbol: {}", symbol_form.into_inner().symbol)

}