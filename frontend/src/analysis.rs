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
        let tx_db_1 = tx_db.clone();
        let tx_db_2 = tx_db.clone();
        let tx_db_3 = tx_db.clone();

        // get both charts' data from database
        let chart_0_result = analysis_chart_net_profit(tx_db).await;
        let chart_1_result = analysis_chart_avg_profit(tx_db_1).await;
        let chart_2_result = chart_profit_daily(tx_db_2).await;
        let chart_3_result = chart_orders_daily(tx_db_3).await;

        if /*chart_0_result.is_ok() && chart_1_result.is_ok() &&*/ chart_2_result.is_ok() {

            let chart_0 = chart_0_result.unwrap();
            let chart_1 = chart_1_result.unwrap();
            let chart_2 = chart_2_result.unwrap();
            let chart_3 = chart_3_result.unwrap();

            // tracing::debug!("[get_analysis] db result: {:?}", &avg_profit);
            // TODO: remove unwrap
            let chart_0_columns = serde_json::to_string(&chart_0.columns).unwrap();
            let chart_0_data = serde_json::to_string(&chart_0.chart_data).unwrap();

            let chart_1_columns = serde_json::to_string(&chart_1.columns).unwrap();
            let chart_1_data = serde_json::to_string(&chart_1.chart_data).unwrap();

            let chart_2_columns = serde_json::to_string(&chart_2.columns).unwrap();
            let chart_2_data = serde_json::to_string(&chart_2.chart_data).unwrap();

            let chart_3_columns = serde_json::to_string(&chart_3.columns).unwrap();
            let chart_3_data = serde_json::to_string(&chart_3.chart_data).unwrap();

            let data = json!({
                "title": "Analysis",
                "parent": "base0",
                "is_logged_in": true,
                "session_username": &session_username,
                "chart_0_columns": chart_0_columns,
                "chart_0_data": chart_0_data,
                "chart_1_columns": chart_1_columns,
                "chart_1_data": chart_1_data,
                "chart_2_columns": chart_2_columns,
                "chart_2_data": chart_2_data,
                "chart_3_columns": chart_3_columns,
                "chart_3_data": chart_3_data,
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

/// get the data for the daily profit totals
/// TODO: all three of these chart functions are essentially duplicated code; convert to a enum/switch
async fn chart_profit_daily(tx_db:Sender<DbMsg>)->Result<Chart,TradeWebError>{
    let (sender, rx) = oneshot::channel();
    match tx_db.send(DbMsg::ChartProfitDaily {sender}){
        Err(e)=>{
            tracing::error!("[chart_profit_daily] error getting chart data: {:?}",&e);
            Err(TradeWebError::ChannelError)
        },
        Ok(_)=> {
            // send okay
            match rx.await {
                Err(e) => {
                    tracing::error!("[chart_profit_daily] receive error: {:?}",&e);
                    Err(TradeWebError::ChannelError)
                },
                Ok(chart_result) => {
                    Ok(chart_result)
                }
            }
        }
    }
}

/// get the data for the daily profit totals
/// TODO: all three of these chart functions are essentially duplicated code; convert to a enum/switch or macro
async fn chart_orders_daily(tx_db:Sender<DbMsg>)->Result<Chart,TradeWebError>{
    let (sender, rx) = oneshot::channel();
    match tx_db.send(DbMsg::ChartOrdersDaily {sender}){
        Err(e)=>{
            tracing::error!("[chart_orders_daily] error getting chart data: {:?}",&e);
            Err(TradeWebError::ChannelError)
        },
        Ok(_)=> {
            // send okay
            match rx.await {
                Err(e) => {
                    tracing::error!("[chart_orders_daily] receive error: {:?}",&e);
                    Err(TradeWebError::ChannelError)
                },
                Ok(chart_result) => {
                    Ok(chart_result)
                }
            }
        }
    }
}


