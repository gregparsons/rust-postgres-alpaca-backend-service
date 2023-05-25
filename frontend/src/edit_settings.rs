//! edit_settings.rs
//!
//! web form to edit settings

use actix_session::Session;
use actix_web::{HttpResponse, web};
use handlebars::{Handlebars};
use serde_json::json;
use sqlx::PgPool;
use common_lib::http::redirect_home;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::settings::Settings;
use common_lib::trade_setting_profile::SettingsProfile;

/// GET /settings
pub async fn get_settings(pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session:Session) -> HttpResponse {
    get_settings_with_message(pool, hb, session, "").await
}

async fn get_settings_with_message(pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session:Session, message:&str)-> HttpResponse {

    // require login
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {
        let setting_result = Settings::load_no_secret(&pool).await;
        match setting_result {
            Ok(settings) => {
                let data = json!({
                    "title": "Settings",
                    "parent": "base0",
                    "is_logged_in": true,
                    "session_username": &session_username,
                    "data": &settings,
                    "message": message,
                });

                let body = hb.render("settings_table", &data).unwrap();
                HttpResponse::Ok().append_header(("cache-control", "no-store")).body(body)

            },
            Err(e) => {
                // TODO: redirect to error message
                tracing::debug!("[get_settings] error getting symbols: {:?}", &e);
                redirect_home().await
            }
        }
    } else {
        redirect_home().await
    }

}



/// select * from fn_set_trade_settings('buy');
/// select * from fn_set_trade_settings('close');
/// select * from fn_set_trade_settings('close2');
///
/// GET /settings/button/{name}
pub async fn get_settings_button(path: web::Path<SettingsProfile>, pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session:Session) -> HttpResponse {

    tracing::debug!("[get_settings_button] path: {:?}", &path);

    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {

        let profile_selected:SettingsProfile = path.into_inner();

        match Settings::change_trade_profile(&profile_selected, &pool).await {

            // TODO: a little duplicated code here

            Ok(settings) => {

                let message = format!("{} trade profile selected", &profile_selected);

                let data = json!({
                    "title": "Settings",
                    "parent": "base0",
                    "is_logged_in": true,
                    "session_username": &session_username,
                    "data": &settings,
                    "message": message,
                });

                let body = hb.render("settings_table", &data).unwrap();
                HttpResponse::Ok().append_header(("cache-control", "no-store")).body(body)

            },
            Err(e) => {
                // TODO: redirect to error message
                tracing::debug!("[get_settings] error getting symbols: {:?}", &e);
                redirect_home().await
            }
        }
    } else {
        redirect_home().await
    }
}