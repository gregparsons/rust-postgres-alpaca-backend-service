//! account.rs
//!
//! Present account data in the frontend, retrieved from the alpaca API

use actix_session::Session;
use actix_web::{web, HttpResponse, Responder};
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::http::redirect_home;
use handlebars::Handlebars;
use serde_json::json;
use common_lib::account::{Account};
use common_lib::db::DbMsg;
use common_lib::settings::Settings;

/// GET /account
pub async fn get_account(hb: web::Data<Handlebars<'_>>, tx_db: web::Data<crossbeam_channel::Sender<DbMsg>>, session: Session, ) -> impl Responder {

    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {

        // tracing::debug!("[main] ping db result: {:?}", tx_db.send(DbMsg::PingDb));

        // Turn the Arc<crossbeam_channel::Sender<DbMsg>> back into the channel we need after
        // Actix turns the channel into web::Data and wraps it in an Arc
        let tx_db = tx_db.into_inner().as_ref().clone();

        match Settings::load_with_secret(tx_db.clone()) {

            Ok(settings) => {

                // let account_result = Account::get_remote(&settings).await;

                let account_result= Account::get(tx_db.clone()).await;

                match account_result{
                    Ok(account)=>{
                        let equity = &account.equity; // BigDecimal::from_str(&account.equity).unwrap_or_else(|_| BigDecimal::from(0));
                        let diff_from_start = equity.clone() - settings.account_start_value.clone();

                        // tracing::debug!("[get_account] account_start: {}, equity: {}, diff: {}", &settings.account_start_value, &equity, &settings.account_start_value - &equity);

                        let data = json!({
                            "title": "Account",
                            "parent": "base0",
                            "is_logged_in": true,
                            "session_username": &session_username,
                            "message": account,
                            "diff_from_start": diff_from_start,
                        });
                        let body = hb.render("account", &data).unwrap();

                        HttpResponse::Ok().append_header(("cache-control", "no-store")).body(body)
                    },
                    Err(e)=>{
                        tracing::error!("[get_account] couldn't load account: {:?}",&e);
                        redirect_home().await
                    }
                }

            }
            Err(e) => {
                tracing::error!("[get_account] couldn't load settings (to get alpaca id/secret): {:?}",&e);
                redirect_home().await
            }
        }

    } else {
        redirect_home().await
    }
}
