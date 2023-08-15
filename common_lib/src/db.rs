//! db.rs
//!
//! start() spawns a long-running thread to maintain an open connection to a database pool
//!

// common imports
use bigdecimal::BigDecimal;
use sqlx::postgres::PgQueryResult;
use sqlx::{Error, PgPool};
use chrono::{DateTime, Utc};
use crossbeam_channel::Sender;
use reqwest::header::HeaderMap;
use tokio::runtime::Handle;
use tokio::sync::oneshot;
use crate::settings::Settings;

// trade imports
// use crate::account::{Account, AccountWithDate};
// use crate::alpaca_activity::{Activity, ActivityLatest};
// use crate::alpaca_api_structs::{AlpacaTradeWs, MinuteBar, Ping};
// use crate::alpaca_order::Order;
// use crate::alpaca_order_log::AlpacaOrderLogEvent;
// use crate::alpaca_position::{Position, TempPosition};
// use crate::error::TradeWebError;
// use crate::finnhub::{FinnhubPing, FinnhubTrade};
// use crate::symbol_list::QrySymbol;
// use crate::trade_struct::TradeSide;
// use crate::alpaca_transaction_status::*;
// use crate::sqlx_pool::create_sqlx_pg_pool;

// trade imports
use crate::account::{Account, AccountWithDate};
use crate::alpaca_activity::{Activity, ActivityLatest, ActivityQuery};
use crate::alpaca_api_structs::{AlpacaTradeWs, MinuteBar, Ping};
use crate::alpaca_order::Order;
use crate::alpaca_order_log::AlpacaOrderLogEvent;
use crate::alpaca_position::{Position, TempPosition};
use crate::error::{PollerError, TradeWebError};
use crate::finnhub::{FinnhubPing, FinnhubTrade};
use crate::symbol_list::QrySymbol;
use crate::trade_struct::{JsonTrade, OrderType, TimeInForce, TradeSide};
use crate::alpaca_transaction_status::*;
use crate::dashboard::Dashboard;
use crate::diff_calc::DiffCalc;
use crate::order_log_entry::OrderLogEntry;
use crate::position_local::PositionLocal;
use crate::sell_position::SellPosition;
use crate::sqlx_pool::create_sqlx_pg_pool;
use crate::symbol::Symbol;

#[derive(Debug)]
pub enum DbMsg {
    PingDb,
    // LastTrade(AlpacaTradeRest),
    TradeAlpaca(AlpacaTradeWs),
    // WsQuote(AlpWsQuote),
    MinuteBar(MinuteBar),
    TradeFinnhub(FinnhubTrade),
    PingFinnhub(FinnhubPing),
    PingAlpaca(Ping),
    RefreshRating,
    OrderLogEvent(AlpacaOrderLogEvent),

    SettingsWithSecret { sender: oneshot::Sender<Settings> },
    SettingsNoSecret { sender: oneshot::Sender<Settings> },

    TransactionDeleteOne{ symbol:String },
    TransactionDeleteAll,
    AccountGet{ resp_tx: Sender<AccountWithDate> },
    // TODO: pass settings to others that'd need updated settings while running
    AccountGetRemote{ settings:Settings, resp_tx: Sender<Account> },
    AccountSaveToDb{ account:Account},
    AcctCashAvailable { symbol:String, sender_tx: oneshot::Sender<MaxBuyPossible> },
    AcctDashboardGet{ sender:oneshot::Sender<Vec<Dashboard>>},

    TransactionInsertPosition { position:Position },
    TransactionStartBuy { symbol:String, sender: oneshot::Sender<BuyResult>},
    TransactionStartSell { symbol:String},


    ActivityLatestDtg{ resp_tx: Sender<DateTime<Utc>> },
    ActivityGetRemote{ since_option:Option<DateTime<Utc>>, settings:Settings, resp_tx: Sender<Vec<Activity>>},
    ActivitySaveToDb { activity:Activity },
    ActivityGetAll{sender:oneshot::Sender<Vec<ActivityQuery>>},
    ActivityGetForSymbol { symbol:String, sender:oneshot::Sender<Vec<ActivityQuery>>},

    SymbolListGetActive {sender_tx: Sender<Vec<String>> },
    SymbolListGetAll {sender_tx: Sender<Vec<String>> },
    SymbolLoadOne{symbol:String, sender_tx: Sender<Symbol> },

    OrderSave{ order:Order },
    OrderLocal{ sender_tx: Sender<Vec<Order>> },

    PositionGetRemote{ settings:Settings, resp_tx: Sender<Vec<Position>> },
    PositionDeleteAll,
    PositionSaveToDb { position:Position },
    PositionListShowingProfit{ pl_filter: BigDecimal, sender_tx: Sender<Vec<SellPosition>>},
    PositionListShowingAge{sender_tx: Sender<Vec<SellPosition>>},

    OrderLogEntrySave{entry: OrderLogEntry},

    DiffCalcGet{ sender: Sender<Result<Vec<DiffCalc>, PollerError>> },

    RestPostOrder{ json_trade:JsonTrade, settings:Settings, sender: Sender<Order> },

    PositionLocalGet{sender: oneshot::Sender<Vec<PositionLocal>>},

    WebsocketAlpacaAlive{sender: oneshot::Sender<bool>},

}

#[derive(Debug)]
pub struct MaxBuyPossible {
    pub price:BigDecimal,
    pub size:BigDecimal,
    pub cash_available:BigDecimal,
    pub qty_possible:BigDecimal

}

pub struct DbActor {
    pub pool: PgPool,
    pub tx:crossbeam_channel::Sender<DbMsg>,
    pub rx:crossbeam_channel::Receiver<DbMsg>,
}

impl DbActor {

    pub async fn new() -> DbActor{
        tracing::debug!("[new]");
        let pool = create_sqlx_pg_pool().await;
        let (tx, rx) = crossbeam::channel::unbounded();
        DbActor{ pool, tx, rx, }
    }

    /// DB Listener
    ///
    /// Other threads can send DbMsg messages via crossbeam to perform inserts into the database cross-thread.
    ///
    /// Each db network call takes 150-300ms on LAN/wifi
    pub fn run(&self, rt: Handle) /*-> JoinHandle<()> */{

        let rx = self.rx.clone();
        let pool = self.pool.clone();

        tracing::debug!("[run]");

        loop {

            // tracing::debug!("[run] loop");

            let pool = pool.clone();
            crossbeam::channel::select! {

                // TODO: potentially handle a separate channel that controls startup and shutdown of this db actor
                // https://ryhl.io/blog/actors-with-tokio/

                recv(rx) -> result => {
                    match result {
                        Ok(msg)=>{
                            // tracing::debug!("[run] msg: {:?}", &msg);

                            // blocking not required for sqlx and reqwest async libraries
                            let _x = rt.spawn(async {
                                tracing::debug!("[run] tokio spawn process_message: {:?}", &msg);
                                process_message(msg, pool).await;
                            });

                        },
                        Err(e)=>{
                            tracing::error!("[run] select error: {:?}", &e);
                        }
                    }
                },
                default=>{
                    // potentially do something if no message is coming in, like check a
                    // semaphore to shutdown, for example.
                    // tracing::debug!("[run] nothing to receive");
                }
            }
        }
    }
}

