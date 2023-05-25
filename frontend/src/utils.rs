//! utils.rs

use actix_session::Session;
use actix_web::{HttpResponse, Responder, web};
use handlebars::Handlebars;
use serde_json::json;
use common_lib::common_structs::SESSION_USERNAME;

/// authorization: not required
pub async fn get_home(hb: web::Data<Handlebars<'_>>, session:Session) -> HttpResponse {
    tracing::debug!("[get_home]");

    let mut is_logged_in=false;
    let mut cookie_user=String::new();

    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {
        is_logged_in = true;
        cookie_user = session_username;
    }

    // pass username if logged in;
    let data = json!({
        "title": "",
        "parent": "base0",
        "is_logged_in": is_logged_in,
        "session_username": cookie_user,
    });
    let body = hb.render("home", &data).unwrap();

    HttpResponse::Ok().append_header(("Cache-Control", "no-store")).body(body)
}

/// say "pong"
/// authorization: none
pub async fn get_ping() -> impl Responder {
    tracing::debug!("[get_pong]");
    HttpResponse::Ok().body("pong")
}

