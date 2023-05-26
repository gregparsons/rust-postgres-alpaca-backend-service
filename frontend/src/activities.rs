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