async fn process_message(msg:DbMsg, pool: PgPool){

    // this will dump settings (and API keys) to the log
    // tracing::debug!("[process_message] DbMsg: {:?}", &msg);

    match msg {

        DbMsg::WebsocketAlpacaAlive{sender}=>{
            match websocket_alpaca_alive(pool).await{
                Ok(alpaca_alive)=>{
                    match sender.send(alpaca_alive){
                        Ok(_)=>{},
                        Err(e)=> tracing::error!("[WebsocketAlpacaAlive] send error: {:?}", &e),
                    }
                },
                Err(e) => tracing::error!("[WebsocketAlpacaAlive] {:?}", &e),
            }
        },

        DbMsg::AcctDashboardGet {sender}=>{
            let vec_dashboard = acct_dashboard(pool).await;
            let _ = sender.send(vec_dashboard);
        },
        DbMsg::ActivityGetAll{ sender}=>{
            let vec_activity = match activities_from_db(pool).await{
                Ok(vec)=>vec,
                Err(e)=>{
                    tracing::error!("[process_message::ActivityGetAll] error: {:?}", &e);
                    vec!()
                }
            };
            let _ = sender.send(vec_activity);
        }
        DbMsg::ActivityGetForSymbol{symbol, sender}=>{
            let vec_activity = match activities_from_db_for_symbol(&symbol, pool).await{
                Ok(vec)=>vec,
                Err(e)=>{
                    tracing::error!("[process_message::ActivityGetForSymbol] error: {:?}", &e);
                    vec!()
                }
            };
            let _ = sender.send(vec_activity);
        }

        // positions for display in the web UI
        DbMsg::PositionLocalGet{sender}=>{
            match position_local_get(pool).await{
                Ok(positions)=>sender.send(positions).unwrap(),
                Err(_e)=>{ } // already reported
            }
        }


        // TODO: move to a REST handler, not the database
        DbMsg::RestPostOrder {json_trade, settings, sender}=>{
            match rest_post_order(&json_trade, &settings).await{
                Ok(order)=>{
                    let _ = sender.send(order);
                },
                Err(_e)=>{
                    //
                }
            }
        },

        DbMsg::DiffCalcGet {sender} => {
            let result = diffcalc_get(pool).await;
            match sender.send(result){
                Ok(_)=>{
                    // sent reply okay
                },
                Err(e)=>{
                    tracing::error!("[process_message][DbMsg::DiffCalcGet] send error: {:?}", &e);
                }
            }
        }

        DbMsg::OrderLogEntrySave{entry}=>{
            let _ = order_log_entry_save(entry, pool).await;
        }

        DbMsg::PingDb=>{
            tracing::debug!("[db_thread][DbMsg::PingDb] pong: db thread here!");
        },

        DbMsg::OrderSave{ order }=>{
            let _ = order_save(order, pool).await;
        },

        DbMsg::SettingsWithSecret { sender: sender_tx }=>{
            match settings_load_with_secret(pool).await {
                Ok(settings) => { let _ = sender_tx.send(settings); },
                Err(_e)=> tracing::debug!("[db] received DbMsg::GetSettingsWithSecret error: {:?}", &_e),
            }
        },

        DbMsg::SettingsNoSecret { sender: sender_tx }=>{
            tracing::debug!("[db] received DbMsg::SettingsNoSecret");
            match settings_load_no_secret(pool).await {
                Ok(settings) => { let _ = sender_tx.send(settings); },
                Err(_e)=> tracing::debug!("[db] received DbMsg::GetSettingsWithSecret error: {:?}", &_e),
            }
        },

        DbMsg::ActivitySaveToDb{activity} => {
            let _ = activity_save_to_db(&activity, pool).await;
        },

        DbMsg::ActivityLatestDtg{resp_tx}=>{
            if let Ok(activity) = activity_latest_dtg(pool).await {
                let _ = resp_tx.send(activity);
            }
        },

        DbMsg::ActivityGetRemote{ since_option, settings, resp_tx}=>{
            if let Ok(activities) = activity_get_remote(since_option, settings).await{
                let _ = resp_tx.send(activities);
            }
        },

        DbMsg::AcctCashAvailable { symbol, sender_tx }=>{
            match acct_cash_available(&symbol, pool).await {
                Ok(cash_available)=>{
                    let _ = sender_tx.send(cash_available);
                }
                Err(_e)=>tracing::error!("[DbMsg::AccountAvailableCash] couldn't get cash available from database: {:?}", &_e),
            }

        }

        DbMsg::AccountGet{resp_tx} =>{
            let result = account_get(&pool).await;
            if let Ok(account) = result {
                let _ = resp_tx.send(account);
            }
        },

        DbMsg::AccountGetRemote{settings, resp_tx} =>{
            let account_result = account_get_remote(&settings).await;
            if let Ok(account) = account_result {
                let _ = resp_tx.send(account);
            }
        },
        DbMsg::PositionGetRemote{settings, resp_tx} =>{
            let result = position_get_remote(&settings).await;
            if let Ok(positions) = result {
                let _ = resp_tx.send(positions);
            }
        },

        DbMsg::PositionDeleteAll =>{
            let _ = position_delete_all(&pool).await;
        },

        DbMsg::PositionSaveToDb{ position }=>{
            let _ = position_save_to_db(&position, &pool).await;
        },

        DbMsg::PositionListShowingProfit{ pl_filter, sender_tx} => {
            if let Ok(position_list_showing_profit) = position_list_showing_profit(pl_filter, pool).await {
                match sender_tx.send(position_list_showing_profit){
                    Ok(_)=>{},
                    Err(e)=>{
                        tracing::error!("[db][PositionListShowingProfit] reply send error: {:?}", &e);
                    }
                };
            }
        }

        DbMsg::PositionListShowingAge{ sender_tx} => {
            if let Ok(position_list_showing_age) = position_list_showing_age(pool).await {
                match sender_tx.send(position_list_showing_age){
                    Ok(_)=>{},
                    Err(e)=>{
                        tracing::error!("[db][PositionListShowingAge] reply send error: {:?}", &e);
                    }
                };
            }
        }

        DbMsg::TransactionInsertPosition{ position }=>{
            let _ = transaction_insert_position(&position, &pool).await;
        },

        // sender:oneshot::Sender<Result<(), TransactionError>>
        DbMsg::TransactionStartBuy { symbol, sender }=>{
            let is_transaction_allowed = transaction_start_buy(symbol.as_str(), pool).await;
            let _ = sender.send(is_transaction_allowed);
        }

        // /// not currently used
        // DbMsg::TransactionStartSell { symbol }=>{
        //     let result = transaction_start_sell(symbol.as_str(), pool).await;
        //
        // }


        DbMsg::AccountSaveToDb{ account }=>{
            let _ = account_save_to_db(&account, &pool).await;
        },


        DbMsg::SymbolListGetActive {sender_tx}=>{
            let result = symbols_get_active(pool).await;
            if let Ok(symbols) = result {
                let _ = sender_tx.send(symbols);
            }
        },

        DbMsg::SymbolListGetAll {sender_tx}=>{
            let result = symbols_get_all(pool).await;
            if let Ok(symbols) = result {
                let _ = sender_tx.send(symbols);
            }
        },

        DbMsg::SymbolLoadOne{symbol, sender_tx }=>{

            if let Ok(symbol) = symbol_load_one(symbol.as_str(), pool).await{
                let _ = sender_tx.send(symbol);
            }
        },

        DbMsg::TransactionDeleteAll=>{
            let _ = transaction_delete_all(&pool).await;
        },

        DbMsg::TransactionDeleteOne{symbol} => {
            let _ = transaction_delete_one(&symbol, pool).await;
        },


        DbMsg::TradeAlpaca(t) => {
            tracing::debug!("[db_thread, DbMsg::WsTrade] trade: {:?}", &t);

            // old; uses tokio::postgres w/o a connection pool
            // crate::db::insert_alpaca_websocket_trade(&client, &t).await;

            match insert_alpaca_trade(&t, &pool).await{
                Ok(_)=> tracing::debug!("[db_thread, DbMsg::WsTrade] alpaca trade inserted"),
                Err(e) => tracing::debug!("[db_thread, DbMsg::WsTrade] alpaca trade not inserted: {:?}", &e),
            }

            // also save a copy to the latest table; saves .5-3 seconds on lookup
            match insert_alpaca_trade_latest(&t, &pool).await {
                Ok(_)=> tracing::debug!("[db_thread, DbMsg::WsTrade] latest alpaca trade inserted"),
                Err(e) => tracing::debug!("[db_thread, DbMsg::WsTrade] latest alpaca trade not inserted: {:?}", &e),
            }
        },

        DbMsg::OrderLocal{ sender_tx }=>{

            if let Ok(result) = order_local(pool).await{
                let _ = sender_tx.send(result);
            }

        },

        DbMsg::OrderLogEvent(event)=>{
            tracing::debug!("[db] received DbMsg::OrderLogEvent: {:?}", &event);
            let _ = insert_order_log_entry(&event, &pool).await;

            // update the alpaca_transaction_status table when a sale happens
            if event.event=="fill" && event.order.side== TradeSide::Sell {
                tracing::info!("[DbMsg::OrderLogEvent][Fill][Sell] {}:{:?}", &event.order.symbol, &event.order.filled_qty);
                if let Some(qty) = event.order.filled_qty{
                    let _ = AlpacaTransaction::decrement(&event.order.symbol.clone(), qty, &pool).await;
                    let _ = AlpacaTransaction::clean(&pool).await;
                }
            }
        },

        DbMsg::RefreshRating => {
            tracing::debug!("[db] received DbMsg::RefreshRating");
            refresh_rating(&pool).await;
        },

        DbMsg::TradeFinnhub(finnhub_trade) => {

            // tracing::debug!("[db_thread, DbMsg::FhTrade] finnhub_trade received by db thread: {:?}", &finnhub_trade);
            match insert_finnhub_trade(&finnhub_trade, &pool).await {
                Ok(_)=> tracing::debug!("[db_thread, DbMsg::FhTrade] finnhub trade inserted"),
                Err(e) => tracing::debug!("[db_thread, DbMsg::FhTrade] finnhub trade not inserted: {:?}", &e),
            }

            // also save a copy to the latest table; saves .5-3 seconds on lookup (could also just save it in memory
            // client-side later when I convert to using RabbitMq
            match insert_finnhub_trade_latest(&finnhub_trade, &pool).await {
                Ok(_)=> tracing::debug!("[db_thread, DbMsg::FhTrade] finnhub trade inserted"),
                Err(e) => tracing::debug!("[db_thread, DbMsg::FhTrade] finnhub trade not inserted: {:?}", &e),
            }
        },

        DbMsg::PingFinnhub(ping) => {
            let _ = insert_finnhub_ping(&ping, &pool).await;

        },

        DbMsg::PingAlpaca(ping) => {
            let _ = insert_alpaca_ping(&ping, &pool).await;
        },

        _ => {
            tracing::error!("[db] received unrecognized DbMsg");

        }
    }
}


