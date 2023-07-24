//! db.rs
//!
//! start() spawns a long-running thread to maintain an open connection to a database pool
//!

use sqlx::postgres::PgQueryResult;
use sqlx::{Error, PgPool};
use chrono::{DateTime, Utc};
use reqwest::header::HeaderMap;
use tokio::runtime::Handle;
use tokio::sync::oneshot;
use crate::account::{Account, AccountWithDate};
use crate::alpaca_activity::{Activity, ActivityLatest};
use crate::alpaca_api_structs::{AlpacaTradeWs, MinuteBar, Ping};
use crate::alpaca_order_log::AlpacaOrderLogEvent;
use crate::alpaca_position::{Position, TempPosition};
use crate::alpaca_transaction_status::TransactionError;
use crate::error::TradeWebError;
use crate::finnhub::{FinnhubPing, FinnhubTrade};
use crate::settings::Settings;
use crate::sqlx_pool::create_sqlx_pg_pool;
use crate::symbol_list::QrySymbol;
use crate::trade_struct::TradeSide;
use crate::alpaca_transaction_status::*;

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
    GetSettingsWithSecret{ resp_tx: crossbeam_channel::Sender<Settings> },
    TransactionDeleteAll,
    AccountGet{ resp_tx: oneshot::Sender<AccountWithDate> },
    // TODO: pass settings to others that'd need updated settings while running
    AccountGetRemote{ settings:Settings, resp_tx: oneshot::Sender<Account> },
    AccountSaveToDb{ account:Account},

    PositionGetRemote{ settings:Settings, resp_tx: oneshot::Sender<Vec<Position>> },
    PositionDeleteAll,
    PositionSaveToDb { position:Position },

    TransactionInsertPosition { position:Position },

    ActivityLatestDtg{ resp_tx: oneshot::Sender<DateTime<Utc>> },
    ActivityGetRemote{ since_option:Option<DateTime<Utc>>, settings:Settings, resp_tx: oneshot::Sender<Vec<Activity>>},
    ActivitySaveToDb { activity:Activity },

    GetSymbolList{resp_tx: crossbeam_channel::Sender<Vec<String>> },

}

pub struct DbActor {
    pub pool: PgPool,
    pub tx:crossbeam_channel::Sender<DbMsg>,
    pub rx:crossbeam_channel::Receiver<DbMsg>,
}

impl DbActor {

    pub async fn new() -> DbActor{
        let pool = create_sqlx_pg_pool().await;
        let (tx, rx) = crossbeam::channel::unbounded();
        DbActor{
            pool,
            tx,
            rx,
        }
    }

