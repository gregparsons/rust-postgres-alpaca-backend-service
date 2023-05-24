//!
//! remote_position.rs
//!
//! Sync positions and orders to a local data structure from the remote.
//!
//! Why?
//! 1. If want to buy, check here before buying. If there's already an outstanding order OR position, don't buy.
//! 2. If want to sell, check here, there needs to be an outstanding position to actually sell. (or really
//! make sure the settings to disallow short sales is turned off.
//!
// use std::collections::HashMap;
use crate::position::Position;
// use once_cell::sync::Lazy;
// use reqwest::header::HeaderMap;
// use crate::position::Position;
// use crate::order::Order;
use crate::settings::Settings;

// pub static POSITION_HMAP:Lazy<Mutex<RemotePosition>> = Lazy::new(||{
//     let m = RemotePosition::new();
//     Mutex::new(m)
// });

/// Data structure to index positions and orders
pub struct RemotePosition {
    // Positions indexed by Symbol; there won't be more than one position for a stock symbol
    // pub positions: HashMap<String, Position>,
    // Orders indexed by Symbol then order ID; there can be multiple orders for the same stock symbol (for example two limit orders with different limits)
    // pub orders: HashMap<String, HashMap<String, Order>>

}

impl RemotePosition {

/*    pub fn new()-> RemotePosition {
        RemotePosition {
            // a hashmap of hashmaps first indexed by stock symbol then by position asset_id (which is effectively the same thing...)
            // positions: HashMap::new(),
            // orders: HashMap::new(),
        }
    }*/

    /// Call the Alpaca API to get the remote position snapshot
    ///
    /// populate the positions hashmap
    pub async fn refresh_remote_positions(settings:&Settings) -> Result<Vec<Position>, reqwest::Error> {

        let mut headers = reqwest::header::HeaderMap::new();

        // TODO: add a setting for USE_PAPER_OR_LIVE
        let api_key_id = settings.alpaca_paper_id.clone();
        let api_secret = settings.alpaca_paper_secret.clone();

        headers.insert("APCA-API-KEY-ID", api_key_id.parse().unwrap());
        headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());

        // get the position list from the positions API
        let client = reqwest::Client::new();
        let remote_positions:Vec<Position> = client.get("https://paper-api.alpaca.markets/v2/positions")
            .headers(headers)
            .send()
            .await?
            .json()
            .await?;

        // tracing::debug!("[get_positions] remote positions: {:?}", &remote_positions);
        // tracing::debug!("[get_positions] self.positions: {:?}", self.positions);

        // Load the vector of Position objects into the static data structure hash map indexed by the stock symbol,
        // then by the asset_id just so there can be no duplicates; this is mainly so the code can be duplicated from
        // orders where there (I think) can be multiple orders of the same stock symbol but of different variety. It
        // doesn't seem there's ever a different asset_id for the same stock symbol, or that Alpaca would ever
        // show the same stock symbol in two different position entries. So asset_id is the unique key (as is
        // symbol.
/*        for remote_posn in &remote_positions {
            match self.positions.entry(remote_posn.symbol.clone()) {
                std::collections::hash_map::Entry::Occupied(mut local_entry) => {
                    // replace what was there with what was found on remote
                    local_entry.insert((*remote_posn).clone());

                }
                std::collections::hash_map::Entry::Vacant(local_vacant) => {
                    // tracing::debug!("[get_positions] entry vacant for {}", &p.symbol);
                    // let posn_hmap = HashMap::from([(position.asset_id.clone(), position)]);
                    local_vacant.insert((*remote_posn).clone());
                }

            }
            // tracing::debug!("[get_positions] self.positions (after insert): {:?}", self.positions);
            // self.positions.insert(p.symbol.clone(), p);
        };*/
        Ok(remote_positions)
    }



/*
    /// Call the Alpaca API to get the remote order snapshot
    ///
    /// populate the order hashmap
    pub async fn refresh_remote_orders(&mut self) -> Result<Vec<Order>, reqwest::Error> {

        // do REST call
        let mut headers = HeaderMap::new();
        let api_key_id = std::env::var("alpaca_id").expect("alpaca_id environment variable not found");
        let api_secret = std::env::var("alpaca_secret").expect("APCA-API-KEY-ID environment variable not found");
        headers.insert("APCA-API-KEY-ID", api_key_id.parse().unwrap());
        headers.insert("APCA-API-SECRET-KEY", api_secret.parse().unwrap());
        let client = reqwest::Client::new();
        let remote_orders:Vec<Order> = client.get("https://paper-api.alpaca.markets/v2/orders")
            .headers(headers)
            .send()
            .await?
            .json()
            .await?;

        // tracing::debug!("[get_positions] remote orders: {:?}", &remote_orders);
        // tracing::debug!("[get_positions] self.orders: {:?}", self.orders);

        // process response
        for new_order in &remote_orders {
            match self.orders.entry(new_order.symbol.clone()) {
                std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                    // tracing::debug!("[get_positions] entry occupied for {}", &p.symbol);
                    let order_hmap: &mut HashMap<String, Order> = occupied_entry.into_mut();
                    let _old_value = order_hmap.insert(new_order.id.clone(), new_order.clone());

                },
                std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                    // tracing::debug!("[get_positions] entry vacant for {}", &p.symbol);
                    let order_hmap:HashMap<String,Order> = HashMap::from([(new_order.id.clone(), new_order.clone())]);
                    vacant_entry.insert(order_hmap);
                },
            }
            // tracing::debug!("[get_positions] self.orders (after insert): {:?}", self.orders);
        };

        Ok(remote_orders)
    }
    */