// ///
// /// TODO: convert to sqlx
// /// - convert to using migrations
// /// - PgConnectOptions would let us load DB credentials from env nicely without having to generate a
// /// connection string
// /// - there's currently no pool; sqlx makes that pretty easy
// ///
// ///
// async fn db_connect(db_url: &str) -> tokio_postgres::Client {
//     // no need to log the db password
//     // tracing::debug!("[db_connect] db_url: {}", &db_url);
//
//     let (client, connection) = tokio_postgres::connect(db_url, tokio_postgres::NoTls).await.unwrap();
//
//     // spin off the database connection to its own thread
//     tokio::spawn(async move {
//         // https://docs.rs/tokio-postgres/0.6.0/tokio_postgres/struct.Connection.html
//         // "Connection implements Future, and only resolves when the connection is closed, either
//         // because a fatal error has occurred, or because its associated Client has dropped and all
//         // outstanding work has completed."
//         if let Err(e) = connection.await {
//             tracing::debug!("postgres connection closed: {}", e);
//         }
//     });
//     tracing::debug!("[db_connect] connected");
//     client
// }

/// call an SQL function to poll the recent
async fn refresh_rating(pool:&PgPool){
    tracing::debug!("[refresh_rating] starting...");
    match sqlx::query!(r#"select * from fn_grade_stocks()"#).execute(pool).await{
        Ok(_)=>{
            tracing::info!("[refresh_rating] refresh done");
        },
        Err(e)=>{
            tracing::error!("[refresh_rating] refresh failed: {:?}", &e);

        }
    }

}

//
// /// TODO: convert this to SQLX; it currently does not use a connection pool
// async fn insert_minute_bar(client: &Client, mb: &MinuteBar) {
//     tracing::debug!("");
//
//     let sql = format!(
//         r"
// 		insert into bar_minute(
// 			dtg,
// 			symbol,
// 			price_open,
// 			price_high,
// 			price_low,
// 			price_close,
// 			volume
// 		)
// 		values ('{}','{}',{},{},{},{},{});",
//         mb.dtg, mb.symbol, mb.price_open, mb.price_high, mb.price_low, mb.price_close, mb.volume
//     );
//
//     tracing::debug!("[insert_minute_bar] sql: {}", &sql);
//
//     // run query
//     if let Ok(result_vec) = client.simple_query(&sql).await {
//         for i in result_vec {
//             match i {
//                 SimpleQueryMessage::CommandComplete(row_count) => {
//                     tracing::debug!("[insert_minute_bar] {} row(s) inserted", row_count);
//                 }
//
//                 SimpleQueryMessage::Row(_row) => {}
//                 _ => tracing::debug!("[insert_minute_bar] Something weird happened on log query."),
//             }
//         }
//     } else {
//         tracing::debug!("[insert_minute_bar] insert failed");
//     }
// }

/// insert a single FinnHub trade into the trade_fh table
async fn insert_finnhub_trade(trade: &FinnhubTrade, pool: &PgPool) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query!(
        r#"
            insert into trade_fh (dtg,symbol, price, volume) values ($1, lower($2), $3, $4)
        "#,
        trade.dtg.naive_utc(),
        trade.symbol.to_lowercase(),
        trade.price,
        trade.volume
    )
        .execute(pool)
        .await
}

/// Continuously overwrite only the latest trade for a given symbol so we have a fast way of getting the most recent price.
async fn insert_finnhub_trade_latest(
    trade: &FinnhubTrade,
    pool: &PgPool,
) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query!(
        r#"
            insert into trade_fh_latest (dtg, symbol, price, volume)
            values ($1, lower($2), $3, $4)
            on conflict (symbol) do update set dtg=$1, price=$3, volume=$4;
        "#,
        trade.dtg.naive_utc(),
        trade.symbol,
        trade.price,
        trade.volume
    )
        .execute(pool)
        .await
}

/// Insert an Alpaca trade received on the websocket
async fn insert_alpaca_trade(t: &AlpacaTradeWs, pool: &PgPool) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query!(
        r#"
            insert into trade_alp (dtg, symbol, price, size)
            values ($1, lower($2), $3, $4)
        "#,
        t.dtg.naive_utc(),t.symbol,t.price,t.size
    ).execute(pool).await
}

/// Continuously overwrite only the latest trade for a given symbol so we have a fast way of getting the most recent price.
async fn insert_alpaca_trade_latest(trade: &AlpacaTradeWs, pool: &PgPool) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query!(
        r#"insert into trade_alp_latest(dtg,symbol,price,size)
            values ($1, lower($2), $3, $4)
            on conflict (symbol) do update set dtg=$1, price=$3, size=$4;
        "#,
        trade.dtg.naive_utc(),
        trade.symbol,
        trade.price,
        trade.size
    ).execute(pool).await
}

/// Append a timestamp to the ping table whenever the Finnhub websocket pings. Use it to determine if the websocket goes down.
async fn insert_finnhub_ping(ping: &FinnhubPing, pool: &PgPool) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query!(
        r#"insert into ping_finnhub (ping) values ($1)"#,
        ping.dtg.naive_utc()
    ).execute(pool).await
}

/// Append a timestamp to the ping table whenever the Finnhub websocket pings. Use it to determine if the websocket goes down.
async fn insert_alpaca_ping(ping: &Ping, pool: &PgPool, ) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query!(
        r#"insert into ping_alpaca (ping) values (now())"#,
        // r#"insert into ping_alpaca (ping) values ($1)"#,
        // ping.dtg.naive_utc()
    ).execute(pool).await
}



