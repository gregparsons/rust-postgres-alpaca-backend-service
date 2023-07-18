//! alpaca_order_log


use chrono::{DateTime, Utc};
use crate::alpaca_order::Order;

#[derive(Debug)]
pub struct AlpacaOrderLogEvent{
    pub dtg:DateTime<Utc>,
    pub event: String,
    pub order: Order,

}

impl AlpacaOrderLogEvent{

    /// Save a single order to the database
    pub async fn save_to_db(&self, pool: &sqlx::PgPool) {
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
            self.dtg,
            self.event,
            self.order.id,
            self.order.client_order_id,
            self.order.symbol.to_lowercase(),
            self.order.qty,
            self.order.filled_qty,
            self.order.filled_avg_price,
            self.order.side.to_string().to_lowercase(),
            self.order.order_type_v2.to_string(),

            // dates 10-13
            self.order.created_at,
            self.order.updated_at,
            self.order.submitted_at,
            self.order.filled_at,
            self.order.expired_at,
            self.order.canceled_at,
            self.order.failed_at

            // self.time_in_force.to_string(), // $13
            // self.limit_price,
            // self.stop_price,
            // self.status
        ).execute(pool).await;
        tracing::debug!("[save_to_db] result: {:?}", result);



    }



}