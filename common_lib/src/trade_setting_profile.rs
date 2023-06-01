//! trade_setting_profile.rs
//!
//!

use serde::Deserialize;
use strum::Display;

/// strongly typed path; fail strongly if someone types anything other than these in the web path
// #[derive(Deserialize, Debug)]
#[derive(Display, Deserialize, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum TradeSettingsProfile {

    // these are needed by actix to deserialize the paths from lowercase to the enum; not sure if strum has this capability
    #[serde(rename="buy")]
    Buy,
   #[serde(rename="buy_two")]
    BuyTwo,
   #[serde(rename="close")]
    Close,
   #[serde(rename="close_with_loss")]
    CloseWithLoss,

    // TODO: manually typing in a path other than one of these results in:
    // unknown variant `close_with_loss`, expected one of `buy`, `close`, `close_2`
    // from the deserialization attempt

}