async fn transaction_delete_all(pool:&PgPool) -> Result<(), TradeWebError> {
    match sqlx::query!(
            r#"
                delete from alpaca_transaction_status
            "#
        ).execute(pool).await{
        Ok(_)=>Ok(()),
        Err(_e)=>Err(TradeWebError::DeleteFailed), // or db error
    }
}

async fn account_get(pool:&PgPool) -> Result<AccountWithDate, TradeWebError> {
    match sqlx::query_as!(AccountWithDate, r#"
            select
                dtg as "dtg!"
                ,cash as "cash!"
                ,position_market_value as "position_market_value!"
                ,equity as "equity!"
                ,last_equity as "last_equity!"
                ,daytrade_count as "daytrade_count!"
                ,balance_asof as "balance_asof!"
                ,pattern_day_trader as "pattern_day_trader!"
                ,id as "id!"
                ,account_number as "account_number!"
                ,status as "status!"
                -- crypto_status as "!"
                ,currency as "currency!"
                            ,buying_power as "buying_power!"
                ,regt_buying_power as "regt_buying_power!"
                ,daytrading_buying_power as "daytrading_buying_power!"
                ,effective_buying_power as "effective_buying_power!"
                ,non_marginable_buying_power as "non_marginable_buying_power!"

                   ,bod_dtbp as "bod_dtbp!"
                ,accrued_fees as "accrued_fees!"
                ,pending_transfer_in as "pending_transfer_in!"
                --,portfolio_value as "portfolio_value!"    --deprecated (same as equity field)
                ,trading_blocked as "trading_blocked!"
                ,transfers_blocked as "transfers_blocked!"
                ,account_blocked as "account_blocked!"
                ,created_at as "created_at!"
                ,trade_suspended_by_user as "trade_suspended_by_user!"
                ,multiplier as "multiplier!"
                ,shorting_enabled as "shorting_enabled!"
                ,long_market_value as "long_market_value!"
                ,short_market_value as "short_market_value!"
                ,initial_margin as "initial_margin!"
                ,maintenance_margin as "maintenance_margin!"
                ,last_maintenance_margin as "last_maintenance_margin!"
                ,sma as "sma!"
            from alpaca_account
            order by dtg desc
            limit 1
        "#).fetch_one(pool).await{
        Ok(acct) => Ok(acct),
        Err(_e) => Err(TradeWebError::SqlxError)
    }
}

async fn symbols_get_active(pool:PgPool) -> Result<Vec<String>, TradeWebError> {

    let result: Result<Vec<QrySymbol>, sqlx::Error> = sqlx::query_as!(
            QrySymbol,
            r#"select symbol as "symbol!" from t_symbol where active=true"#
        ).fetch_all(&pool).await;

    match result {
        Ok(symbol_list) => {
            tracing::debug!("[get_symbols] symbol_list: {:?}", &symbol_list);
            let s = symbol_list.iter().map(|x| x.symbol.clone()).collect();
            Ok(s)
        }
        Err(e) => {
            tracing::debug!("[get_symbols] error: {:?}", &e);
            Err(TradeWebError::SqlxError)
        }
    }
}

/// get a vec of stock symbols
async fn symbols_get_all(pool: PgPool) -> Result<Vec<String>, TradeWebError> {

    let result = sqlx::query_as!(QrySymbol,r#"select symbol as "symbol!" from t_symbol order by symbol"#).fetch_all(&pool).await;

    match result {
        Ok(symbol_list) => {
            tracing::debug!("[get_symbols] symbol_list: {:?}", &symbol_list);
            let s = symbol_list.iter().map(|x| x.symbol.clone()).collect();
            Ok(s)
        }
        Err(e) => {
            tracing::debug!("[get_symbols] error: {:?}", &e);
            Err(TradeWebError::SqlxError)
        }
    }
}

async fn insert_order_log_entry(event: &AlpacaOrderLogEvent, pool:&PgPool) -> Result<PgQueryResult, Error> {
    /*

    [{"id":"2412874c-45a4-4e47-b0eb-98c00c1f05eb","client_order_id":"b6f91215-4e78-400d-b2ac-1bb546f86237","created_at":"2023-03-17T06:02:42.552044Z","updated_at":"2023-03-17T06:02:42.552044Z","submitted_at":"2023-03-17T06:02:42.551444Z","filled_at":null,"expired_at":null,"canceled_at":null,"failed_at":null,"replaced_at":null,"replaced_by":null,"replaces":null,"asset_id":"8ccae427-5dd0-45b3-b5fe-7ba5e422c766","symbol":"TSLA","asset_class":"us_equity","notional":null,"qty":"1","filled_qty":"0","filled_avg_price":null,"order_class":"","order_type":"market","type":"market","side":"buy","time_in_force":"day","limit_price":null,"stop_price":null,"status":"accepted","extended_hours":false,"legs":null,"trail_percent":null,"trail_price":null,"hwm":null,"subtag":null,"source":null}]

    */

    let result = sqlx::query!(
            r#"insert into alpaca_order_log(
                dtg
                ,event
                ,id
                ,client_order_id
                ,symbol
                ,qty
                ,filled_qty
                ,filled_avg_price
                ,side
                ,order_type_v2

                ,created_at
                ,updated_at
                ,submitted_at
                ,filled_at
                ,expired_at
                ,canceled_at
                ,failed_at


                -- replaced_at,
                -- replaced_by,
                -- replaces,
                -- asset_id,
                -- asset_class,
                -- notional,
                -- order_class,
                -- order_type_v2,
                -- time_in_force,
                -- limit_price,
                -- stop_price,
                -- status
                -- extended_hours,
                -- trail_percent,
                -- trail_price,
                -- hwm
                )
                values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)

            "#,
            // 1-9
            event.dtg,
            event.event,
            event.order.id,
            event.order.client_order_id,
            event.order.symbol.to_lowercase(),
            event.order.qty,
            event.order.filled_qty,
            event.order.filled_avg_price,
            event.order.side.to_string().to_lowercase(),
            event.order.order_type_v2.to_string(),

            // dates 10-13
            event.order.created_at,
            event.order.updated_at,
            event.order.submitted_at,
            event.order.filled_at,
            event.order.expired_at,
            event.order.canceled_at,
            event.order.failed_at

            // event.order.time_in_force.to_string(), // $13
            // event.order.limit_price,
            // event.order.stop_price,
            // event.order.status
        ).execute(pool).await;
    tracing::debug!("[insert_order_log_entry] result: {:?}", result);

    result

}

async fn account_get_remote(settings:&Settings) -> Result<Account, TradeWebError> {
    // tracing::debug!("[account_get_remote] ************** settings: {:?}", settings);

    let api_key = settings.alpaca_paper_id.clone();
    let api_secret = settings.alpaca_paper_secret.clone();
    let mut headers = HeaderMap::new();
    headers.insert("APCA-API-KEY-ID", api_key.parse().unwrap());
    headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());
    let url = format!("https://paper-api.alpaca.markets/v2/account");

    let client = reqwest::Client::new();
    let http_result = client.get(url).headers(headers).send().await;
    match http_result {
        Ok(resp) => {
            let json_text = &resp.text().await.unwrap();
            tracing::debug!("json: {}", &json_text);
            match serde_json::from_str::<Account>(&json_text) {
                Ok(account) => {
                    tracing::debug!("[account_get_remote] got remote account: {:?}", &account);
                    Ok(account)
                },
                Err(e) => {
                    tracing::error!("[get_account] json: {}", &json_text);
                    tracing::error!("[get_account] json error: {:?}", &e);
                    Err(TradeWebError::JsonError)
                }
            }
        }
        Err(e) => {
            tracing::error!("[get_account] reqwest error: {:?}", &e);
            format!("reqwest error: {:?}", &e);
            Err(TradeWebError::SqlxError)
        }
    }
}

