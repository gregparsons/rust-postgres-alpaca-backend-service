//! symbols.rs
//!
//! get, post a list of stock symbols and whether they're actively traded.
//!

use actix_session::Session;
use actix_web::web::Form;
use actix_web::{web, HttpResponse};
use bigdecimal::BigDecimal;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::http::redirect_home;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

/// POST /symbols
pub async fn post_symbols(form: Form<UiSymbol>, pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session: Session) -> HttpResponse {

    tracing::debug!("[post_symbols] form: {:?}", &form);

    // require login
    if let Ok(Some(_session_username)) = session.get::<String>(SESSION_USERNAME) {

        // price_last is fake here
        let form = form.into_inner();

        let ui_symbol = UiSymbol {
            symbol: form.symbol,
            active: form.active,
            trade_size: form.trade_size,
            price_last: BigDecimal::from(0)
        };

        // update t_symbol set active=true and trade_size=10 where symbol = 'arvl';

        let result_message = match ui_symbol.save(&pool).await {
            None => {
                // no problem return
                "Symbol change saved"
            }
            Some(e) => {
                tracing::debug!("[post_symbols] error saving symbol/active: {:?}", &e);
                "Error saving symbol"
            }
        };

        // render the GET page
        get_symbols_with_message(pool, hb, session, format!("{} [{}]", result_message, &ui_symbol.symbol).as_str(), ).await

    } else {
        // not logged in
        redirect_home().await
    }
}

/// GET /symbols
pub async fn get_symbols(pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session: Session ) -> HttpResponse {
    get_symbols_with_message(pool, hb, session, "").await
}


async fn get_symbols_with_message(pool: web::Data<PgPool>, hb: web::Data<Handlebars<'_>>, session: Session, message: &str, ) -> HttpResponse {

    // require login
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {

        let symbol_vec_result = UiSymbol::load(&pool).await;

        tracing::debug!("[get_symbols_with_message] symbols: \n{:?}", &symbol_vec_result);

        match symbol_vec_result {
            Ok(symbol_vec) => {
                let data = json!({
                    "title": "Symbols",
                    "parent": "base0",
                    "is_logged_in": true,
                    "session_username": &session_username,
                    "data": &symbol_vec,
                    "message": message,
                });
                let body = hb.render("symbol_table", &data).unwrap();
                HttpResponse::Ok()
                    .append_header(("cache-control", "no-store"))
                    .body(body)
            }
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

#[derive(Serialize, Deserialize, Debug)]
pub struct UiSymbol {
    symbol: String,
    active: bool,
    trade_size: BigDecimal,
    price_last: BigDecimal,
}

impl UiSymbol {

    /// get a vec of stock symbols
    /// TODO: make the coalesced trade_size the settings default size
    async fn load(pool: &PgPool) -> Result<Vec<UiSymbol>, sqlx::Error> {
        sqlx::query_as!(
            UiSymbol,
            r#"
                select
                    a.symbol as "symbol!"
                    , active as "active!"
                    ,coalesce(trade_size, 0.0) as "trade_size!"
                    ,coalesce(b.price,0.0) as "price_last!"
                from t_symbol a left join trade_alp_latest b on lower(a.symbol)=lower(b.symbol)
                order by a.symbol
            "#
        ).fetch_all(pool).await
    }

    /// save a change to the symbol's active status
    async fn save(&self, pool: &PgPool) -> Option<sqlx::Error> {
        match sqlx::query!(
            r#"
                update t_symbol set active = $2, trade_size = $3 where symbol = $1
            "#,
            self.symbol,
            self.active,
            self.trade_size
        ).execute(pool).await {

            Ok(_) => None,

            Err(e) => {
                tracing::debug!("[save_symbol_active] error saving symbol and active field: {:?}",&e);
                Some(e)
            }
        }
    }
}


