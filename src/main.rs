//! A highly configurable [KDE Connect](https://kdeconnect.kde.org/) module for [Waybar](https://github.com/Alexays/Waybar/)
//!
//! allows you to display many information about your mobile devices (phone or tablet)
//! such as battery, notifications, ...
//!
//! # Configuring
//! This documentation assumes you have installed [Waybar](https://github.com/Alexays/Waybar/) and know how to configure it,
//! as well as [KDE Connect](https://kdeconnect.kde.org/) and already paired a device using it
//!
//! It is also recommended to have a [Nerd-Font](https://www.nerdfonts.com/#home) installed on your Waybar
//!
//! ## 🔧 Installation
//! Check out [Installation](https://github.com/Adrien5902/kdeconnect_waybar#-installation) for detailed installation instructions
//!
//! ## ⚙️ Updating your waybar config
//! Once installed start by adding the module to your waybar's config :
//! ```jsonc
//!~/.config/waybar/config.jsonc
//!
//!"custom/kdeconnect": {
//!    "format": "{}",
//!    "exec": "kdeconnect_waybar", <-- or "kdeconnect_waybar -c <name>" to use a custom config name
//!    "return-type": "json",
//!    "on-click": ""
//!}
//! ```
//!
//! ## ⚠️ Important
//! Before continuing to the next steps I'd recommend you execute the command
//! ```
//! kdeconnect_waybar gen_schema
//! ```
//! for it to generate a json schema file which will tell your IDE what should be in the config file
//!
//!
//! ## ✨ Configuring the module to your taste
//! Then locate the config directory it should be under :
//!
//! `$XDG_CONFIG_HOME/kdeconnect_waybar` or `$HOME/.config/kdeconnect_waybar` e.g. `/home/alice/.config/kdeconnect_waybar`
//!
//! > If it doesn't appear create it manually or run `kdeconnect_waybar`
//!
//! In it make a file called `config.json` with your custom config (hot reloading supported)
//!
//! Here's an example of what it could look like :
//! ```json
//! {
//! 	"$schema": "./config.schema.json",
//! 	"configs": [
//! 		{
//! 			"update_interval_secs": 5,
//! 			"format": "{Battery::ChargePercent}% {Battery::ChargeTexts} {Notification::Grouped}",
//! 			"tooltip_format": "Device type: {DeviceInfo::DeviceTypeText}\nBattery status: {Battery::IsChargingText} {Battery::ChargePercent}% \nNotifications:\n{Notification::Single}",
//! 			"device_not_found_text": "",
//! 			"device_not_found_tooltip_text": "Device not found make sure kdeconnect is running and phone is connected",
//! 			"device_phone_text": "Phone ",
//! 			"device_tablet_text": "Tablet ",
//!         }
//!     ]
//! }
//! ```
//!
//! You may wanna look at [examples](https://github.com/Adrien5902/kdeconnect_waybar/tree/main/examples) for more inspiration !
//!
//! The two final text that will be displayed on your waybar are [`Config::format`] and [`Config::tooltip_format`] see [`GlobalFormat`] to understand how to configure them
//!
//! configs is an array so you can configure multiple ones and use them with `kdeconnect_waybar -c <name>` in your Waybar module `exec` field
//!
//! ## 👀 Look at whole documentation
//! See also [`Config`] to know all that's available for your config
//!
//! ## 🎨 Styling
//! You can edit the module's style by referring to it with `#custom-kdeconnect` in your waybar's css
//!
//! ## 🐞 Bugs and Errors
//! If something appears to be broken, before submitting an issue,
//! try running the program outside out of the waybar (just run `kdeconnect_waybar` in your terminal),
//! if anything goes wrong it will display an error,
//! it is useful for debugging your config (if you misspelled some field for example),
//!
//! If you can pin point the issue or wanna request a new feature then feel free to open an issue [here](https://github.com/Adrien5902/kdeconnect_waybar/issues)

#![feature(once_cell_try)]

use clap::{Command, Parser, arg, command, value_parser};
use color_eyre::eyre::{Result, eyre};
use notify::{Event, EventKind, Watcher};
use serde::Serialize;
use std::{
    borrow::Cow,
    io::{Write, stdout},
    rc::Rc,
    sync::mpsc,
};

pub mod config;
pub mod formatter;
pub mod wrapper;
use config::*;
use formatter::*;
#[cfg(feature = "dbus")]
use wrapper::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Name of the config to use
    #[arg(short, long)]
    pub config_name: String,
    /// Generate the config.schema.json file
    #[arg(short, long, default_value_t = false)]
    pub gen_schema: bool,
}

