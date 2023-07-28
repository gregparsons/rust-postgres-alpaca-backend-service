//! frontend/edit_positions.rs
//!
//! web form to edit settings

use actix_session::Session;
use actix_web::{web, HttpResponse};
use crossbeam_channel::Sender;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::http::redirect_home;
use handlebars::Handlebars;
use serde_json::json;
use common_lib::db::DbMsg;
use common_lib::position_local::PositionLocal;

/// GET /positions
pub async fn get_positions(
    tx_db: web::Data<Sender<DbMsg>>,
    hb: web::Data<Handlebars<'_>>,
    session: Session,
) -> HttpResponse {
    get_positions_with_message(tx_db, hb, session, "").await
}

/// Same as get_positions but displays a message above the list of positions
async fn get_positions_with_message(
    tx_db: web::Data<Sender<DbMsg>>,
    hb: web::Data<Handlebars<'_>>,
    session: Session,
    message: &str,
) -> HttpResponse {

    // require login
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {

        // get positions from the database (synced from Alpaca by the backend)
        let tx_db = tx_db.into_inner().as_ref().clone();
        let position_vec = PositionLocal::get_all(tx_db).await;

        // match position_vec_result {
        //     Ok(position_vec) => {
                let data = json!({
                    "title": "Positions",
                    "parent": "base0",
                    "is_logged_in": true,
                    "session_username": &session_username,
                    "data": &position_vec,
                    "message": message,
                });

                let body = hb.render("position", &data).unwrap();
                HttpResponse::Ok()
                    .append_header(("cache-control", "no-store"))
                    .body(body)
            // }
            // Err(e) => {
            //     // TODO: redirect to error message
            //     tracing::debug!("[get_positions] error getting symbols: {:?}", &e);
            //     redirect_home().await
            // }
        // }
    } else {
        redirect_home().await
    }
}
