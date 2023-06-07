//! db.rs
//!
//! start() spawns a long-running thread to maintain an open connection to a database pool
//!

use crossbeam::channel::Sender;
use std::thread::JoinHandle;
use sqlx::PgPool;
use sqlx::postgres::PgQueryResult;
use tokio_postgres::{Client, SimpleQueryMessage};
use common_lib::alpaca_api_structs::{AlpWsQuote, AlpWsTrade};
use common_lib::common_structs::{AlpacaPing, MinuteBar};
use common_lib::finnhub::{FinnhubPing, FinnhubTrade};
use common_lib::sqlx_pool::create_sqlx_pg_pool;

#[derive(Debug)]
pub enum DbMsg {
    // LastTrade(AlpacaTradeRest),
    WsTrade(AlpWsTrade),
    WsQuote(AlpWsQuote),
    MinuteBar(MinuteBar),
    FhTrade(FinnhubTrade),
    FhPing(FinnhubPing),
    AlpacaPing(AlpacaPing),
}

pub struct DbActor{}

impl DbActor {
    /// start()
    ///
    /// return a crossbeam_channel::channel::Sender in order to be able to send messages to the
    /// db listener thread (to be able to send cross-thread inserts)
    pub async fn start() -> Sender<DbMsg> {
        tracing::debug!("");

        // Channel for websocket thread to send to database thread
        let (tx, rx) = crossbeam::channel::unbounded();

        // connect to Postgres
        let client: Client = tokio::spawn(async {
            let db_log_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not found");
            db_connect(&db_log_url).await
        })
            .await
            .unwrap();

        // upon connection start a message-listening thread
        tokio::spawn(async move {
            crate::db::db_thread(client, rx).await;
        });

        // return a means of sending messages to the db listener thread
        tx
    }
}

///
/// TODO: convert to sqlx::PgConnect
/// - convert to using migrations
/// - PgConnectOptions would let us load DB credentials from env nicely without having to generate a
/// connection string
/// - there's currently no pool; sqlx makes that pretty easy
///
///
pub async fn db_connect(db_url: &str) -> tokio_postgres::Client {
    // no need to log the db password
    // tracing::debug!("[db_connect] db_url: {}", &db_url);

    let (client, connection) = tokio_postgres::connect(db_url, tokio_postgres::NoTls)
        .await
        .unwrap();

    // spin off the database connection to its own thread
    tokio::spawn(async move {
        // https://docs.rs/tokio-postgres/0.6.0/tokio_postgres/struct.Connection.html
        // "Connection implements Future, and only resolves when the connection is closed, either
        // because a fatal error has occurred, or because its associated Client has dropped and all
        // outstanding work has completed."
        if let Err(e) = connection.await {
            tracing::debug!("postgres connection closed: {}", e);
        }
    });
    tracing::debug!("[db_connect] connected");
    client
}