/*    /// insert a new order, usually received in response to a order submitted
    pub fn insert_order(&mut self, new_order:Order) {

        // find this symbol in the hashmap then push or create a new vector and push if doesn't exist
        // TODO: change this from a vector to a hashmap (indexed by order ID) to deconflict duplicates
        // TODO: give up and use an in-memory database
        match self.orders.entry(new_order.symbol.clone()) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                let order_hmap = occupied_entry.into_mut();
                let _old = order_hmap.insert(new_order.id.clone(), new_order);
            }
            std::collections::hash_map::Entry::Vacant(v) => {
                // tracing::debug!("[get_positions] entry vacant for {}", &p.symbol);
                let order_hmap = HashMap::from([(new_order.id.clone(), new_order)]);
                v.insert(order_hmap);
            },
        }
        tracing::debug!("[insert_order] order inserted: \n{:?}", self.orders);
    }*/

/*    /// print the orders and positions
    pub fn _print_debug(&self){
        println!("[positions] ({})", self.positions.len());
        self.positions.iter().for_each(|p|{
            println!("[position] {:?}", p);
        });
        println!("[orders] {}", self.orders.len());
        self.orders.iter().for_each(|o|{
            println!("[order] {:?}", o);
        });
    }*/

    // /// check if a position in this stock already exists or if a buy order is pending.
    // ///
    // /// TODO: differentiate between buy and sell orders and only care here if it's the buy side
    // ///
    // /// expects symbol ALL caps
    // pub fn position_exists(&self, symbol: &str) -> Option<&Position> {
    //     self.positions.get(symbol.to_uppercase().as_str())
    // }

/*    /// does an order to buy this stock exist (aka: long buy order)?
    pub fn buy_order_exists(&self, symbol: &str) -> bool {
        // tracing::debug!("[buy_order_exists]...");
        let order_exists = match self.orders.get(symbol){
            Some(order_hmap) => {
                let test:Vec<Order> = order_hmap.values().map(|x| x.clone()).filter(|x:&Order| x.side == "buy" ).collect();  // cloned().collect::<Vec<Order>>().iter().filter(|&order| &order.side == "buy").collect::<Vec<&Order>>();
                let buy_order_len = test.len();
                // println!("[buy_order_exists] buy_order_len: {}", &buy_order_len);
                if buy_order_len > 0{
                    true
                } else {
                    false
                }
            },
            None => false
        };
        // tracing::debug!("[buy_order_exists] {}", &order_exists);
        order_exists
    }*/

/*    /// does an order to sell this stock exist (aka: short sell order)?
    /// TODO: don't duplicate all this code
    pub fn sell_order_exists(&self, symbol: &str) -> bool {
        // tracing::debug!("[sell_order_exists]...");
        let order_exists = match self.orders.get(symbol){
            Some(order_hmap) => {
                let test:Vec<Order> = order_hmap.values().map(|x| x.clone()).filter(|x:&Order| x.side == "sell" ).collect();  // cloned().collect::<Vec<Order>>().iter().filter(|&order| &order.side == "buy").collect::<Vec<&Order>>();
                let sell_order_len = test.len();
                // println!("[sell_order_exists] sell_order_len: {}", &sell_order_len);
                if sell_order_len > 0{
                    true
                } else {
                    false
                }
            },
            None => false
        };
        // tracing::debug!("[sell_order_exists] {}", &order_exists);
        order_exists

    }*/

}



/*
/// Get orders and positions via the rest api.
///
/// TODO: make idempotent; no duplicates when called multiple times
/// TODO: what happens when remote deletions happen? Is it best to to just clear out the local list every time?
/// TODO: the solution to both the above problems would be to delete all local entries before calling these
/// load functions. Instead of trying to one by one reconcile.
///
pub async fn refresh_remote_alpaca(settings:&Settings){

    let mut remotes = POSITION_HMAP.lock().unwrap();

    remotes.positions.clear();
    // remotes.orders.clear();

    // get any remote positions
    if let Ok(positions) = remotes.refresh_remote_positions(settings).await{
        for position in positions {
            tracing::debug!("[refresh_remote_orders] position: {}", &position);
        }
    }

    // // get any remote orders
    // if let Ok(orders) = remotes.refresh_remote_orders().await{
    //     for order in orders {
    //         tracing::debug!("[refresh_remote_orders] order: {}", &order);
    //     }
    // }
}*/





