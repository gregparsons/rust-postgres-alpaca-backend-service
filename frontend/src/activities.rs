//! symbols.rs
//!
//! get, post a list of stock symbols and whether they're actively traded.
//!

use actix_session::Session;
use actix_web::{HttpResponse, web};
use handlebars::Handlebars;
use serde_json::json;
use sqlx::PgPool;
use common_lib::http::redirect_home;
use common_lib::alpaca_activity::Activity;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::symbol_list::SymbolList;

///
/// GET /symbols
///
pub async fn get_activities(pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session:Session) -> HttpResponse {
    get_activities_with_message(pool, hb, session, "").await
}

async fn get_activities_with_message(pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session:Session, message:&str)-> HttpResponse {

    // require login
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {

        let activity_vec_result = Activity::get_activities_from_db(&pool).await;

        let symbol_list = match SymbolList::get_all_symbols(&pool).await {
            Ok(symbol_list) => {
                symbol_list
            },
            Err(e) => {
                tracing::debug!("[get_dashboard] error getting symbols {:?}", &e);
                vec![]
            }
        };

        match activity_vec_result {
            Ok(activity_vec) => {

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
            },
            Err(e) => {
                // TODO: redirect to error message
                tracing::debug!("[get_symbols] error getting symbols: {:?}", &e);
                redirect_home().await
            }
        }
    } else {
        redirect_home().await
    }

}


// GET /activity/{symbol}
pub async fn get_activity_for_symbol(symbol: web::Path<String>, pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session:Session) -> HttpResponse {

    let message = "";
    let symbol =symbol.into_inner();

    // require login
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {

        let symbol_list = match SymbolList::get_all_symbols(&pool).await {
            Ok(symbol_list) => {
                symbol_list
            },
            Err(e) => {
                tracing::debug!("[get_dashboard] error getting symbols {:?}", &e);
                vec![]
            }
        };

        let activity_vec_result = Activity::get_activities_from_db_for_symbol(&symbol, &pool).await;

        match activity_vec_result {
            Ok(activity_vec) => {

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
            },
            Err(e) => {
                tracing::debug!("[get_symbols] error getting symbols: {:?}", &e);
                redirect_home().await
            }
        }
    } else {
        redirect_home().await
    }

}