    /// DB Listener
    ///
    /// Other threads can send DbMsg messages via crossbeam to perform inserts into the database cross-thread.
    ///
    /// Each db network call takes 150-300ms on LAN/wifi
    pub fn run(&self, tr: Handle) /*-> JoinHandle<()> */{

        let rx = self.rx.clone();
        let pool = self.pool.clone();

        tracing::debug!("[run]");

        loop {

            // tracing::debug!("[run] loop");

            let pool = pool.clone();
            crossbeam::channel::select! {
                recv(rx) -> result => {
                    match result {
                        Ok(msg)=>{
                            // tracing::debug!("[run] msg: {:?}", &msg);

                            // blocking not required for sqlx and reqwest async libraries
                            let _x = tr.spawn(async {
                                tracing::debug!("[run] tokio spawn process_message()");
                                process_message(msg, pool).await;
                            });

                        },
                        Err(e)=>{
                            tracing::error!("[run] select error: {:?}", &e);
                        }
                    }
                },
                default=>{
                    // infinite loop if nothing to read
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
        DbMsg::PingDb=>{
            tracing::debug!("[db_thread][DbMsg::PingDb] pong: db thread here!");
        },

        DbMsg::GetSettingsWithSecret{resp_tx}=>{
            tracing::debug!("[db] received DbMsg::GetSettingsWithSecret");
            match settings_load_with_secret(pool).await {
                Ok(settings)=>{
                    tracing::debug!("[db] got settings; sending them back...");
                    let send_result = resp_tx.send(settings);
                    tracing::debug!("[db] send_result: {:?}", &send_result);

                },
                Err(_e)=>{
                    tracing::debug!("[db] received DbMsg::GetSettingsWithSecret error: {:?}", &_e);

                }
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

        DbMsg::TransactionInsertPosition{ position }=>{
            let _ = transaction_insert_position(&position, &pool).await;
        },


        DbMsg::AccountSaveToDb{ account }=>{
            let _ = account_save_to_db(&account, &pool).await;
        },


        DbMsg::GetSymbolList{resp_tx}=>{
            let result = symbols_get_active(&pool).await;
            if let Ok(symbols) = result {
                let _ = resp_tx.send(symbols);
            }
        },


        DbMsg::TransactionDeleteAll=>{
            let _ = transaction_delete_all(&pool).await;
        }


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
        }

        DbMsg::OrderLogEvent(event)=>{
            tracing::debug!("[db] received DbMsg::OrderLogEvent: {:?}", &event);

            let _ = insert_order_log_entry(&event, &pool).await;

            // if this is a Fill event on the Sell side, decrement the shares filled from the alpaca_transaction_status entry

            // tracing::info!("[DbMsg::OrderLogEvent][Fill] log_evt: {:?}", &log_evt);

            if event.event=="fill" && event.order.side== TradeSide::Sell {

                tracing::info!("[DbMsg::OrderLogEvent][Fill][Sell] {}:{:?}", &event.order.symbol, &event.order.filled_qty);
                if let Some(qty) = event.order.filled_qty{

                    let _ = AlpacaTransaction::decrement(&event.order.symbol.clone(), qty, &pool).await;
                    // if the remaining shares are zero or less delete the entry (TODO: make this one sql statement)
                    let _ = AlpacaTransaction::clean(&pool).await;
                }
            }
        },

        DbMsg::RefreshRating => {
            tracing::debug!("[db] received DbMsg::RefreshRating");
            refresh_rating(&pool).await;
        }

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
        }

        _ => { }
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
            insert into trade_fh (dtg,symbol, price, volume) values ($1, $2, $3, $4)
        "#,
        trade.dtg.naive_utc(),
        trade.symbol,
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
            values ($1, $2, $3, $4)
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
            values ($1, $2, $3, $4)
        "#,
        t.dtg.naive_utc(),t.symbol,t.price,t.size
    ).execute(pool).await
}

/// Continuously overwrite only the latest trade for a given symbol so we have a fast way of getting the most recent price.
async fn insert_alpaca_trade_latest(trade: &AlpacaTradeWs, pool: &PgPool) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query!(
        r#"insert into trade_alp_latest(dtg,symbol,price,size)
            values ($1, $2, $3, $4)
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
        r#"insert into ping_alpaca (ping) values ($1)"#,
        ping.dtg.naive_utc()
    ).execute(pool).await
}

async fn settings_load_with_secret(pool:PgPool) -> Result<Settings, Error> {

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
        )
        .fetch_one(&pool)
        .await;

    settings_result

}

async fn transaction_delete_all(pool:&PgPool) -> Result<(), TransactionError> {
    match sqlx::query!(
            r#"
                delete from alpaca_transaction_status
            "#
        ).execute(pool).await{
        Ok(_)=>Ok(()),
        Err(_e)=>Err(TransactionError::DeleteFailed), // or db error
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

async fn symbols_get_active(pool:&PgPool) -> Result<Vec<String>, TradeWebError> {

    let result: Result<Vec<QrySymbol>, sqlx::Error> = sqlx::query_as!(
            QrySymbol,
            r#"select symbol as "symbol!" from t_symbol where active=true"#
        ).fetch_all(pool).await;

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
                        now(),$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11, $12, $13, $14, $15
                    )"#,
            position.symbol, position.exchange, position.asset_class, position.avg_entry_price, position.qty, position.qty_available,
            position.side.to_string(), position.market_value, position.cost_basis, position.unrealized_pl, position.unrealized_plpc, position.current_price,
            position.lastday_price, position.change_today, position.asset_id
        ).execute(pool).await;

    // tracing::info!("[activity::save_to_db] position insert result: {:?}", &result);
    result
}

/// create a new entry or update the position's shares with the current timestamp
async fn transaction_insert_position(position:&Position, pool:&PgPool) ->Result<(), TransactionError>{

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
            tracing::debug!("[transaction_insert_position] successful insert");
            Ok(())
        },
        Err(e)=>{
            tracing::error!("[transaction_insert_position] unsuccessful insert: {:?}", &e);
            Err(TransactionError::DeleteFailed)
        } // or db error
    }
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
            activity.symbol,
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

// /// insert a new transaction if one doesn't currently exist, otherwise error
// /// TODO: not currently used; used by trade_poller
// async fn start_buy(symbol:&str, pool:&PgPool)->Result<(), TransactionError>{
//     // if an entry exists then a buy order exists; otherwise create one with default 0 shares
//     match sqlx::query!(r#"
//             insert into alpaca_transaction_status(dtg, symbol, posn_shares)
//             values(now()::timestamptz, $1, 0.0)"#, symbol
//         ).execute(pool).await{
//         Ok(_)=>Ok(()),
//         Err(_e)=>Err(TransactionError::PositionExists), // or db error
//     }
// }

