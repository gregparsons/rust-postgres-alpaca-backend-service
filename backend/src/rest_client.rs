//! rest_client
//!
//! Restful Alpaca Poller

use chrono::Utc;
use common_lib::alpaca_activity::Activity;
use common_lib::alpaca_order::Order;
use common_lib::alpaca_position::Position;
use common_lib::market_hours::{MARKET_CLOSE_EXTENDED, MARKET_OPEN_EXTENDED};
use common_lib::settings::Settings;
use common_lib::sqlx_pool::create_sqlx_pg_pool;
use tokio::runtime::Handle;

const REST_POLL_RATE_OPEN_MILLIS_STR: &str = "5000";
const REST_POLL_RATE_OPEN_MILLIS: u64 = 5000;
const REST_POLL_RATE_CLOSED_MILLIS: u64 = 30000;

/// Spawn a new thread to poll the Alpaca REST API
pub async fn run() {
    // run an async runtime inside the thread; it's a mess to try to run code copied from elsewhere
    // that normally runs async but is now running in a thread; much easier to just start a new
    // tokio runtime than to try to deal with FnOnce etc
    // people asking why you'd want to do this: https://stackoverflow.com/questions/61292425/how-to-run-an-asynchronous-task-from-a-non-main-thread-in-tokio/63434522#63434522

    let tokio_handle = Handle::current();
    let pool = create_sqlx_pg_pool().await;

    /// spawn the entire rest client into a new OS thread
    std::thread::spawn(move || {
        tracing::debug!("[run]");

        let mut alpaca_poll_rate_ms: u64;

        // this is set in all.sh via docker run
        let time_open_ny = MARKET_OPEN_EXTENDED.clone();
        let time_close_ny = MARKET_CLOSE_EXTENDED.clone();
        // Call the API if the market is open in NYC

        loop {
            let pool3 = pool.clone();

            let time_current_ny = Utc::now()
                .with_timezone(&chrono_tz::America::New_York)
                .time();
            alpaca_poll_rate_ms = {
                // if market is open, set the poll rate to the desired open rate

                // TODO: use new is_open() function
                if time_current_ny >= time_open_ny && time_current_ny <= time_close_ny {
                    tracing::info!(
                        "[rest_service:loop] NY time: {:?}, open: {:?}, close: {:?}",
                        &time_current_ny,
                        &time_open_ny,
                        &time_close_ny
                    );
                    std::env::var("API_INTERVAL_MILLIS")
                        .unwrap_or_else(|_| REST_POLL_RATE_OPEN_MILLIS_STR.to_string())
                        .parse()
                        .unwrap_or(REST_POLL_RATE_OPEN_MILLIS)
                } else {
                    // back off to a slower poll rate.
                    tracing::debug!("[rest_service:start] market is closed. NY time: {:?}, open: {:?}, close: {:?}", &time_current_ny, &time_open_ny, &time_close_ny);
                    // 30 seconds
                    REST_POLL_RATE_CLOSED_MILLIS
                }
            };

            // Poll the activity API; spawn into a new Tokio-managed thread because it's three web API calls that could take a while.
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

                        // Positions: sync from Alpaca
                        match Position::get_remote(&settings).await {

                            Ok(positions)=>{

                                // clear the database table
                                match Position::delete_all_db(&pool3).await{
                                    Ok(_)=>tracing::debug!("[alpaca_position] positions cleared"),
                                    Err(e)=> tracing::error!("[alpaca_position] positions not cleared: {:?}", &e),
                                }

                                // save to database
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
            std::thread::sleep(std::time::Duration::from_millis(alpaca_poll_rate_ms));
        }
    });
}
