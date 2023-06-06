//! web_client_service.rs
//!
//! Restful Alpaca Poller

use chrono::{Utc};
use tokio::runtime::Handle;
use common_lib::alpaca_activity::Activity;
use common_lib::alpaca_order::Order;
use common_lib::alpaca_position::Position;
use common_lib::market_hours::{MARKET_CLOSE_EXTENDED, MARKET_OPEN_EXTENDED};
use common_lib::settings::Settings;
use common_lib::sqlx_pool::create_sqlx_pg_pool;

/// Spawn a new thread to poll the Alpaca REST API
pub async fn run() {

    // run an async runtime inside the thread; it's a mess to try to run code copied from elsewhere
    // that normally runs async but is now running in a thread; much easier to just start a new
    // tokio runtime than to try to deal with FnOnce etc
    // people asking why you'd want to do this: https://stackoverflow.com/questions/61292425/how-to-run-an-asynchronous-task-from-a-non-main-thread-in-tokio/63434522#63434522

    let tokio_handle = Handle::current();
    let pool = create_sqlx_pg_pool().await;
    std::thread::spawn(move || {

        tracing::debug!("[run]");

        // this is set in all.sh via docker run
        let alpaca_poll_rate_ms: u64 = std::env::var("API_INTERVAL_MILLIS").unwrap_or_else(|_| "15000".to_string()).parse().unwrap_or(5000);
        let time_open_ny = MARKET_OPEN_EXTENDED.clone();
        let time_close_ny = MARKET_CLOSE_EXTENDED.clone();

        loop {

            let pool3 = pool.clone();

            // Call the API if the market is open in NYC
            let time_current_ny = Utc::now().with_timezone(&chrono_tz::America::New_York).time();

            if time_current_ny >= time_open_ny && time_current_ny <= time_close_ny {
                tracing::info!("[rest_service:start] NY time: {:?}, open: {:?}, close: {:?}", &time_current_ny, &time_open_ny, &time_close_ny);

                // Poll the activity API
                // https://stackoverflow.com/questions/61292425/how-to-run-an-asynchronous-task-from-a-non-main-thread-in-tokio/63434522#63434522
                tokio_handle.spawn( async move {
                    // refresh settings from the database
                    match Settings::load(&pool3).await {
                        Ok(settings)=>{

                            // update alpaca activities
                            match Activity::get_remote(&settings).await{
                                Ok(activities) => {
                                    tracing::debug!("[alpaca_activities] got activities: {}", activities.len());
                                    // save to postgres
                                    for a in activities {
                                        let _ = a.save_to_db(&pool3).await;
                                    }
                                },
                                Err(e) => {
                                    tracing::error!("[alpaca_activity] error: {:?}", &e);
                                }
                            }

                            // get alpaca positions
                            match Position::get_remote(&settings).await {

                                Ok(positions)=>{
                                    // clear out the database assuming the table will only hold what alpaca's showing as open orders
                                    match Position::delete_all_db(&pool3).await{
                                        Ok(_)=>tracing::debug!("[alpaca_position] positions cleared"),
                                        Err(e)=> tracing::error!("[alpaca_position] positions not cleared: {:?}", &e),
                                    }

                                    // save to postgres
                                    let now = Utc::now();
                                    for position in positions.iter() {
                                        let _ = position.save_to_db(now, &pool3).await;
                                    }
                                    tracing::debug!("[alpaca_position] updated positions at {:?}", &now);
                                },
                                Err(e) => {
                                    tracing::error!("[alpaca_position] could not load positions from Alpaca web API: {:?}", &e);
                                }
                            }

                            // get alpaca orders
                            match Order::get_remote(&settings).await {
                                Ok(orders) => {
                                    tracing::debug!("[alpaca_order] orders: {}", &orders.len());

                                    // clear out the database assuming the table will only hold what alpaca's showing as open orders
                                    match Order::delete_all_db(&pool3).await {
                                        Ok(_)=>tracing::debug!("[alpaca_order] orders cleared"),
                                        Err(e)=>tracing::error!("[alpaca_order] orders not cleared: {:?}", &e)
                                    }

                                    // save to postgres
                                    for order in orders.iter(){
                                        let _ = order.save_to_db(&pool3).await;
                                    }
                                },
                                Err(e)=>{
                                    tracing::error!("[alpaca_order] could not load orders from Alpaca web API: {:?}", &e);
                                }
                            }
                        },
                        Err(e) => {
                            tracing::error!("[run] couldn't load settings in loop to update activities/positions: {:?}", &e);
                        }
                    }
                });
            } else {
                tracing::debug!("[rest_service:start] market is closed. NY time: {:?}, open: {:?}, close: {:?}", &time_current_ny, &time_open_ny, &time_close_ny);
            }
            std::thread::sleep(std::time::Duration::from_millis(alpaca_poll_rate_ms));
        }
    });
}
