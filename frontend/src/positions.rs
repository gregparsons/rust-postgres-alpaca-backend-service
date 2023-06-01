//! frontend/edit_positions.rs
//!
//! web form to edit settings

use actix_session::Session;
use actix_web::{HttpResponse, web};
use handlebars::Handlebars;
use serde_json::json;
use sqlx::PgPool;
use common_lib::http::redirect_home;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::alpaca_position::Position;

/// GET /positions
pub async fn get_positions(pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session:Session) -> HttpResponse {
    get_positions_with_message(pool, hb, session, "").await
}

async fn get_positions_with_message(pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session:Session, message:&str)-> HttpResponse {

    // require login
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {

        // match Settings::load(&pool).await{
        //     Ok(_settings)=>{

                // get positions from the postgres database (already synced from Alpaca by the backend)
                let position_vec_result = Position::get_open_positions_from_db(&pool).await;

                // get positions from the Alpaca web api
                // let position_vec_result = Position::get_remote(&settings).await;

                match position_vec_result {
                    Ok(position_vec) => {

                        let data = json!({
                            "title": "Positions",
                            "parent": "base0",
                            "is_logged_in": true,
                            "session_username": &session_username,
                            "data": &position_vec,
                            "message": message,
                        });

                        let body = hb.render("position_table", &data).unwrap();
                        HttpResponse::Ok().append_header(("cache-control", "no-store")).body(body)

                    },
                    Err(e) => {
                        // TODO: redirect to error message
                        tracing::debug!("[get_positions] error getting symbols: {:?}", &e);
                        redirect_home().await
                    }
                // }

            // },
            // Err(e)=>{
            //     tracing::debug!("[get_positions] error getting settings from database: {:?}", &e);
            //     redirect_home().await
            // }
        }
    } else {
        redirect_home().await
    }

}