async fn account_save_to_db(account:&Account, pool:&PgPool) -> Result<(), TradeWebError> {

    match sqlx::query!(
            r#"
                insert into alpaca_account(
                    dtg,
                    cash,
                    position_market_value,
                    equity,
                    last_equity,
                    daytrade_count,
                    balance_asof,
                    pattern_day_trader,
                    id,
                    account_number,
                    status,
                    -- crypto_status,
                    currency,
                    buying_power,
                    regt_buying_power,
                    daytrading_buying_power,
                    effective_buying_power,
                    non_marginable_buying_power,
                    bod_dtbp,
                    accrued_fees,
                    pending_transfer_in,
                    -- portfolio_value,    -- deprecated (same as equity)
                    trading_blocked,
                    transfers_blocked,
                    account_blocked,
                    created_at,
                    trade_suspended_by_user,
                    multiplier,
                    shorting_enabled,
                    long_market_value,
                    short_market_value,
                    initial_margin,
                    maintenance_margin,
                    last_maintenance_margin,
                    sma
                    -- crypto_tier: usize,

                ) values(now()::timestamptz, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32)

            "#,
            account.cash,account.position_market_value,account.equity,account.last_equity, account.daytrade_count,
            account.balance_asof, account.pattern_day_trader, account.id, account.account_number, account.status,
            account.currency, account.buying_power, account.regt_buying_power,account.daytrading_buying_power, account.effective_buying_power,
            account.non_marginable_buying_power, account.bod_dtbp, account.accrued_fees, account.pending_transfer_in, account.trading_blocked,
            account.transfers_blocked, account.account_blocked, account.created_at, account.trade_suspended_by_user, account.multiplier, account.shorting_enabled,
            account.long_market_value, account.short_market_value, account.initial_margin, account.maintenance_margin, account.last_maintenance_margin,
            account.sma

        ).execute(pool).await {
        Ok(_) => {
            tracing::debug!("[account_save_to_db] save successful");
            Ok(())
        },
        Err(e) => {
            tracing::error!("[account_save_to_db] save unsuccessful: {:?}", &e);
            Err(TradeWebError::SqlxError)
        }
    }

}




/// the amount of unused funds above the minimum day trade balance
/// option 1: select ((select acct_min_cash_dollars from t_settings) - (select position_market_value from alpaca_account order by dtg limit 1)) as "cash_available!"
/// option 2 (using own calculations and transactions): select coalesce(cash_available,0.0) as "cash_available!" from v_cash_available
/// option 3: below
async fn acct_cash_available(symbol:&str, pool: PgPool) ->Result<MaxBuyPossible, TradeWebError>{
    match sqlx::query_as!(MaxBuyPossible, r#"
        select
            a.price as "price!"
            , size as "size!"
            , cash_available_before as "cash_available!"
            , coalesce(case when cash_available_before > 0 then floor(cash_available_before / a.price) end, 0.0)::numeric as "qty_possible!"
        from (
            select
                price
                ,size
                ,(select v from v_dashboard where k='acct_cash_available (market)')::numeric as cash_available_before
            from trade_alp_latest
            where lower(symbol) = lower($1)
        ) a;
    "#, symbol).fetch_one(&pool).await {
        Ok(result_struct)=> Ok(result_struct),
        Err(_e)=>{
            tracing::error!("[acct_cash_available] sqlx error: {:?}", &_e);
            Err(TradeWebError::SqlxError)
        }
    }
}

async fn acct_dashboard(pool:PgPool)->Vec<Dashboard>{
    match sqlx::query_as!(Dashboard, r#"select k as "k!", v as "v!" from v_dashboard"#).fetch_all(&pool).await{
        Ok(result)=>result,
        Err(e)=>{
            tracing::error!("[acct_dashboard] failed to retrieve dashboard: {:?}", &e);
            vec!()
        }
    }
}

/// get a vec of alpaca trading activities from the postgres database (as a reflection of what's been
/// synced from the Alpaca API)
async fn activities_from_db(pool: PgPool) -> Result<Vec<ActivityQuery>, sqlx::Error> {
    // https://docs.rs/sqlx/0.4.2/sqlx/macro.query.html#type-overrides-bind-parameters-postgres-only
    sqlx::query_as!(
            ActivityQuery,
            r#"
                select
                    dtg::timestamp as "dtg_utc!"
                    ,timezone('US/Pacific', dtg) as "dtg_pacific!"
                    ,symbol as "symbol!"
                    ,side as "side!:TradeSide"
                    ,qty as "qty!"
                    ,price as "price!"
                    ,order_id as "client_order_id!"
                from alpaca_activity
                order by dtg desc
            "#
        )
        .fetch_all(&pool)
        .await
}

async fn activities_from_db_for_symbol(symbol: &str, pool: PgPool) -> Result<Vec<ActivityQuery>, sqlx::Error> {
    // https://docs.rs/sqlx/0.4.2/sqlx/macro.query.html#type-overrides-bind-parameters-postgres-only

    sqlx::query_as!(
            ActivityQuery,
            r#"
                select
                    dtg::timestamp as "dtg_utc!"
                    ,timezone('US/Pacific', dtg) as "dtg_pacific!"
                    ,symbol as "symbol!"
                    ,side as "side!:TradeSide"
                    ,qty as "qty!"
                    ,price as "price!"
                    ,order_id as "client_order_id!"
                from alpaca_activity
                where symbol = lower($1)
                order by dtg desc
            "#,
            symbol
        )
        .fetch_all(&pool)
        .await
}

async fn activity_save_to_db(activity: &Activity, pool: PgPool) -> Result<PgQueryResult, sqlx::Error> {
    let result = sqlx::query!(
            r#"
            insert into alpaca_activity
                (
                id
                , activity_type
                , activity_subtype
                , dtg
                , symbol
                , side
                , qty
                , price
                , cum_qty
                , leaves_qty
                , order_id
                )
                values (
                    $1
                    ,$2
                    ,$3
                    ,$4
                    ,$5
                    ,lower($6)
                    ,$7
                    ,$8
                    ,$9
                    ,$10
                    ,$11
                    )"#,
            activity.id,
            activity.activity_type.to_string(),
            activity.activity_subtype.to_string(),
            activity.dtg,
            activity.symbol.to_lowercase(),
            activity.side.to_string(),
            activity.qty,
            activity.price,
            activity.cum_qty,
            activity.leaves_qty,
            activity.order_id
        ).execute(&pool).await;

    tracing::info!("[save_to_db] activity insert result: {:?}", &result);
    result
}


/// latest_dtg: get the date of the most recent activity; used to filter the activity API
async fn activity_latest_dtg(pool:PgPool)->Result<DateTime<Utc>, TradeWebError>{

    match sqlx::query_as!(ActivityLatest, r#"select max(dtg)::timestamptz as "dtg!" from alpaca_activity"#).fetch_one(&pool).await{
        Ok(latest_dtg)=> Ok(latest_dtg.dtg),
        Err(_e)=>Err(TradeWebError::ReqwestError),
    }
}

async fn activity_get_remote(since_filter:Option<DateTime<Utc>>, settings: Settings) -> Result<Vec<Activity>, TradeWebError> {
    let mut headers = HeaderMap::new();
    let api_key = settings.alpaca_paper_id.clone();
    let api_secret = settings.alpaca_paper_secret.clone();
    headers.insert("APCA-API-KEY-ID", api_key.parse().unwrap());
    headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());


    let url = match since_filter{
        Some(since) =>{
            format!("https://paper-api.alpaca.markets/v2/account/activities/FILL?after={}", urlencoding::encode(since.to_rfc3339().as_str()))
        },
        None => {
            // TODO: put in today's date at least
            format!("https://paper-api.alpaca.markets/v2/account/activities/FILL")
        }
    };

    tracing::debug!("[get_remote] getting activities since: {:?}", &since_filter);

    tracing::debug!("[get_remote] calling API: {}", &url);
    let client = reqwest::Client::new();
    let http_result = client.get(url).headers(headers).send().await;
    let return_val = match http_result {
        Ok(response) => match &response.text().await {
            Ok(response_text) => match serde_json::from_str::<Vec<Activity>>(&response_text) {
                Ok(results) => Ok(results),
                Err(e) => {
                    tracing::debug!("[get_remote] deserialization to json vec failed: {:?}",&e);
                    Err(TradeWebError::JsonError)
                }
            },
            Err(e) => {
                tracing::debug!("[get_remote] deserialization to json text failed: {:?}", &e);
                Err(TradeWebError::JsonError)
            }
        },
        Err(e) => {
            tracing::debug!("[get_remote] reqwest error: {:?}", &e);
            Err(TradeWebError::ReqwestError)
        }
    };
    return_val
}



