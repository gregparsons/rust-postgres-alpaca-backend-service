//! alpaca_api_structs
//!
//! websocket and rest api structs

use std::fmt;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::Display;
use crate::alpaca_order::Order;

#[derive(Deserialize, Serialize, Debug)]
pub struct RequestAuthenticate {
    pub action: RequestAction,
    pub key: String,
    pub secret: String,
}

// { "action": "listen", "data": { "streams": ["T.TSLA", "Q.TSLA", "Q.AAPL", "T.AAPL"]}}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RequestListen {
    pub action: RequestAction,
    pub data: RequestListenData,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RequestListenData {
    pub streams: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RequestAction {
    Auth,
    Listen,
    TradeUpdates,
    AccountUpdates,
    // {"action":"subscribe","bars":["TSLA","ARNC","BBAI","FFIE","ARVL","SKLZ","LYG","AMZN","AAPL","T","SOFI","PLUG","WBD","NIO","BRDS","PACW","MULN","AMD"],"trades":["TSLA","ARNC","BBAI","FFIE","ARVL","SKLZ","LYG","AMZN","AAPL","T","SOFI","PLUG","WBD","NIO","BRDS","PACW","MULN","AMD"]}
    Subscribe,
    Ping,
}

/// enable to_string(); print enum in lowercase
impl fmt::Display for RequestAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", snakecase::ascii::to_snakecase(format!("{:?}", self)))
    }
}

#[derive(Serialize, Debug)]
pub struct Ping {
    pub dtg: DateTime<Utc>,
}

