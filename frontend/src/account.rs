//! account.rs
//!
//! present account data in the frontend, retrieved from the alpaca API

use actix_session::Session;
use actix_web::{web, HttpResponse, Responder};
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::http::redirect_home;
use handlebars::Handlebars;
use reqwest::header::HeaderMap;
use serde_json::json;
use sqlx::PgPool;
// use serde::{Serialize, Deserialize};
use common_lib::account::Account;
use common_lib::settings::Settings;

/// GET /account
pub async fn get_account(
    hb: web::Data<Handlebars<'_>>,
    pool: web::Data<PgPool>,
    session: Session,
) -> impl Responder {
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {
        tracing::debug!("session id: {}", &session_username);
        let mut headers = HeaderMap::new();

        // let api_key_id = std::env::var("ALPACA_API_ID").expect("ALPACA_API_ID environment variable not found");
        // let api_secret = std::env::var("ALPACA_API_SECRET").expect("alpaca_secret environment variable not found");

        match Settings::load(&pool).await {
            Ok(settings) => {
                let api_key = settings.alpaca_paper_id.clone();
                let api_secret = settings.alpaca_paper_secret.clone();
                headers.insert("APCA-API-KEY-ID", api_key.parse().unwrap());
                headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());
                let url = format!("https://paper-api.alpaca.markets/v2/account");
                tracing::debug!("[load_fill_activities] calling API: {}", &url);
                // get a single order
                let client = reqwest::Client::new();
                let http_result = client.get(url).headers(headers).send().await;
                let account_body: Account = match http_result {
                    Ok(resp) => {
                        let json_text = &resp.text().await.unwrap();
                        tracing::debug!("json: {}", &json_text);
                        match serde_json::from_str::<Account>(&json_text) {
                            Ok(account) => {
                                tracing::debug!("[get_account] account\n: {:?}", &account);
                                // 3. merge remote results to local database
                                account
                            }
                            Err(e) => {
                                tracing::debug!("[get_account] json: {}", &json_text);
                                tracing::debug!("[get_account] json error: {:?}", &e);
                                Account::blank()
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("[get_account] reqwest error: {:?}", &e);
                        format!("reqwest error: {:?}", &e);
                        Account::blank()
                    }
                };

                // HttpResponse::Ok().body(body)

                let data = json!({
                    "title": "Account",
                    "parent": "base0",
                    "is_logged_in": true,
                    "session_username": &session_username,
                    "message": account_body,
                });
                let body = hb.render("account", &data).unwrap();
                HttpResponse::Ok()
                    .append_header(("cache-control", "no-store"))
                    .body(body)
            }
            Err(e) => {
                tracing::debug!(
                    "[get_account] couldn't load settings (to get alpaca id/secret): {:?}",
                    &e
                );
                redirect_home().await
            }
        }
    } else {
        redirect_home().await
    }
}
