//! analysis
//!

use actix_session::Session;
use actix_web::{web, HttpResponse};
use crossbeam_channel::Sender;
use handlebars::Handlebars;
use serde_json::json;
use tokio::sync::oneshot;
use common_lib::common_structs::SESSION_USERNAME;
use common_lib::db::{Chart, DbMsg};
use common_lib::error::TradeWebError;
use common_lib::http::redirect_home;


/// GET /profit
/// print a table of stocks P/L
pub async fn get_analysis(tx_db: web::Data<crossbeam_channel::Sender<DbMsg>>, hb: web::Data<Handlebars<'_>>, session: Session) -> HttpResponse {
    tracing::debug!("[get_profit]");

    // logged in?
    if let Ok(Some(session_username)) = session.get::<String>(SESSION_USERNAME) {

        tracing::debug!("[get_analysis] logged in with session id: {}", &session_username);

        let tx_db = tx_db.into_inner().as_ref().clone();
        let tx_db1 = tx_db.clone();

        // get both charts' data from database
        let result_net = analysis_chart_net_profit(tx_db).await;
        let result_avg = analysis_chart_avg_profit(tx_db1).await;

        if result_net.is_ok() && result_avg.is_ok() {

            let net_profit = result_net.unwrap();
            let avg_profit = result_avg.unwrap();

            // tracing::debug!("[get_analysis] db result: {:?}", &avg_profit);
            // TODO: remove unwrap
            let chart_0_columns = serde_json::to_string(&net_profit.columns).unwrap();
            // tracing::debug!("[get_analysis] chart_columns: {}", &chart_0_columns);
            let chart_0_data = serde_json::to_string(&net_profit.chart_data).unwrap();
            // tracing::debug!("[get_analysis] chart_data: {}", &chart_0_data);

            let chart_1_columns = serde_json::to_string(&avg_profit.columns).unwrap();
            let chart_1_data = serde_json::to_string(&avg_profit.chart_data).unwrap();


            let data = json!({
                "title": "Analysis",
                "parent": "base0",
                "is_logged_in": true,
                "session_username": &session_username,
                "chart_0_columns": chart_0_columns,
                "chart_0_data": chart_0_data,
                "chart_1_columns": chart_1_columns,
                "chart_1_data": chart_1_data,
            });

            let body = hb.render("analysis", &data).unwrap();
            HttpResponse::Ok().append_header(("cache-control", "no-store")).body(body)

        } else {
            // TODO: figure out how to do a match with two results
            tracing::info!("[get_analysis] database error getting chart data");
            redirect_home().await
        }
    } else {
        tracing::info!("[get_analysis] not logged in redirecting home");
        redirect_home().await
    }
}

async fn analysis_chart_net_profit(tx_db:Sender<DbMsg>)->Result<Chart,TradeWebError>{
    let (sender, rx) = oneshot::channel();
    match tx_db.send(DbMsg::AnalysisProfitNetChart {sender}){
        Err(e)=>{
            tracing::error!("[analysis_chart_net_profit] error getting chart data: {:?}",&e);
            Err(TradeWebError::ChannelError)
        },
        Ok(_)=> {
            // send okay
            match rx.await {
                Err(e) => {
                    tracing::error!("[analysis_chart_net_profit] receive error: {:?}",&e);
                    Err(TradeWebError::ChannelError)
                },
                Ok(chart_result) => {
                    Ok(chart_result)
                }
            }
        }
    }
}

async fn analysis_chart_avg_profit(tx_db:Sender<DbMsg>)->Result<Chart,TradeWebError>{
    let (sender, rx) = oneshot::channel();
    match tx_db.send(DbMsg::AnalysisProfitAvgChart {sender}){
        Err(e)=>{
            tracing::error!("[analysis_chart_avg_profit] error getting chart data: {:?}",&e);
            Err(TradeWebError::ChannelError)
        },
        Ok(_)=> {
            // send okay
            match rx.await {
                Err(e) => {
                    tracing::error!("[analysis_chart_avg_profit] receive error: {:?}",&e);
                    Err(TradeWebError::ChannelError)
                },
                Ok(chart_result) => {
                    Ok(chart_result)
                }
            }
        }
    }
}



// async fn analysis_chart(chart_mesg: DbMsg,tx_db:Sender<DbMsg>)->Result<Chart,TradeWebError>{
//     let (sender, rx) = oneshot::channel();
//     match tx_db.send(chart_mesg){
//         Err(e)=>{
//             tracing::error!("[analysis_chart_net_profit] error getting chart data: {:?}",&e);
//             Err(TradeWebError::ChannelError)
//         },
//         Ok(_)=> {
//             // send okay
//             match rx.await {
//                 Err(e) => {
//                     tracing::error!("[analysis_chart_net_profit] receive error: {:?}",&e);
//                     Err(TradeWebError::ChannelError)
//                 },
//                 Ok(chart_result) => {
//                     Ok(chart_result)
//                 }
//             }
//         }
//     }
// }