#[doc(hidden)]
fn main() -> Result<()> {
    color_eyre::install()?;
    let matches = command!()
        .arg(
            arg!(
                -c --config <NAME> "Use config with a specific name"
            )
            .required(false)
            .value_parser(value_parser!(String)),
        )
        .subcommand(
            Command::new("gen_schema")
                .about("Generates json schema file associated with config.json"),
        )
        .subcommand(Command::new("path").about("Prints the config.json path"))
        .get_matches();

    if let Some(_matches) = matches.subcommand_matches("gen_schema") {
        ConfigFile::gen_schema()?;
        return Ok(());
    }

    if let Some(_matches) = matches.subcommand_matches("path") {
        let path = ConfigFile::config_file_path()?;
        println!("{}", path.to_str().unwrap());
        return Ok(());
    }

    let selected_config = matches.get_one::<String>("config");
    let path = ConfigFile::config_file_path()?;

    let mut configs: Vec<Rc<Config>> = Vec::new();

    let mut refresh_configs = || {
        println!("{:?}", "Reloading config");

        configs = ConfigFile::read_all()?
            .configs
            .into_iter()
            .map(|c| Rc::new(c))
            .collect();

        let config = match selected_config {
            Some(name) => configs
                .iter()
                .find(|c| c.name.as_deref() == Some(&name))
                .ok_or(eyre!(
                    "No config with name {name} found at {}",
                    path.to_string_lossy()
                )),
            None => configs
                .get(0)
                .ok_or(eyre!("No config found at {}", path.to_string_lossy())),
        }?
        .clone();

        let update_interval = config.update_interval_secs;
        let client = Client::new(update_interval)?;
        Ok::<_, color_eyre::eyre::Report>((config, update_interval, client))
    };

    let (mut config, mut update_interval, mut client) = refresh_configs()?;

    let mut stdout_lock = stdout().lock();

    let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(&path, notify::RecursiveMode::NonRecursive)?;

    loop {
        // TODO: Catch errors and restart rather than panic
        let devices = match client.devices() {
            Ok(v) => Some(v),
            Err(e) => {
                let Error::DBusError(de) = &e else {
                    return Err(e.into());
                };

                match de.kind {
                    // This means connection to kdeconnect failed
                    // In this case we should proceed as if no device was found
                    DBusErrorKind::UnknownObject => None,
                    _ => return Err(e.into()),
                }
            }
        };

        let device = match &config.device_id {
            Some(id) => devices
                .as_ref()
                .and_then(|d| d.iter().find(|d| d.id == *id)),
            None => devices.as_ref().and_then(|d| d.get(0)),
        };

        let output = OutputFormat::format_output(device, &config)?;

        writeln!(&mut stdout_lock, "{}", serde_json::to_string(&output)?)?;

        match rx.recv_timeout(update_interval) {
            Ok(res) => match res?.kind {
                EventKind::Modify(_) => (config, update_interval, client) = refresh_configs()?,
                EventKind::Create(_) => (config, update_interval, client) = refresh_configs()?,
                _ => (),
            },
            Err(e) => match e {
                mpsc::RecvTimeoutError::Timeout => (),
                _ => Err(e)?,
            },
        }
    }
}

#[doc(hidden)]
#[derive(Default, Serialize)]
struct OutputFormat<'a> {
    text: Cow<'a, str>,
    tooltip: Option<Cow<'a, str>>,
}

impl<'a> OutputFormat<'a> {
    fn format_output(device_opt: Option<&Device>, config: &'a Config) -> Result<Self> {
        let Some(device) = device_opt else {
            return Ok(Self::device_not_found(config));
        };
        let cache = DeviceCategoryDataCache::new(device);
        let info = cache.get_device_info()?;

        if !info.is_reachable {
            return Ok(Self::device_not_found(config));
        }

        let text = config.format.to_string(config, &cache)?;
        let tooltip = match &config.tooltip_format {
            Some(f) => Some(f.to_string(config, &cache)?),
            None => None,
        };

        Ok(OutputFormat {
            text: Cow::Owned(text),
            tooltip: tooltip.map(|s| Cow::Owned(s)),
        })
    }

    fn device_not_found(config: &'a Config) -> Self {
        OutputFormat {
            text: Cow::Borrowed(&config.device_not_found_text),
            tooltip: Some(Cow::Borrowed(&config.device_not_found_tooltip_text)),
        }
    }
}
