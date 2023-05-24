//! trade_setting_profile.rs
//!
//!

use std::fmt;
use serde::Deserialize;

/// strongly typed path; fail strongly if someone types anything other than these in the web path
#[derive(Deserialize, Debug)]
pub enum SettingsProfile {
    #[serde(rename="buy")]
    Buy,
    #[serde(rename="close")]
    Close,
    #[serde(rename="close_2")]
    Close2,

}

impl fmt::Display for SettingsProfile {

    /// enable to_string()
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}