//! order.rs

use actix_session::Session;
use actix_web::{HttpResponse, web};
use handlebars::Handlebars;
use serde_json::json;
use sqlx::PgPool;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::http::redirect_home;
use common_lib::order::Order;
use common_lib::settings::Settings;

pub async fn get_orders(pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session:Session) -> HttpResponse {
    // require login
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {
        let setting_result = Settings::load_no_secret(&pool).await;
        match setting_result {
            Ok(settings) => {

                let orders = Order::get_unfilled_orders_from_db(&pool).await;

                let (message, orders) = match orders {
                    Ok(orders)=>{
                        ("Got orders".to_string(), orders)
                    }
                    Err(e) => {
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
