//! AlpacaAPI.rs
//!
//! utilities for the alpaca API
//!
//!

use bigdecimal::{BigDecimal, FromPrimitive};
use crossbeam_channel::Sender;

use crate::account::Account;
use crate::alpaca_order::Order;
use crate::alpaca_transaction_status::{AlpacaTransaction, BuyResult, TransactionNextStep};
use crate::db::DbMsg;
use crate::error::TradeWebError;
use crate::market_hours::{BUY_EXTENDED_HOURS, SELL_EXTENDED_HOURS};
use crate::order_log_entry::OrderLogEntry;
use crate::settings::Settings;
use crate::symbol::Symbol;
use crate::trade_struct::{JsonTrade, OrderType, TimeInForce, TradeSide};

const QTY_SIZE_SAFETY_LIMIT:usize=1001;



/// Submit a sell order without doing any checking if there's already any sell orders in place, or even a position to sell.
/// Assumes short sells setting is turned off since we don't want short sells for now.
/// market, normal hours
/// TODO: need to get the position to get the correct number of shares to sell
///
/// Side effect: copies the resulting Order to the database
///
/// So...apparently the quantity "available" can be less than the quantity I actually own. No clue what that means
/// except that you're blocked from selling it. So when selling, need to use qty_available instead of qty
/// to determine how many we can sell now. "Held for orders" could be a reason?
///
/// Limit Price: if the Option is None then this is intended to be a Market-priced sale
pub async fn sell(symbol: &str, qty_to_sell: BigDecimal, limit_price:Option<BigDecimal>,
                  settings: &Settings, tx_db:Sender<DbMsg>) -> Option<String> {

    tracing::info!("[alpaca_api::sell] ************** SELL ************** {}, {} shares for {:?}", symbol, qty_to_sell, limit_price);

    // generate a new order and save to the order log
    // not the TransactionLog (which prevents duplicates)
    // TODO: move this to after the sell order is successful; or even after it fills(? requires monitoring websocket and more error prone)
    let order_log_entry = OrderLogEntry::new(symbol.to_string().to_uppercase(), TradeSide::Sell, qty_to_sell.clone());
    let tx_db1 = tx_db.clone();
    let _result = order_log_entry.save(tx_db1);

    let (json_trade, id) = match limit_price{
        Some(limit_price) => {

            // Limit order
            let json = JsonTrade {
                symbol: order_log_entry.symbol(), // to uppercase
                side: TradeSide::Sell,
                time_in_force: TimeInForce::Day,
                qty: qty_to_sell,
                order_type: OrderType::Limit,
                limit_price: Some(limit_price.clone()),
                extended_hours: Some(SELL_EXTENDED_HOURS),
                client_order_id: order_log_entry.id_client(),
            };

            (json,Some(order_log_entry.id_client()))
        },
        None => {

            // Market order: almost always results in a loss in close-quarter trading. Don't bother.

            let json = JsonTrade {
                symbol: order_log_entry.symbol(), // to uppercase
                side: TradeSide::Sell,
                time_in_force: TimeInForce::Day,
                qty: qty_to_sell,
                order_type: OrderType::Market,
                limit_price: None,
                extended_hours: None,
                client_order_id: order_log_entry.id_client()
            };

            (json, None)

        }
    };

    // TODO: use TransactionLog to prevent duplicate sell attempts; not totally a big deal as long as
    // alpaca is set to prevent short sales but it'd be better not to hit the API in the first place
    // if we know there are no shares to sell (ie there exists no entry at all in the transaction_log table)


    let tx_rest = tx_db.clone();

    match post_order(json_trade, settings, tx_rest) {
        Ok(order)=>{
            tracing::info!("[sell] sale order posted: {:?}", &order);
            let tx_db2 = tx_db.clone();
            let _save_result = order.save(tx_db2);
        },
        Err(e) => tracing::error!("[sell] sale error: {:?}", &e)
    }

    id

}

/// round to two digits if the amount is greater than or equal to one dollar, 4 digits if less.
fn fix_alpaca_price_rounding(price:BigDecimal) ->BigDecimal{
    let fixed_price = if price < BigDecimal::from(1) {
        price.round(4)
    } else {
        price.round(2)
    };

    fixed_price
}

