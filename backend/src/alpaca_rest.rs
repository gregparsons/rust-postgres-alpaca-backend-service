//! rest_client
//!
//! Restful Alpaca Poller

use chrono::Utc;
use crossbeam_channel::Sender;
use common_lib::alpaca_activity::Activity;
use common_lib::alpaca_position::Position;
use common_lib::market_hours::{MARKET_CLOSE_EXT, MARKET_OPEN_EXT};
use common_lib::settings::Settings;
use tokio::runtime::Handle;
use common_lib::account::Account;
use common_lib::alpaca_transaction_status::AlpacaTransaction;
use common_lib::db::DbMsg;

// see .env first
const REST_POLL_RATE_OPEN_MILLIS_STR: &str = "3000";
const REST_POLL_RATE_OPEN_MILLIS: u64 = 3000;
const REST_POLL_RATE_CLOSED_MILLIS: u64 = 10000;

// quickly disable pieces of the Alpaca API
const ENABLE_REST_ACTIVITY: bool = true;
const ENABLE_REST_POSITION: bool = true;
// don't need this
// const ENABLE_REST_ORDER: bool = false;
const ENABLE_REST_ACCOUNT: bool = true;



// ****** TODO: crossbeam tick insted of thread sleep



pub  struct AlpacaRest{}

impl AlpacaRest {

    /// Spawn a new thread to poll the Alpaca REST API
    pub fn run(tx_db_rest: Sender<DbMsg>, _tokio_handle: Handle) {

        tracing::debug!("[rest_client::run] starting alpaca rest client");

        tracing::debug!("[run] inside tokio spawn");

        // on every startup clean out the order slate; it'll refill from positions and the order process
        // minor possibility of an order already existing on Alpaca, but rare in the times we're restarting the app

        let tx_db_1 = tx_db_rest.clone();
        let _ = AlpacaTransaction::delete_all(tx_db_1);

        let mut alpaca_poll_rate_ms: u64;

        // this is set in all.sh via docker run
        let time_open_ny = MARKET_OPEN_EXT.clone();
        let time_close_ny = MARKET_CLOSE_EXT.clone();
        // Call the API if the market is open in NYC

        loop {

            // let pool3 = pool.clone();

            let time_current_ny = Utc::now().with_timezone(&chrono_tz::America::New_York).time();
            alpaca_poll_rate_ms = {
                // if market is open, set the poll rate to the desired open rate

                // TODO: use new is_open() function
                if time_current_ny >= time_open_ny && time_current_ny <= time_close_ny {
                    tracing::info!("[rest_service:loop] NY time: {:?}, open: {:?}, close: {:?}",&time_current_ny,&time_open_ny,&time_close_ny);
                    std::env::var("API_INTERVAL_MILLIS").unwrap_or_else(|_| REST_POLL_RATE_OPEN_MILLIS_STR.to_string()).parse().unwrap_or(REST_POLL_RATE_OPEN_MILLIS)

                } else {
                    // back off to a slower poll rate.
                    tracing::debug!("[run] market is closed. NY time: {:?}, open: {:?}, close: {:?}", &time_current_ny, &time_open_ny, &time_close_ny);
                    // 30 seconds
                    REST_POLL_RATE_CLOSED_MILLIS
                }
            };

            // refresh settings from the database
            let tx_db_1 = tx_db_rest.clone();
            let tx_db_2 = tx_db_rest.clone();
            match Settings::load_with_secret(tx_db_1) {
                Ok(settings) => {

                    tracing::debug!("[run] got settings, running rest API calls");

                    if ENABLE_REST_ACTIVITY {
                        AlpacaRest::load_activities(&settings, tx_db_2.clone());
                    }

                    if ENABLE_REST_POSITION {
                        AlpacaRest::load_positions(&settings, tx_db_2.clone());
                    }

                    // if ENABLE_REST_ORDER {
                    //     AlpacaRest::load_orders(&pool3, &settings).await;
                    // }
                    //
                    if ENABLE_REST_ACCOUNT{
                        Account::load_account(&settings, tx_db_2.clone());
                    }
                },
                Err(e) => {
                    tracing::error!("[run] couldn't load settings in loop to update activities/positions: {:?}", &e);
                }
            }

            tracing::debug!("[run] done");

            std::thread::sleep(std::time::Duration::from_millis(alpaca_poll_rate_ms));
            // tokio::time::sleep(Duration::from_secs(3)).await;

        }

    }



    /// load activities from the REST api and put them in the Postgres database; filter by the most
    /// recent activity timestamp
    fn load_activities(settings:&Settings, tx_db:crossbeam_channel::Sender<DbMsg>){

        // get latest activity timestamp from database (slow, not ideal, easier than extracting/manipulating in memory)

        let since_filter = match Activity::latest_dtg(tx_db.clone()){
            Ok(last_dtg) => Some(last_dtg),
            Err(_)=> None,
        };

        match Activity::get_remote(since_filter, &settings, tx_db.clone()) {
            Ok(activities) => {
                tracing::debug!("[alpaca_activities] got activities: {}", activities.len());
                // save to postgres
                for a in activities {
                    let _ = a.save_to_db(tx_db.clone());
                }
            },
            Err(e) => tracing::error!("[alpaca_activity] error: {:?}", &e),
        }
    }


    /// load positions from the REST api and put them in the Postgres database
    fn load_positions(settings:&Settings, tx_db: crossbeam_channel::Sender<DbMsg>){

        // Positions: sync from Alpaca

        match Position::get_remote(&settings, tx_db.clone()) {

            Ok(positions) => {

                // clear the database table
                Position::delete_all_db(tx_db.clone());

                // save to database

                for position in positions.iter() {

                    let _ = position.save_to_db(tx_db.clone());
                    AlpacaTransaction::insert_existing_position(&position, tx_db.clone());

                }
                tracing::debug!("[alpaca_position] updated positions");
            },
            Err(e) => tracing::error!("[alpaca_position] could not load positions from Alpaca web API: {:?}", &e),
        }
    }

    // /// load orders from the REST api and put them in the Postgres database
    // ///
    // /// TODO: convert to crossbeam, non-async
    // async fn load_orders(pool:&PgPool, settings:&Settings){
    //     // get alpaca orders
    //     match Order::get_remote(&settings).await {
    //         Ok(orders) => {
    //             tracing::debug!("[alpaca_order] orders: {}", &orders.len());
    //
    //             // clear out the database assuming the table will only hold what alpaca's showing as open orders
    //             match Order::delete_all_db(pool).await {
    //                 Ok(_) => tracing::debug!("[alpaca_order] orders cleared"),
    //                 Err(e) => tracing::error!("[alpaca_order] orders not cleared: {:?}", &e)
    //             }
    //
    //             // save to postgres
    //             for order in orders.iter() {
    //                 let _ = order.save_to_db(pool).await;
    //             }
    //         },
    //         Err(e) => tracing::error!("[alpaca_order] could not load orders from Alpaca web API: {:?}", &e),
    //     }
    // }

}