#[derive(Debug, PartialEq)]
pub enum WebsocketMessageFormat {
    TextData,
    BinaryUpdates,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", content = "data", tag = "stream")]
pub enum WebsocketMessage {
    Authorization(MesgAuthorization),
    Listening(MesgListening),
    TradeUpdates(MesgOrderUpdate),
    AccountUpdates,
}

#[derive(Debug, Deserialize)]
pub struct MesgAuthorization {
    pub status: AuthStatus,
    pub action: AuthAction,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MesgListening {
    pub streams: Vec<String>
}

/*


"{\"stream\":\"trade_updates\",\"data\":{\"event\":\"new\",\"timestamp\":\"2023-07-17T15:46:32.738504149Z\",\"order\":{\"id\":\"506f20d6-0921-41a2-9938-653f2481383c\",\"client_order_id\":\"2cc88458-0b51-4ec6-8c9b-6b27079943b3---8f0905f0-e2e6-4db1-baae-153c0c63e700\",\"created_at\":\"2023-07-17T15:46:32.729647106Z\",\"updated_at\":\"2023-07-17T15:46:32.740005225Z\",\"submitted_at\":\"2023-07-17T15:46:32.738505019Z\",\"filled_at\":null,\"expired_at\":null,\"cancel_requested_at\":null,\"canceled_at\":null,\"failed_at\":null,\"replaced_at\":null,\"replaced_by\":null,\"replaces\":null,\"asset_id\":\"b5a245fd-cf59-4eb8-878c-97241b9dd807\",\"symbol\":\"PACW\",\"asset_class\":\"us_equity\",\"notional\":null,\"qty\":\"1\",\"filled_qty\":\"0\",\"filled_avg_price\":null,\"order_class\":\"\",\"order_type\":\"limit\",\"type\":\"limit\",\"side\":\"sell\",\"time_in_force\":\"day\",\"limit_price\":\"8.21\",\"stop_price\":null,\"status\":\"new\",\"extended_hours\":true,\"legs\":null,\"trail_percent\":null,\"trail_price\":null,\"hwm\":null},\"execution_id\":\"4e1027dc-0fee-4a29-b48b-fe4253d998d9\"}}"



 */
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum MesgOrderUpdate {
    Accepted { order:Order},
    Calculated { order: Order },
    Canceled { timestamp: DateTime<Utc>, order: Order },
    DoneForDay { order: Order },
    Expired { timestamp: DateTime<Utc>, order: Order },
    Fill { timestamp: DateTime<Utc>, price: BigDecimal, qty: BigDecimal, order: Order },
    New { order: Order },
    OrderCancelRejected { order: Order },
    OrderReplaceRejected { order: Order },
    PartialFill { timestamp: DateTime<Utc>, price: BigDecimal, qty: BigDecimal, order: Order },
    PendingCancel { order: Order },
    PendingNew { order: Order },
    PendingReplace { order: Order },
    Rejected { timestamp: DateTime<Utc>, order: Order },
    Replaced { timestamp: DateTime<Utc>, order: Order },
    Stopped { order: Order },
    Suspended { order: Order },
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthStatus {
    Authorized,
    Unauthorized,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthAction {
    Authenticate,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "T")]
pub enum DataMessage{

    Success(DataMesgSuccess),

    Subscription(DataMesgSubscriptionList),

    #[serde(rename = "t")]
    Trade(AlpacaTradeWs),

    #[serde(rename = "b")]
    Bar,

    #[serde(rename = "q")]
    Quote,

    #[serde(rename = "d")]
    DailyBar,

    #[serde(rename = "s")]
    Status,

    Error,

}

/// [{"T":"success","msg":"connected"}]
/// [{"T":"success","msg":"authenticated"}]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag="msg")]
pub enum DataMesgSuccess {
    Connected,
    Authenticated,
}

/*
[{"T":"subscription",
"trades":["AAPL","AMD","AMZN","ARNC","ARVL","BBAI","BRDS","FFIE","LYG","MULN","NIO","PACW","PLUG","SKLZ","SOFI","T","TSLA","WBD"],
"quotes":[],"bars":["AAPL","AMD","AMZN","ARNC","ARVL","BBAI","BRDS","FFIE","LYG","MULN","NIO","PACW","PLUG","SKLZ","SOFI","T","TSLA","WBD"],
"updatedBars":[],
"dailyBars":[],
"statuses":[],
"lulds":[],
"corrections":["AAPL","AMD","AMZN","ARNC","ARVL","BBAI","BRDS","FFIE","LYG","MULN","NIO","PACW","PLUG","SKLZ","SOFI","T","TSLA","WBD"],
"cancelErrors":["AAPL","AMD","AMZN","ARNC","ARVL","BBAI","BRDS","FFIE","LYG","MULN","NIO","PACW","PLUG","SKLZ","SOFI","T","TSLA","WBD"]}]
 */
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DataMesgSubscriptionList {
    pub trades:Vec<String>,
    pub quotes:Vec<String>,
    #[serde(rename="updatedBars")]
    pub updated_bars:Vec<String>,
    #[serde(rename="cancelErrors")]
    pub cancel_errors:Vec<String>,
    pub corrections:Vec<String>,
    #[serde(rename="dailyBars")]
    pub daily_bars:Vec<String>,
    pub statuses:Vec<String>,
    pub lulds:Vec<String>,
}

/// AlpacaTradeWs
///         Attribute 	Type 	Notes
///         T 	string 	message type, always “t”
///         S 	string 	symbol
///         i 	int 	trade ID
///         x 	string 	exchange code where the trade occurred
///         p 	number 	trade price
///         s 	int 	trade size
///         t 	string 	RFC-3339 formatted timestamp with nanosecond precision
///         c 	array 	trade condition
///         z 	string 	tape
///
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AlpacaTradeWs {

    // not needed with #[serde(tag="T")] in DataMessage
    // #[serde(rename = "T")]
    // pub event: String,

    #[serde(rename = "S")]
    pub symbol: String,

    #[serde(rename = "i")]
    pub id_trade: usize,

    #[serde(rename = "x")]
    pub exchange: String,

    #[serde(rename = "p")]
    pub price: BigDecimal,

    #[serde(rename = "s")]
    pub size: BigDecimal,

    #[serde(rename = "t")]
    pub dtg: DateTime<Utc>,

    // #[serde(default)]
    // pub c:Vec<usize>,

    #[serde(rename = "z")]
    pub id_tape: String,

    // #[serde(default = "Utc::now")]
    // pub dtg_updated: DateTime<Utc>,
}

/*


    {
        "ev": "Q",
        "T": "SPY",
        "x": 17,
        "p": 283.35,
        "s": 1,
        "X": 17,
        "P": 283.4,
        "S": 1,
        "c": [1],
        "t": 1587407015152775000
    }
// #[derive(Deserialize, Serialize, Debug, Clone)]
// struct AlpacaStreamQuote {
//     stream: String,
//     data: AlpWsQuote,
// }

#[derive(Deserialize, Serialize, Debug, Clone)]

pub struct AlpWsQuote {

    /*
        Attribute 	Type 	Notes
        T 	string 	message type, always “q”
        S 	string 	symbol
        ax 	string 	ask exchange code
        ap 	number 	ask price
        as 	int 	ask size
        bx 	string 	bid exchange code
        bp 	number 	bid price
        bs 	int 	bid size
        s 	int 	trade size
        t 	string 	RFC-3339 formatted timestamp with nanosecond precision
        c 	array 	quote condition
        z 	string 	tape
    */
    #[serde(rename = "T")]
    pub event: String,

    #[serde(rename = "S")]
    pub symbol: String,

    #[serde(rename = "s")]
    pub size_trade: usize,

    // exchange code for bid quote
    #[serde(rename = "bx")]
    pub exchange_bid: usize,

    #[serde(rename = "bp")]
    pub price_bid: BigDecimal,

    #[serde(rename = "bs")]
    pub size_bid: usize,

    // exchange code for ask quote
    #[serde(rename = "ax")]
    pub exchange_ask: usize,

    #[serde(rename = "ap")]
    pub price_ask: BigDecimal,

    #[serde(rename = "as")]
    pub size_ask: usize,

    // condition flags
    // pub c:Vec<usize>,

    // timestamp nanoseconds
    // #[serde(with = "ts_nanoseconds")]
    #[serde(rename = "t")]
    pub dtg: String, // DateTime<Utc>,

    #[serde(default = "Utc::now")]
    pub dtg_updated: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Quote {
    pub status: Status,
    pub data: Data,

    {
        "BTC": Object({
                "circulating_supply": Number(18530887),
                "cmc_rank": Number(1),
                "date_added": String("2013-04-28T00:00:00.000Z"),
                "id": Number(1),
                "is_active": Number(1),
                "is_fiat": Number(0),
                "last_updated": String("2020-11-01T02:41:02.000Z"),
                "max_supply": Number(21000000),
                "name": String("Bitcoin"),
                "num_market_pairs": Number(9191),
                "platform": Null,
                "quote": Object({"USD": Object({"last_updated": String("2020-11-01T02:41:02.000Z"), "market_cap": Number(254545818840.16373), "percent_change_1h": Number(0.14435433), "percent_change_24h": Number(1.0432072), "percent_change_7d": Number(4.47102129), "price": Number(13736.299770224909), "volume_24h": Number(30562293700.698463)})}),
                "slug": String("bitcoin"),
                "symbol": String("BTC"),
                "tags": Array([String("mineable"), String("pow"), String("sha-256"), String("store-of-value"), String("state-channels")]),
                "total_supply": Number(18530887)
            }
        )
    }
    // timestamps can be in string here because they insert fine to postgres as an rfc string
    // pub id:i64,
    // pub dtg:chrono::DateTime<chrono::FixedOffset>,

    // #[serde(rename = "timestamp")]
    // pub dtg:String,
    // pub symbol:String,
    // pub price:f64,
    // pub qt_mkt_cap:f64,
    // pub qt_vol_24:f64,
    // pub qt_updated:chrono::DateTime<chrono::FixedOffset>,
    // pub last_updated:String,
}
*/

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinuteBar {
    #[serde(rename = "T")]
    msg_type: String,
    #[serde(rename = "S")]
    pub symbol: String,
    #[serde(rename = "o")]
    pub price_open: BigDecimal,
    #[serde(rename = "h")]
    pub price_high: BigDecimal,
    #[serde(rename = "l")]
    pub price_low: BigDecimal,
    #[serde(rename = "c")]
    pub price_close: BigDecimal,
    #[serde(rename = "v")]
    pub volume: usize,
    #[serde(rename = "t")]
    pub dtg: DateTime<Utc>,
}



/*
    {
        "ev": "T",
        "T": "SPY",
        "i": 117537207,
        "x": 2,
        "p": 283.63,
        "s": 2,
        "t": 1587407015152775000,
        "c": [
        14,
        37,
        41
        ],
        "z": 2
    }
*/

// #[derive(Deserialize, Serialize, Debug, Clone)]
// pub struct Trade {
//
//     // #[serde(with = "ts_nanoseconds")]
//     #[serde(rename = "t")]
//     // pub dtg: String,
//     pub dtg: DateTime<Utc>,
//
//     #[serde(rename = "x")]
//     pub exchange: String, //	"x": "C",
//
//     #[serde(rename = "p")]
//     pub price: BigDecimal, // "p": 387.62,
//
//     #[serde(rename = "s")]
//     pub size: u64, // "s": 100,
// }

// /// AlpacaTradeRest
// /// Ok(Object({"symbol": String("BAC"), "trade": Object({"c": Array([String(" ")]), "i": Number(55359749378617), "p": Number(39.57), "s": Number(100), "t": String("2022-04-12T16:03:26.419177394Z"), "x": String("V"), "z": String("A")})}))
// /// ref: https://docs.rs/chrono/0.4.19/chrono/serde/ts_nanoseconds/index.html
// #[derive(Deserialize, Serialize, Debug, Clone)]
// pub struct AlpacaTradeRest {
//     #[serde(default)]
//     pub symbol: String,
//     pub trade: Trade,
//     // #[serde(with = "ts_nanoseconds")]
//     // #[serde(rename = "timestamp")]
//     // pub dtg:DateTime<Utc>,
//     // pub price:Decimal,
//     // pub size:u64,
//     // pub exchange:u64,
//     // pub cond1:u64,
//     // pub cond2:u64,
//     // pub cond3:u64,
//     // pub cond4:u64,
//     #[serde(default = "Utc::now")]
//     pub dtg_updated: DateTime<Utc>,
// }


// TODO: trade parse test
// [{"T":"t","S":"TSLA","i":5471,"x":"V","p":286.39,"s":100,"c":["@"],"z":"C","t":"2023-07-17T15:21:55.787214158Z"}]
// [{"T":"t","S":"T","i":54191554971494,"x":"V","p":13.77,"s":41,"c":[" ","I"],"z":"A","t":"2023-07-17T15:21:55.585613056Z"},{"T":"t","S":"T","i":54191554971509,"x":"V","p":13.77,"s":100,"c":[" "],"z":"A","t":"2023-07-17T15:21:55.585622016Z"},{"T":"t","S":"T","i":54191554971717,"x":"V","p":13.77,"s":59,"c":[" ","I"],"z":"A","t":"2023-07-17T15:21:55.58563712Z"}]

#[derive(Debug, Display, Clone)]
pub enum CrossStatus{
    Up,
    Down,
    None,
}