/// buy
pub async fn buy(stock_symbol: &Symbol, settings: &Settings, tx_db:Sender<DbMsg>) {

    tracing::info!("[buy] ******************************************************** BUY ********************************************************");
    tracing::info!("[buy] ***** BUY {}: {}", &stock_symbol.symbol, &stock_symbol.trade_size);

    // 1. check if a position or order already exists
    // old, no longer refreshing order table from API
    // let count_result = Position::check_position_and_order(&stock_symbol.symbol, pool).await;

    // TODO: safety checks to prevent buying when data is bad and
    // 1. check t_settings.allow_buy
    // 2. check available cash: t_settings.max - account.equity > one share price at least
    // 3. check account, transaction_status, and trade table updates are recent otherwise default to not buying (websocket probably down)


    // start_buy checks if there's already an order in play; if there is it returns an error
    let tx_db1 = tx_db.clone();

    // TODO: combine the db call to get the stock symbol with Account::buy_decision_cash_available
    let start_result = AlpacaTransaction::buy_check(&stock_symbol.symbol, tx_db1).await;
    tracing::info!("[buy] ***** buy check: {:?}", &start_result);
    match start_result {

        BuyResult::NotAllowed { error } => tracing::debug!("[buy] ***** already a position or order: {:?}", &error),

        BuyResult::Allowed => {

            let max_buy_result = Account::max_buy_possible(&stock_symbol.symbol, tx_db.clone()).await;
            tracing::info!("[buy] ***** max shares: {:?}", &max_buy_result);
            let (qty, limit_price) = match max_buy_result {
                Err(_e)=> {
                    tracing::info!("[buy] ***** cannot buy; error: {:?}", _e);
                    (BigDecimal::from(0), BigDecimal::from(0))
                },
                Ok(buy_possible) => {
                    // minimum of the max possible and max qty allowed
                    tracing::info!("[buy] ***** cash available: {:?}", &buy_possible);
                    let qty = std::cmp::min(buy_possible.qty_possible, stock_symbol.trade_size.clone());
                    (qty, buy_possible.price)
                }
            };
            tracing::info!("[buy] ***** shares to attempt to buy: {}", &qty);

            if qty <= BigDecimal::from(0) {
                // qty we can buy is zero
                tracing::info!("[buy] ***** buying zero: {}", &qty);
            } else {

                // buy the quantity pertaining to the specific stock in the t_symbol table
                tracing::info!("[buy] BUY (minimum of std size and max possible) {}:{}", &stock_symbol.symbol, &qty);

                // catch whatever was causing us to buy 65000 shares
                assert!(qty <= BigDecimal::from_usize(QTY_SIZE_SAFETY_LIMIT).unwrap_or_else(|| BigDecimal::from(300)), "{}",
                        format!("[alpaca_api::buy quantity to sell is not less than {}", QTY_SIZE_SAFETY_LIMIT));

                // generate a new order and save to the order log
                let order_log_entry = OrderLogEntry::new(stock_symbol.symbol.clone(), TradeSide::Buy, qty.clone());

                let tx_db1 = tx_db.clone();
                let _result = order_log_entry.save(tx_db1);

                // 422: errors:
                // extended hours order must be DAY limit orders
                // market orders require no stop or limit price



                // TODO: only use buy limit price during after hours

                let (limit_price_opt, order_type) = match BUY_EXTENDED_HOURS {
                    false => (None, OrderType::Market),
                    true => (Some(fix_alpaca_price_rounding(limit_price)), OrderType::Limit),
                };


                // JSON for alpaca API
                let json_trade = JsonTrade {
                    symbol: order_log_entry.symbol(), // to uppercase,
                    side: TradeSide::Buy,
                    time_in_force: TimeInForce::Day,
                    qty: qty.clone(),
                    order_type: order_type,
                    limit_price: limit_price_opt,
                    extended_hours: Some(BUY_EXTENDED_HOURS),
                    client_order_id: order_log_entry.id_client(),
                };

                // Delete the new transaction in alpaca_transaction_status if posting an order fails
                // We know the order was newly created and currently set to 0.0 shares since it allowed
                // creating a new order above.
                let tx_rest1 = tx_db.clone();
                let next_step = match post_order(json_trade, settings, tx_rest1) {
                    Ok(order) => {
                        let tx_db1 = tx_db.clone();
                        order.save(tx_db1);
                        TransactionNextStep::Continue
                    },
                    Err(_) => TransactionNextStep::DeleteTransaction
                };

                if next_step == TransactionNextStep::DeleteTransaction {
                    let tx_db2 = tx_db.clone();
                    AlpacaTransaction::delete_one(&stock_symbol.symbol, tx_db2);
                }
            }
        }
    }
    tracing::info!("[buy] ******************************************************** END BUY ****************************************************");
}

/// do the REST part of the orders POST API
/// see documentation above
pub fn post_order(json_trade: JsonTrade, settings: &Settings, tx_db:crossbeam_channel::Sender<DbMsg>) -> Result<Order, TradeWebError> {
    let (tx, rx) = crossbeam_channel::unbounded();
    let _ = tx_db.send(DbMsg::RestPostOrder {json_trade: json_trade.clone(), settings:settings.clone(), sender: tx});
    match rx.recv(){
        Ok(order)=> Ok(order),
        Err(_e)=> Err(TradeWebError::ChannelError),
    }
}
