use crate::formatter::*;
use color_eyre::eyre::{Context, Result, eyre};
use schemars::{JsonSchema, Schema, schema_for};
use serde::{Deserialize, Deserializer};
use std::{collections::HashMap, fmt::Display, fs, path::PathBuf, time::Duration};

mod defaults;
use defaults::*;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConfigFile {
    pub configs: Vec<Config>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Config {
    /// Name of this config, use `kdeconnect_waybar --config <name>` in the `exec` field of the module config to start this config
    /// Blank means default config
    pub name: Option<String>,

    /// A kdeconnect device id, e.g. `"4cc0978ea8b44b2fa33c188711071a9c"`
    /// Tells the app to specifically use this device for this config
    /// Device ids can be obtained with command `kdeconnect-cli -l`
    pub device_id: Option<String>,

    #[serde(deserialize_with = "deserialize_duration_secs")]
    #[serde(default = "default_update_interval")]
    #[schemars(with = "f64")]
    /// The interval at which the waybar module text refreshes in seconds
    /// Default is 5
    pub update_interval_secs: Duration,

    #[schemars(with = "String")]
    /// The default [`GlobalFormat`] used for the module text
    pub format: GlobalFormat,
    #[schemars(with = "Option<String>")]
    /// The default [`GlobalFormat`] used for the module tooltip text
    pub tooltip_format: Option<GlobalFormat>,

    #[serde(default = "default_device_not_found_text")]
    /// The [`GlobalFormat`] used for the module text when kdeconnect isn't running or when device isn't connected
    pub device_not_found_text: String,
    #[serde(default = "default_device_not_found_tooltip_text")]
    /// The [`GlobalFormat`] used for the module tooltip text when kdeconnect isn't running or when device isn't connected
    pub device_not_found_tooltip_text: String,

    #[serde(default = "default_is_charging_text")]
    /// The text replacing {[Battery::IsChargingText]} (in any [`GlobalFormat`]) when device is charging
    /// Can contain Nerd-Font icons
    /// e.g. `"󰂄 Charging... "`
    pub is_charging_text: String,
    #[serde(default = "default_isnt_charging_text")]
    /// The text replacing {[Battery::IsChargingText]} (in any [`GlobalFormat`]) when device isn't charging
    /// Can contain Nerd-Font icons
    /// `"󱟩 Not charging"`
    pub isnt_charging_text: String,

    #[serde(default = "default_charge_ranges")]
    /// An array of battery charge ranges values
    /// e.g. [25, 50, 75] => contains 4 ranges 0-25, 25-50, 50-75, 75-100
    /// used alongside [`Config::is_charging_texts`] and [`Config::isnt_charging_texts`]
    pub charge_ranges: Vec<i64>,
    #[serde(default = "default_is_charging_texts")]
    /// Can contain Nerd-Font icons
    /// used alongside [`Config::charge_ranges`], must contains len([`Config::charge_ranges`])+1 strings
    /// When device is charging will replace {[`Battery::ChargeTexts`]} in any format with the nth string,
    /// corresponding to the nth charge range the device battery charge is into
    /// e.g. ["󰢜", "󰂆", "󰂇", "󰂈", "󰢝", "󰂉", "󰢞", "󰂊", "󰂋", "󰂅"] or ["Critical", "Low", "Good", "Super-charged"]
    pub is_charging_texts: Vec<String>,
    #[serde(default = "default_isnt_charging_texts")]
    /// used alongside [`Config::charge_ranges`], must contains len([`Config::charge_ranges`])+1 strings
    /// When device isn't charging will replace {[`Battery::ChargeTexts`]} in any format with the nth string,
    /// corresponding to the nth charge range the device battery charge is into
    /// e.g. ["󰁺","󰁻","󰁼","󰁽","󰁾","󰁿","󰂀","󰂁","󰂂","󰁹"] or ["Critical", "Low", "Good", "Super-charged"]
    /// Can contain Nerd-Font icons
    pub isnt_charging_texts: Vec<String>,

    #[serde(default = "default_device_phone_text")]
    /// Will replace {[`DeviceInfo::DeviceTypeText`]} in any [`GlobalFormat`] if device is a phone
    ///  e.g. `"Phone "`,
    /// Can contain Nerd-Font icons
    pub device_phone_text: String,
    #[serde(default = "default_device_tablet_text")]
    /// Will replace {[`DeviceInfo::DeviceTypeText`]} in any [`GlobalFormat`] if device is a tablet
    /// e.g. `"Tablet "`
    /// Can contain Nerd-Font icons
    pub device_tablet_text: String,

    #[schemars(with = "Option<String>")]
    /// Groups notifications per app, and for each app replaces {[`Notification::Grouped`]} with the given [`NotificationFormat`]
    pub notification_grouped_format: NotificationFormat,
    #[schemars(with = "Option<String>")]
    /// For each notification replaces {[`Notification::Single`]} with the given [`NotificationFormat`]
    pub notification_single_format: NotificationFormat,
    /// A dictionary with ints as keys and text strings as values
    /// When in a [`Grouped`](Notification::Grouped) [`NotificationFormat`] replaces {[`CountText`](NotificationFormatField::CountText)} with the given string matching the amount of notifications for this app
    /// 0 is a special key that is used when the notification count of the app doesn't match any other keys
    /// e.g.:
    /// - `{1: "One", 2: "Two", 0: "Three or more"}`
    /// - or with Nerd-Font icons `{"1": "󰲠","2": "󰲢","3": "󰲤","4": "󰲦","5": "󰲨","6": "󰲪","7": "󰲬","8": "󰲮","9": "󰲰","0": "󰲲"}`
    /// Can contain Nerd-Font icons
    pub notifications_count_text: HashMap<i64, String>,
    #[serde(default)]
    /// When in a [`Grouped`](Notification::Grouped) [`NotificationFormat`] replaces {[`CustomIcon`](NotificationFormatField::CustomIcon)} with the given string matching the amount of notifications for this app
    /// Recommended with Nerd-Font icons
    /// A dictionary with app names as keys and text strings as values
    /// WARNING: app names are case-sensitive: for example youtube should be YouTube
    pub app_icons: HashMap<String, String>,
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Needs better implementation, used in context when config produces errors
        f.write_str(&format!("{:?}", self))
    }
}

fn deserialize_duration_secs<'de, D>(d: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Duration::from_secs_f64(f64::deserialize(d)?))
}

impl ConfigFile {
    pub const DIR_NAME: &'static str = env!("CARGO_PKG_NAME");
    pub const FILE_NAME: &'static str = "config.json";

    pub fn dir() -> Result<PathBuf> {
        Ok(dirs::config_dir()
            .ok_or(eyre!("Unable to find config dir"))?
            .join(Self::DIR_NAME))
    }

    pub fn config_file_path() -> Result<PathBuf> {
        Ok(Self::dir()?.join(Self::FILE_NAME))
    }

    pub fn read_all() -> Result<Self> {
        let path = Self::config_file_path()?;
        let config_str = fs::read_to_string(&path)
            .with_context(move || path.into_os_string().into_string().unwrap())?;
        let config = serde_json::from_str(&config_str).with_context(|| config_str)?;

        Ok(config)
    }

    pub fn schema() -> Schema {
        schema_for!(Self)
    }
}
