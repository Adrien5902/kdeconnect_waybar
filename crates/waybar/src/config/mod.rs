use color_eyre::eyre::{Context, Result, eyre};
use schemars::{JsonSchema, Schema, schema_for};
use serde::{Deserialize, Deserializer};
use std::{fmt::Display, fs, time::Duration};

use crate::formatter::Format;

mod defaults;
use defaults::*;

pub const CONFIG_FILE: &'static str = "kdeconnect_waybar.json";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Config {
    pub name: Option<String>,
    pub device_id: Option<String>,
    #[serde(deserialize_with = "deserialize_duration_secs")]
    #[serde(default = "default_update_interval")]
    #[schemars(with = "f64")]
    pub update_interval_secs: Duration,

    #[schemars(with = "String")]
    pub format: Format,
    #[schemars(with = "String")]
    pub tooltip_format: Option<Format>,

    #[serde(default = "default_device_not_found_text")]
    pub device_not_found_text: String,
    #[serde(default = "default_device_not_found_tooltip_text")]
    pub device_not_found_tooltip_text: String,

    #[serde(default = "default_is_charging_text")]
    pub is_charging_text: String,
    #[serde(default = "default_isnt_charging_text")]
    pub isnt_charging_text: String,

    #[serde(default = "default_charge_ranges")]
    pub charge_ranges: Vec<i64>,
    #[serde(default = "default_is_charging_texts")]
    pub is_charging_texts: Vec<String>,
    #[serde(default = "default_isnt_charging_texts")]
    pub isnt_charging_texts: Vec<String>,

    #[serde(default = "default_device_phone_text")]
    pub device_phone_text: String,
    /// DeviceInfo:
    #[serde(default = "default_device_tablet_text")]
    pub device_tablet_text: String,
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Needs better implementation, used in context when config produces errors
        f.write_str(&format!("{:?}", self))
    }
}

fn deserialize_duration_secs<'de, D>(d: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Duration::from_secs_f64(f64::deserialize(d)?))
}

impl Config {
    pub fn read_all() -> Result<Vec<Self>> {
        let path = dirs::config_dir()
            .ok_or(eyre!("Unable to find config dir"))?
            .join(CONFIG_FILE);

        let config_str = fs::read_to_string(&path)
            .with_context(move || path.into_os_string().into_string().unwrap())?;
        let config = serde_json::from_str(&config_str)?;

        Ok(config)
    }

    pub fn schema() -> Schema {
        schema_for!(Self)
    }
}