/// Get the grid provided by v_alpaca_diff
async fn diffcalc_get(pool:PgPool)->Result<Vec<DiffCalc>, PollerError>{

    /*

    -- if this number is positive the crossover is rising; a less negative number minus a more negative number
    -- price_8 = -20 (slower to rise)
    -- price_5 = -5  (faster to rise)
    -- price_5_8 = -5 - -20 = +15 (rising!)
    -- Or, if peaking and falling...
    -- price_8 = 20 (slower to rise)
    -- price_5 = 15  (faster to rise)
    -- price_5_8 = 15 - 20 = -5 (negative, so falling, crossover has occurred and we're headed down)

     */
    let time_start = std::time::SystemTime::now();

    let result = sqlx::query_as!(
        DiffCalc,
        r#"
        select
            *
        -- from v_finnhub_diff
        from v_alpaca_diff
        "#
    ).fetch_all(&pool).await;

    if let Ok(elapsed) = time_start.elapsed(){ tracing::debug!("[Poller::get] elapsed: {:?}", &elapsed); }

    match result {
        Ok(vec) => Ok(vec),
        Err(e) => {
            tracing::error!("[DiffCalc::get] sqlx error: {:?}", &e);
            Err(PollerError::Sqlx)
        }
    }
}

/// Save a single order to the database
pub async fn order_save(order:Order, pool:PgPool) {
    /*

        [{"id":"2412874c-45a4-4e47-b0eb-98c00c1f05eb","client_order_id":"b6f91215-4e78-400d-b2ac-1bb546f86237","created_at":"2023-03-17T06:02:42.552044Z","updated_at":"2023-03-17T06:02:42.552044Z","submitted_at":"2023-03-17T06:02:42.551444Z","filled_at":null,"expired_at":null,"canceled_at":null,"failed_at":null,"replaced_at":null,"replaced_by":null,"replaces":null,"asset_id":"8ccae427-5dd0-45b3-b5fe-7ba5e422c766","symbol":"TSLA","asset_class":"us_equity","notional":null,"qty":"1","filled_qty":"0","filled_avg_price":null,"order_class":"","order_type":"market","type":"market","side":"buy","time_in_force":"day","limit_price":null,"stop_price":null,"status":"accepted","extended_hours":false,"legs":null,"trail_percent":null,"trail_price":null,"hwm":null,"subtag":null,"source":null}]

    */

    let result = sqlx::query!(
            r#"insert into alpaca_order(
                id,
                client_order_id,
                created_at,
                updated_at,
                submitted_at,
                filled_at,

                -- expired_at,
                -- canceled_at,
                -- failed_at,
                -- replaced_at,
                -- replaced_by,
                -- replaces,
                -- asset_id,

                symbol,
                -- asset_class,
                -- notional,
                qty,
                filled_qty,
                filled_avg_price,
                -- order_class,
                order_type_v2,
                side,
                time_in_force,
                limit_price,
                stop_price,
                status
                -- extended_hours,
                -- trail_percent,
                -- trail_price,
                -- hwm
                )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, lower($11), lower($12), lower($13), $14, $15, $16)

            "#,
            order.id, order.client_order_id, order.created_at, order.updated_at, order.submitted_at,
            order.filled_at, // $7
            order.symbol.to_lowercase(),
            order.qty, order.filled_qty, order.filled_avg_price,
            order.order_type_v2.to_string(), // $11
            order.side.to_string(),          // $12
            order.time_in_force.to_string(), // $13
            order.limit_price,
            order.stop_price,
            order.status
        ).execute(&pool).await;

    tracing::debug!("[save_to_db] result: {:?}", result);
}

/// do the REST part of the orders POST API
/// see documentation above
async fn rest_post_order(json_trade: &JsonTrade, settings: &Settings) -> Result<Order, TradeWebError> {
    let mut headers = HeaderMap::new();

    let api_key = settings.alpaca_paper_id.clone();
    let api_secret = settings.alpaca_paper_secret.clone();

    headers.insert("APCA-API-KEY-ID", api_key.parse().unwrap());
    headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());

    let client = reqwest::Client::new();

    let response = client.post("https://paper-api.alpaca.markets/v2/orders")
        .headers(headers)
        .json(json_trade)
        .send()
        .await;

    match response {
        Ok(resp) => {
            match resp.status() {
                reqwest::StatusCode::OK => {
                    tracing::info!("[perform_trade] response code: {:?} from the orders API", reqwest::StatusCode::OK);

                    let json: Result<Order, reqwest::Error> = resp.json().await;

                    match json {
                        Ok(json) => Ok(json),
                        Err(e) => {
                            tracing::error!("[post_order] json error: {:?}", &e);
                            Err(TradeWebError::JsonError)
                        }
                    }
                },
                reqwest::StatusCode::FORBIDDEN => {
                    // 403
                    // https://docs.rs/http/0.2.9/http/status/struct.StatusCode.html#associatedconstant.UNPROCESSABLE_ENTITY
                    // https://alpaca.markets/docs/api-references/trading-api/orders/
                    // response may look like:
                    // {"code":40310000,"message":"cannot open a long buy while a short sell order is open"}

                    // ("{\"available\":\"7\",\"code\":40310000,\"existing_qty\":\"14\",\"held_for_orders\":\"7\",\"message\":\"insufficient qty available for order (requested: 14, available: 7)\",\"related_orders\":[\"2da05e7c-7d91-4a06-9275-b7bb96f6d45c\"],\"symbol\":\"WBD\"}")

                    tracing::error!("[perform_trade:403] response code: {:?} from the orders API", reqwest::StatusCode::FORBIDDEN);
                    tracing::info!("[perform_trade:403] response: {:?} ", &resp);

                    let json = resp.text().await;
                    tracing::error!("[perform_trade:403] json body: {:?}", &json);


                    // resp.json().await
                    Err(TradeWebError::Alpaca403)
                },
                reqwest::StatusCode::UNPROCESSABLE_ENTITY => {
                    // 422
                    tracing::error!("[perform_trade] response code: {:?} from the orders API", reqwest::StatusCode::UNPROCESSABLE_ENTITY);
                    tracing::error!("[perform_trade] response: {:?} ", &resp);
                    // tracing::error!("[perform_trade] response: {:?} ", &resp.text().await.unwrap());

                    let json = resp.text().await;
                    tracing::error!("[perform_trade:422] json body: {:?}", &json);

                    Err(TradeWebError::Alpaca422)
                },
                _ => {
                    tracing::error!("[perform_trade] response code: {:?} from the orders API", &resp.status());
                    // reqwest::Error
                    // resp.json().await
                    Err(TradeWebError::ReqwestError)
                }
            }
        },
        Err(e) => {
            // Err(e)
            tracing::debug!("[post_order] reqwest error: {:?}", &e);
            Err(TradeWebError::ReqwestError)
        }
    }
}

