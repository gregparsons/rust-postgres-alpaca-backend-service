//! analysis
//!

use actix_session::Session;
use actix_web::{web, HttpResponse};
use handlebars::Handlebars;
use serde_json::json;
use tokio::sync::oneshot;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::db::DbMsg;
use common_lib::http::redirect_home;


/// GET /profit
/// print a table of stocks P/L
pub async fn get_analysis(tx_db: web::Data<crossbeam_channel::Sender<DbMsg>>, hb: web::Data<Handlebars<'_>>, session: Session) -> HttpResponse {
    tracing::debug!("[get_profit]");

    // logged in?
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {

        tracing::debug!("[get_analysis] logged in with session id: {}", &session_username);

        let tx_db = tx_db.into_inner().as_ref().clone();

        let (sender, rx) = oneshot::channel();
        // match tx_db.send(DbMsg::AnalysisDailyProfitJson_OLD {sender}){
        match tx_db.send(DbMsg::AnalysisProfitDailyChart {sender}){
            Err(e)=>{
                tracing::error!("[get_analysis] error getting chart data: {:?}",&e);
                // String::new()
                redirect_home().await
            },
            Ok(_)=>{
                // send okay
                match rx.await{
                    Err(e)=>{
                        tracing::error!("[get_analysis] receive error: {:?}",&e);
                        // String::new()
                        redirect_home().await
                    },
                    Ok(chart_result)=>{

                        tracing::debug!("[get_analysis] db result: {:?}", &chart_result);
                        // TODO: remove unwrap
                        let chart_columns = serde_json::to_string(&chart_result.columns).unwrap();
                        tracing::debug!("[get_analysis] chart_columns: {}", &chart_columns);

                        let chart_data = serde_json::to_string(&chart_result.chart_data).unwrap();
                        tracing::debug!("[get_analysis] chart_data: {}", &chart_data);


                        let data = json!({
                            "title": "Analysis",
                            "parent": "base0",
                            "is_logged_in": true,
                            "session_username": &session_username,
                            "chart_columns": chart_columns,
                            "chart_data": chart_data,
                        });

                        let body = hb.render("analysis", &data).unwrap();
                        HttpResponse::Ok().append_header(("cache-control", "no-store")).body(body)
                    }
                }
            }
        }

    } else {
        tracing::info!("[get_analysis] not logged in redirecting home");
        redirect_home().await
    }
}