/// DB Listener
///
/// Other threads can send DbMsg messages via crossbeam to perform inserts into the database cross-thread.
///
/// Each db network call takes 150-300ms on LAN/wifi
async fn db_thread(client: Client, rx: crossbeam::channel::Receiver<DbMsg>) -> JoinHandle<()> {
    tracing::debug!("[db_thread]");


    // TODO: starting here I'm using SQLX instead of tokio postgres
    let pool = create_sqlx_pg_pool().await;


    loop {
        crossbeam::channel::select! {
            recv(rx) -> result => {
                if let Ok(msg) = result {
                    match msg {

                        // DbMsg::LastTrade(last_trade) => {
                        //     tracing::debug!("[db_thread, DbMsg::LastTrade] {:?}", &last_trade);
                        //     crate::db::insert_trade_rest(&client, last_trade).await;
                        // },

                        DbMsg::WsQuote(q) => {
                            tracing::debug!("[db_thread, DbMsg::WsQuote] quote: {:?}", &q);
                            crate::db::insert_ws_quote(&client, q).await;
                        },

                        DbMsg::WsTrade(t) => {
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

                        DbMsg::MinuteBar(minute_bar) => {
                            // tracing::debug!("[db_thread, DbMsg::MinuteBar] minute_bar received by db thread: {:?}", &minute_bar);
                            crate::db::insert_minute_bar(&client, &minute_bar).await;
                        }

                        DbMsg::FhTrade(finnhub_trade) => {

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

                        DbMsg::FhPing(ping) => {
                            let _ = insert_finnhub_ping(&ping, &pool).await;

                        },
                        DbMsg::AlpacaPing(ping) => {
                            let _ = insert_alpaca_ping(&ping, &pool).await;

                        }
                    }
                }
            }
        }
    }
}



/// deprecated
// async fn insert_alpaca_websocket_trade(client: &Client, t: &AlpWsTrade) {
//     tracing::debug!("");
//
//     let sql = format!(
//         r"
// 		insert into t_ws_trade(
// 			dtg,
// 			event,
// 			symbol,
// 			-- exchange,
// 			price,
// 			size,
// 			id_trade
// 			--,
// 			-- c:Vec<usize>,
// 			-- id_tape
// 		)
// 		values ('{}','{}','{}',{},{}, {});",
//         t.dtg, t.event, t.symbol, /*t.exchange,*/ t.price, t.size, t.id_trade/*, t.id_tape*/
//     );
//
//     // tracing::debug!("[insert_ws_trade] sql: {}", &sql);
//
//     // run query
//     if let Ok(result_vec) = client.simple_query(&sql).await {
//         for i in result_vec {
//             match i {
//                 SimpleQueryMessage::CommandComplete(row_count) => {
//                     tracing::debug!("[insert_ws_trade] {} row(s) inserted", row_count);
//                 }
//
//                 SimpleQueryMessage::Row(_row) => {}
//                 _ => tracing::debug!("[insert_ws_trade] Something weird happened on log query."),
//             }
//         }
//     } else {
//         // TODO: why is this happening? (1/20/2021)
//         tracing::debug!("[insert_ws_trade] insert failed");
//     }
// }

/// TODO: convert this to SQLX; it currently does not use a connection pool
async fn insert_ws_quote(client: &Client, t: AlpWsQuote) {
    let sql = format!(
        r"
		insert into t_ws_quote(
			dtg,
			dtg_updated,
			event,
			symbol,

			exchange_bid,
			price_bid,
			size_bid,

			exchange_ask,
			price_ask,
			size_ask
		)
		values ('{}','{}','{}','{}',
			{},{},{},
			{},{},{});",
        t.dtg,
        t.dtg_updated,
        t.event,
        t.symbol,
        t.exchange_bid,
        t.price_bid,
        t.size_bid,
        t.exchange_ask,
        t.price_ask,
        t.size_ask
    );

    // tracing::debug!("[insert_ws_quote] sql: {}", &sql);

    // run query
    if let Ok(result_vec) = client.simple_query(&sql).await {
        for i in result_vec {
            match i {
                SimpleQueryMessage::CommandComplete(row_count) => {
                    tracing::debug!("[insert_ws_quote] {} row(s) inserted", row_count);
                }
                SimpleQueryMessage::Row(_row) => {}
                _ => tracing::debug!("[insert_ws_quote] Something weird happened on log query."),
            }
        }
    } else {
        tracing::debug!("[insert_ws_quote] log insert failed");
    }
}

// /// insert a result from polling the rest API
// /// TODO: convert this to SQLX; it currently does not use a connection pool
// async fn insert_trade_rest(client: &Client, trade_rest: AlpacaTradeRest) {
//     /*
//         insert into t_last_trade(dtg, price, size, exchange, cond1, cond2, cond3, cond4)
//         values (now(), 0.0, 0.0, 0, 0, 0, 0, 0)
//     */
//
//     let sql = format!(
//         r#"
// 		insert into t_last_trade(dtg_updated, symbol, dtg, price, size)
// 		values ('{}', '{}', '{}'::timestamp, {}, {})"#,
//         trade_rest.dtg_updated,
//         trade_rest.symbol,
//         trade_rest.trade.dtg,
//         trade_rest.trade.price,
//         trade_rest.trade.size // , trade_rest.trade.exchange
//     );
//
//     tracing::debug!("[insert_trade_rest] sql: {}", &sql);
//
//     // run query; results come as messages from the tokio_postgres crate
//     if let Ok(result_vec) = client.simple_query(&sql).await {
//         for i in result_vec {
//             match i {
//                 SimpleQueryMessage::CommandComplete(row_count) => {
//                     tracing::debug!("[insert_trade_rest] {} row(s) inserted", row_count);
//                 }
//
//                 SimpleQueryMessage::Row(_row) => {}
//
//                 _ => tracing::debug!("[insert_trade_rest] Something weird happened on log query."),
//             }
//         }
//     } else {
//         tracing::debug!("[insert_trade_rest] insert failed");
//     }
// }

/// TODO: convert this to SQLX; it currently does not use a connection pool
async fn insert_minute_bar(client: &Client, mb: &MinuteBar) {
    tracing::debug!("");

    let sql = format!(
        r"
		insert into bar_minute(
			dtg,
			symbol,
			price_open,
			price_high,
			price_low,
			price_close,
			volume
		)
		values ('{}','{}',{},{},{},{},{});",
        mb.dtg, mb.symbol, mb.price_open, mb.price_high, mb.price_low, mb.price_close, mb.volume
    );

    tracing::debug!("[insert_minute_bar] sql: {}", &sql);

    // run query
    if let Ok(result_vec) = client.simple_query(&sql).await {
        for i in result_vec {
            match i {
                SimpleQueryMessage::CommandComplete(row_count) => {
                    tracing::debug!("[insert_minute_bar] {} row(s) inserted", row_count);
                }

                SimpleQueryMessage::Row(_row) => {}
                _ => tracing::debug!("[insert_minute_bar] Something weird happened on log query."),
            }
        }
    } else {
        tracing::debug!("[insert_minute_bar] insert failed");
    }
}

/// insert a single FinnHub trade into the trade_fh table
async fn insert_finnhub_trade(trade: &FinnhubTrade, pool: &PgPool) -> Result<PgQueryResult,sqlx::Error>{

    sqlx::query!(
        r#"
            insert into trade_fh (dtg,symbol, price, volume) values ($1, $2, $3, $4)
        "#,
        trade.dtg.naive_utc(),
        trade.symbol,
        trade.price,
        trade.volume
    ).execute(pool).await

}


/// Continuously overwrite only the latest trade for a given symbol so we have a fast way of getting the most recent price.
async fn insert_finnhub_trade_latest(trade: &FinnhubTrade, pool: &PgPool) ->Result<PgQueryResult,sqlx::Error>{
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
    ).execute(pool).await

}


/// Insert an Alpaca trade received on the websocket
async fn insert_alpaca_trade(t:&AlpWsTrade, pool:&PgPool) ->Result<PgQueryResult,sqlx::Error>{
    sqlx::query!(r#"
        insert into trade_alp (dtg, symbol, price, size)
        values ($1, $2, $3, $4)
    "#, t.dtg.naive_utc(), t.symbol, t.price, t.size).execute(pool).await
}

/// Continuously overwrite only the latest trade for a given symbol so we have a fast way of getting the most recent price.
async fn insert_alpaca_trade_latest(trade: &AlpWsTrade, pool: &PgPool) ->Result<PgQueryResult,sqlx::Error>{
    sqlx::query!(r#"insert into trade_alp_latest(dtg,symbol,price,size)
            values ($1, $2, $3, $4)
            on conflict (symbol) do update set dtg=$1, price=$3, size=$4;
        "#,
        trade.dtg.naive_utc(),trade.symbol,trade.price,trade.size
    ).execute(pool).await
}

/// Append a timestamp to the ping table whenever the Finnhub websocket pings. Use it to determine if the websocket goes down.
async fn insert_finnhub_ping(ping:&FinnhubPing, pool:&PgPool) -> Result<PgQueryResult,sqlx::Error> {
    sqlx::query!(r#"insert into ping_finnhub (ping) values ($1)"#,ping.dtg.naive_utc()).execute(pool).await
}

/// Append a timestamp to the ping table whenever the Finnhub websocket pings. Use it to determine if the websocket goes down.
async fn insert_alpaca_ping(ping:&AlpacaPing, pool:&PgPool) -> Result<PgQueryResult,sqlx::Error> {
    sqlx::query!(r#"insert into ping_alpaca (ping) values ($1)"#, ping.dtg.naive_utc()).execute(pool).await
}