/// Get the most recent list of positions from the database
///
/// TODO: make these simple inserts non-blocking and non-async
pub async fn order_local(pool:PgPool) -> Result<Vec<Order>, TradeWebError> {

    // Assume the latest batch was inserted at the same time; get the most recent timestamp, get the most recent matching positions
    // https://docs.rs/sqlx/0.4.2/sqlx/macro.query.html#type-overrides-bind-parameters-postgres-only
    let result_vec = sqlx::query_as!(
            Order,
            r#"
                select
                    id as "id!"
                    , client_order_id as "client_order_id!"
                    , created_at as "created_at!"
                    , updated_at as "updated_at!"
                    , submitted_at as "submitted_at!"
                    , filled_at
                    , expired_at
                    , canceled_at
                    , failed_at
                    , replaced_at
                    , replaced_by
                    , replaces
                    , asset_id as "asset_id!:Option<String>"
                    , symbol as "symbol!:String"
                    , asset_class as "asset_class!:Option<String>"
                    , notional as "notional!:Option<BigDecimal>"
                    , coalesce(qty, 0.0) as "qty!:BigDecimal"
                    , filled_qty as "filled_qty!:Option<BigDecimal>"
                    , filled_avg_price as "filled_avg_price!:Option<BigDecimal>"
                    , order_class as "order_class!:Option<String>"
                    , order_type_v2 as "order_type_v2!:OrderType"
                    , side as "side!:TradeSide"
                    , time_in_force as "time_in_force!:TimeInForce"
                    , limit_price as "limit_price!:Option<BigDecimal>"
                    , stop_price as "stop_price!:Option<BigDecimal>"
                    , status as "status!"
                    , coalesce(extended_hours, false) as "extended_hours!"
                    , trail_percent
                    , trail_price
                    , hwm
                from alpaca_order
                where filled_at is null
                order by symbol asc
            "#
        ).fetch_all(&pool).await;

    match result_vec {
        Ok(v) => Ok(v),
        Err(_e) => Err(TradeWebError::SqlxError),
    }

}


/// save a new order before it's submitted
pub async fn order_log_entry_save(entry: OrderLogEntry, pool:PgPool) -> Result<PgQueryResult, Error> {
    tracing::debug!("[order_log_entry_save]: {:?}", &entry);
    sqlx::query!(
            r#"insert into log_orders(id, id_group, id_client, dtg, symbol, side, qty)
            values($1, $2, $3, $4, $5, $6, $7)"#,
            entry.id, entry.id_group, entry.id_client(), entry.dtg, entry.symbol.to_lowercase(), entry.side.to_string(), entry.qty
        ).execute(&pool).await
}

async fn position_get_remote(settings: &Settings) -> Result<Vec<Position>, reqwest::Error> {

    let mut headers = reqwest::header::HeaderMap::new();
    let api_key_id = settings.alpaca_paper_id.clone();
    let api_secret = settings.alpaca_paper_secret.clone();
    headers.insert("APCA-API-KEY-ID", api_key_id.parse().unwrap());
    headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());

    let client = reqwest::Client::new();
    let remote_positions: Vec<TempPosition> = client
        .get("https://paper-api.alpaca.markets/v2/positions")
        .headers(headers)
        .send()
        .await?
        .json()
        .await?;

    let now = Utc::now();
    let remote_positions:Vec<Position> = remote_positions
        .iter()
        .map(move |x| Position::from_temp(now, x.clone()))
        .collect();

    tracing::debug!("[get_remote] got {} positions", &remote_positions.len());
    Ok(remote_positions)
}

async fn position_delete_all(pool: &PgPool) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query!(r#"delete from alpaca_position"#)
        .execute(pool)
        .await
}

async fn position_save_to_db(position:&Position, pool:&PgPool) -> Result<PgQueryResult, sqlx::Error> {

    let result = sqlx::query!(
            r#"
                insert into alpaca_position(dtg, symbol, exchange, asset_class, avg_entry_price, qty, qty_available, side, market_value
                    , cost_basis, unrealized_pl, unrealized_plpc, current_price, lastday_price, change_today, asset_id)
                values
                    (
                        now(),lower($1),$2,$3,$4,$5,$6,$7,$8,$9,$10,$11, $12, $13, $14, $15
                    )"#,
            position.symbol, position.exchange, position.asset_class, position.avg_entry_price, position.qty, position.qty_available,
            position.side.to_string(), position.market_value, position.cost_basis, position.unrealized_pl, position.unrealized_plpc, position.current_price,
            position.lastday_price, position.change_today, position.asset_id
        ).execute(pool).await;

    // tracing::info!("[activity::save_to_db] position insert result: {:?}", &result);
    result
}


