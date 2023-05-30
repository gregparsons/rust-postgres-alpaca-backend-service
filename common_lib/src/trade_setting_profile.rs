//! trade_setting_profile.rs
//!
//!

use serde::Deserialize;
use strum::Display;

/// strongly typed path; fail strongly if someone types anything other than these in the web path
// #[derive(Deserialize, Debug)]
#[derive(Display, Deserialize, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum SettingsProfile {
    #[serde(rename="buy")]
    Buy,
    #[serde(rename="close")]
    Close,
    #[serde(rename="close_2")]
    Close2,

}
