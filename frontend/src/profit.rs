//! profit.rs
//!

use actix_session::Session;
use actix_web::{web, HttpResponse};
use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDateTime, Utc};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use common_lib::common_structs::SESSION_USERNAME;
use common_lib::http::redirect_home;

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryProfit {
    pub symbol: String,
    pub profit_to_date: BigDecimal,
    pub activity_count: BigDecimal,
    pub count_today: BigDecimal,
    pub count_yesterday: BigDecimal,
    pub price_avg: BigDecimal,
    pub volume: BigDecimal,
    pub profit_vs_activities: BigDecimal,
    pub profit_vs_price: BigDecimal,
    pub profit_vs_volume: BigDecimal,
    pub trade_latest: NaiveDateTime,
    pub activity_latest: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryProfitTotal {
    pub profit: BigDecimal,
}

/// GET /profit
/// print a table of stocks P/L
pub async fn get_profit(
    hb: web::Data<Handlebars<'_>>,
    db_pool: web::Data<PgPool>,
    session: Session,
) -> HttpResponse {
    tracing::debug!("[get_profit]");

    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {
        tracing::debug!("session id: {}", &session_username);

        // exclamation point means we're overriding sqlx requiring Option<> on nullables (assuming we know it'll never be null)
        let profit_vec = match sqlx::query_as!(
            QueryProfit,
            r#"
                    select
                        *
                    from v_stats
                "#,
        )
        .fetch_all(db_pool.as_ref())
        .await
        {
            Ok(vec_of_profit) => vec_of_profit,

            Err(e) => {
                tracing::debug!("[get_profit] profit report error: {:?}", &e);
                vec![]
            }
        };

        let data = json!({
            "title": "Profit",
            "parent": "base0",
            "is_logged_in": true,
            "session_username": &session_username,
            "data": profit_vec,
        });

        let body = hb.render("profit_table", &data).unwrap();
        // tracing::debug!("[get_profit] body: {:?}", &body);
        HttpResponse::Ok()
            .append_header(("cache-control", "no-store"))
            .body(body)
    } else {
        redirect_home().await
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ProfitSummary {
    pub symbol: String,
    pub active: bool,
    pub closed_pl: BigDecimal,
    pub unrealized_pl: BigDecimal,
    pub unrealized_pl_avg: BigDecimal,
    pub posn_pl: BigDecimal,
    pub qty_buy_today: BigDecimal,
    pub qty_sell_today: BigDecimal,
    pub count_buy_activity_today: BigDecimal,
    pub count_sell_activity_today: BigDecimal,
    pub dtg_latest_buy: DateTime<Utc>,
}

/// GET /profit_summary
/// print a table of stocks P/L
pub async fn get_profit_summary(
    hb: web::Data<Handlebars<'_>>,
    db_pool: web::Data<PgPool>,
    session: Session,
) -> HttpResponse {
    tracing::debug!("[get_profit]");

    // require authentication (authorization tbd)
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {
        tracing::debug!("session id: {}", &session_username);

        // exclamation point means we're overriding sqlx requiring Option<> on nullables (assuming we know it'll never be null)
        let profit_vec = match sqlx::query_as!(
            ProfitSummary,
            r#"
                select
                    symbol as "symbol!"
                    ,active as "active!"
                    , closed_pl as "closed_pl!"
                    , unrealized_pl as "unrealized_pl!"
                    , posn_pl as "posn_pl!"
                    , unrealized_pl_avg as "unrealized_pl_avg!"
                    , qty_buy_today as "qty_buy_today!"
                    , qty_sell_today as "qty_sell_today!"
                    , count_buy_activity_today as "count_buy_activity_today!"
                    , count_sell_activity_today as "count_sell_activity_today!"
                    , dtg_latest_buy as "dtg_latest_buy!"
                from v_posn_activity_summary
            "#,
        ).fetch_all(db_pool.as_ref()).await {
            Ok(vec_of_profit) => vec_of_profit,
            Err(e) => {
                tracing::debug!("[get_profit] profit summary error: {:?}", &e);
                // send a blank vector to the web page on error; could send an error message if I cared about UI
                vec![]
            }
        };

        let data = json!({
            "title": "Profit",
            "parent": "base0",
            "is_logged_in": true,
            "session_username": &session_username,
            "data": profit_vec,
        });

        let body = hb.render("profit_summary", &data).unwrap();
        // tracing::debug!("[get_profit] body: {:?}", &body);
        HttpResponse::Ok()
            .append_header(("cache-control", "no-store"))
            .body(body)
    } else {
        redirect_home().await
    }
}