/// positions with a market value per share higher than their purchase price per share.
pub async fn position_list_showing_profit(pl_filter:BigDecimal, pool: PgPool) ->Result<Vec<SellPosition>,PollerError>{
    let result = sqlx::query_as!(SellPosition,r#"
            select
                stock_symbol as "symbol!"
                , price as "avg_entry_price!"
                , sell_qty as "qty!"
                --, sell_qty_available as "qty_available!"
                , unrealized_pl_per_share as "unrealized_pl_per_share!"
                , cost as "cost_basis!"
                , unrealized_pl_total as "unrealized_pl_total!"
                , coalesce(trade_size,0.0) as "trade_size!"
                , coalesce(age_min,0.0) as "age_minute!"
            from fn_positions_to_sell_high($1) a
            left join t_symbol b on lower(a.stock_symbol) = lower(b.symbol)
        "#, pl_filter).fetch_all(&pool).await;
    match result {
        Ok(positions)=>Ok(positions),
        Err(_e)=> Err(PollerError::Sqlx)
    }
}

/// positions to sell with age beyond limits
pub async fn position_list_showing_age(pool: PgPool) ->Result<Vec<SellPosition>,PollerError>{
    let result = sqlx::query_as!(SellPosition, r#"
            select
                a.symbol as "symbol!"
                ,price_posn_entry as "avg_entry_price!"
                ,qty_posn as "qty!"
                ,pl_posn_share as "unrealized_pl_per_share!"
                ,basis as "cost_basis!"
                ,pl_posn as "unrealized_pl_total!"
                ,coalesce(trade_size,0.0) as "trade_size!"
                ,coalesce(posn_age_sec / 60.0,0.0) as "age_minute!"
            from v_positions_aging_sell_at_loss a
            left join t_symbol b on a.symbol = b.symbol;
        "#).fetch_all(&pool).await;
    match result {
        Ok(positions)=>Ok(positions),
        Err(_e)=> Err(PollerError::Sqlx)
    }
}

pub async fn transaction_delete_one(symbol:&str, pool:PgPool)->Result<(), TradeWebError>{
    match sqlx::query!(r#"delete from alpaca_transaction_status where symbol=$1"#, symbol.to_lowercase()).execute(&pool).await{
        Ok(_)=>Ok(()),
        Err(_e)=>Err(TradeWebError::DeleteFailed), // or db error
    }
}

/// return blank secret for front-end type uses
pub async fn settings_load_no_secret(pool: PgPool) -> Result<Settings, TradeWebError> {
    tracing::debug!("[settings_load_no_secret]");

    let settings_result = sqlx::query_as!(Settings,
        r#"
            SELECT
                dtg as "dtg!",
                alpaca_paper_id as "alpaca_paper_id!:String",
                '' as "alpaca_paper_secret!:String",
                alpaca_live_id as "alpaca_live_id!:String",
                '' as "alpaca_live_secret!:String",
                trade_size as "trade_size!",
                trade_enable_buy as "trade_enable_buy!",
                trade_ema_small_size as "trade_ema_small_size!",
                trade_ema_large_size as "trade_ema_large_size!",
                trade_sell_high_per_cent_multiplier as "trade_sell_high_per_cent_multiplier!",
                trade_sell_high_upper_limit_cents as "trade_sell_high_upper_limit_cents!"
                ,finnhub_key as "finnhub_key!:String"
                ,coalesce(account_start_value,0.0) as "account_start_value!"
                ,coalesce(max_position_age_minute,0.0) as "max_position_age_minute!"
                ,coalesce(upgrade_min_profit,0.0) as "upgrade_min_profit!"
                ,coalesce(upgrade_sell_elapsed_minutes_min,60.0) as "upgrade_sell_elapsed_minutes_min!"
                ,coalesce(upgrade_posn_max_elapsed_minutes,60.0) as "upgrade_posn_max_elapsed_minutes!"
                ,coalesce(upgrade_posn_loss_allowed_dollars,10.0) as "upgrade_posn_loss_allowed_dollars!"
                ,coalesce(acct_max_position_market_value,10.0) as "acct_max_position_market_value!"
                ,coalesce(acct_min_cash_dollars,10.0) as "acct_min_cash_dollars!"
            FROM t_settings
            ORDER BY t_settings.dtg DESC
            LIMIT 1
        "#
    ).fetch_one(&pool).await;
    match settings_result{
        Ok(result)=>Ok(result),
        Err(_e)=>Err(TradeWebError::SqlxError),
    }
}

async fn settings_load_with_secret(pool:PgPool) -> Result<Settings, TradeWebError> {

    tracing::debug!("[settings_load_with_secret]");

    let settings_result = sqlx::query_as!(
        Settings,
        r#"
            SELECT
                dtg as "dtg!",
                alpaca_paper_id as "alpaca_paper_id!:String",
                alpaca_paper_secret as "alpaca_paper_secret!:String",
                alpaca_live_id as "alpaca_live_id!:String",
                alpaca_live_secret as "alpaca_live_secret!:String",
                trade_size as "trade_size!",
                trade_enable_buy as "trade_enable_buy!",
                trade_ema_small_size as "trade_ema_small_size!",
                trade_ema_large_size as "trade_ema_large_size!",
                trade_sell_high_per_cent_multiplier as "trade_sell_high_per_cent_multiplier!",
                trade_sell_high_upper_limit_cents as "trade_sell_high_upper_limit_cents!"
                ,finnhub_key as "finnhub_key!:String"
                ,coalesce(account_start_value,0.0) as "account_start_value!"
                ,coalesce(max_position_age_minute,0.0) as "max_position_age_minute!"
                ,coalesce(upgrade_min_profit,0.0) as "upgrade_min_profit!"
                ,coalesce(upgrade_sell_elapsed_minutes_min,60.0) as "upgrade_sell_elapsed_minutes_min!"
                ,coalesce(upgrade_posn_max_elapsed_minutes,60.0) as "upgrade_posn_max_elapsed_minutes!"
                ,coalesce(upgrade_posn_loss_allowed_dollars,10.0) as "upgrade_posn_loss_allowed_dollars!"
                ,coalesce(acct_max_position_market_value,10.0) as "acct_max_position_market_value!"
                ,coalesce(acct_min_cash_dollars,10.0) as "acct_min_cash_dollars!"
            FROM t_settings
            ORDER BY t_settings.dtg DESC
            LIMIT 1
        "#
    ).fetch_one(&pool).await;

    match settings_result{
        Ok(result)=>Ok(result),
        Err(_e)=>{
            tracing::error!("[settings_load_with_secret] sqlx error: {:?}", &_e);
            Err(TradeWebError::SqlxError)
        },
    }

}

pub async fn symbol_load_one(symbol: &str, pool: PgPool) -> Result<Symbol, TradeWebError> {
    match sqlx::query_as!(Symbol,
        r#"
            select
                symbol as "symbol!"
                ,active as "active!"
                ,coalesce(trade_size,0.0) as "trade_size!"
            from t_symbol where lower(symbol)=lower($1)
        "#,
        symbol
    ).fetch_one(&pool).await {
        Ok(symbol_list) => {
            // tracing::debug!("[get_symbols] symbol_list: {:?}", &symbol_list);
            // let s = symbol_list.iter().map(|x| { x.symbol.clone() }).collect();
            Ok(symbol_list)
        },
        Err(e) => {
            tracing::debug!("[get_symbols] error: {:?}", &e);
            Err(TradeWebError::SqlxError)
        }
    }
}

/// create a new entry or update the position's shares with the current timestamp
async fn transaction_insert_position(position:&Position, pool:&PgPool) ->Result<(), TradeWebError>{

    match sqlx::query!(
        r#"
            insert into alpaca_transaction_status(dtg, symbol, posn_shares)
            values(now()::timestamptz, $1, $2)
            ON CONFLICT (symbol) DO UPDATE
                SET posn_shares = $2, dtg=now()::timestamptz;
        "#,
        position.symbol.to_lowercase(),
        position.qty
    ).execute(pool).await{
        Ok(_)=>{
            // tracing::debug!("[transaction_insert_position] successful insert");
            Ok(())
        },
        Err(e)=>{
            tracing::error!("[transaction_insert_position] unsuccessful insert: {:?}", &e);
            Err(TradeWebError::DeleteFailed)
        } // or db error
    }
}

/// insert a new transaction if one doesn't currently exist, otherwise error
pub async fn transaction_start_buy(symbol:&str, pool:PgPool)->BuyResult{
    // if an entry exists then a buy order exists; otherwise create one with default 0 shares
    match sqlx::query!(r#"
                insert into alpaca_transaction_status(dtg, symbol, posn_shares)
                values(now()::timestamptz, $1, 0.0
            )"#, symbol.to_lowercase()
    ).execute(&pool).await{
        Ok(_)=>{
            BuyResult::Allowed
        },
        Err(_e)=>BuyResult::NotAllowed{error:TradeWebError::PositionExists}, // or db error
    }
}

// /// sell if a buy order previously created an entry in this table and subsequently the count of shares is greater than zero
// /// TODO: not currently used
// pub async fn transaction_start_sell(symbol:&str, pool:PgPool)->Result<BigDecimal, TransactionError>{
//     match sqlx::query_as!(AlpacaTransaction, r#"select dtg as "dtg!", symbol as "symbol!", posn_shares as "posn_shares!" from alpaca_transaction_status where symbol=$1 and posn_shares > 0.0"#, symbol.to_lowercase())
//         .fetch_one(&pool).await{
//             Ok(transaction)=> Ok(transaction.posn_shares),
//             Err(_e)=>Err(TradeWebError::NoSharesFound), // or db error
//         }
// }

/// get the positions from the local database, filtered and P/L computed to display on the web frontend
async fn position_local_get(pool:PgPool)->Result<Vec<PositionLocal>, TradeWebError>{

    let result = sqlx::query_as!(PositionLocal, r#"
        select
            symbol as "symbol!"
            ,profit_closed as "profit_closed!"
            ,coalesce(pl_posn,0.0) as "pl_posn!"
            ,coalesce(pl_posn_share,0.0) as "pl_posn_share!"
            ,coalesce(posn_age_sec, 0.0) as "posn_age_sec!"
            ,qty_posn as "qty_posn!"
            ,price_posn_entry as "price_posn_entry!"
            ,coalesce(basis,0.0) as "basis!"
            ,coalesce(price_market,0.0) as "price_market!"
            ,coalesce(market_value,0.0) as "market_value!"
            ,dtg as "dtg!"
        from v_positions_from_activity
    "#).fetch_all(&pool).await;

    match result {
        Ok(vec)=>Ok(vec),
        Err(e) =>{
            tracing::error!("[position_get_local] error: {:?}", &e);
            Err(TradeWebError::SqlxError)
        }
    }

}

struct DbBigDecimal{
    result_big_decimal:BigDecimal
}

async fn websocket_alpaca_alive(pool:PgPool)->Result<bool,TradeWebError>{

    let result:Result<DbBigDecimal, sqlx::Error> = sqlx::query_as!(DbBigDecimal, r#"
        select
            extract(epoch from (current_timestamp - max(ping))::interval)::numeric as "result_big_decimal!"
        from ping_alpaca"#
    ).fetch_one(&pool).await;

    match result {
        Ok(ping_alive) => {
            if ping_alive.result_big_decimal > BigDecimal::from(60) {
                Ok(false)
            }
            else {
                Ok(true)
            }
        },
        Err(e) => {
            tracing::error!("[websocket_alpaca_alive] {:?}", &e);
            Err(TradeWebError::SqlxError)
        }


    }

}

