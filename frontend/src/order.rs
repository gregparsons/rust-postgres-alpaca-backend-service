//! alpaca_order

use actix_session::Session;
use actix_web::{web, HttpResponse};
use crossbeam_channel::Sender;
use common_lib::alpaca_order::Order;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::http::redirect_home;
use common_lib::settings::Settings;
use handlebars::Handlebars;
use serde_json::json;
use common_lib::db::DbMsg;

/// GET /order
pub async fn get_order(
    tx_db: web::Data<Sender<DbMsg>>, hb: web::Data<Handlebars<'_>>, session: Session) -> HttpResponse {
    // require login
    tracing::debug!("[get_orders]");
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {

        // no need to bring in passwords here

        let tx_db = tx_db.into_inner().as_ref().clone();
        let tx_db1 = tx_db.clone();


        let setting_result = Settings::load_no_secret(tx_db1).await;

        match setting_result {
            Ok(settings) => {

                let tx_db2 = tx_db.clone();
                let orders = Order::local(tx_db2);

                let (message, orders) = match orders {
                    Ok(orders) => {
                        tracing::debug!("[get_orders] orders: {:?}", &orders);
                        ("Got orders".to_string(), orders)
                    }
                    Err(e) => {
                        tracing::debug!("[get_orders] db error getting orders: {:?}", &e);
                        tracing::debug!("[get_orders] error getting orders from db: {:?}", &e);

                        ("Error getting orders from database".to_string(), vec![])
                    }
                };

                let data = json!({
                    "title": "Settings",
                    "parent": "base0",
                    "is_logged_in": true,
                    "session_username": &session_username,
                    "data": &settings,
                    "message": message,
                    "orders": orders
                });

                let body = hb.render("order_table", &data).unwrap();
                HttpResponse::Ok()
                    .append_header(("cache-control", "no-store"))
                    .body(body)
            }
            Err(e) => {
                // TODO: redirect to error message
                tracing::error!("[get_settings] error getting settings: {:?}", &e);
                redirect_home().await
            }
        }
    } else {
        redirect_home().await
    }
}
