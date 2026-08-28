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

use clap::{ArgAction, Command, arg, command, value_parser};
use color_eyre::eyre::{Result, eyre};
use notify::{Event, EventKind, Watcher};
use serde::Serialize;
use std::{
    borrow::Cow,
    cell::OnceCell,
    io::{Write, stdout},
    path::Path,
    sync::mpsc,
};

pub mod config;
pub mod formatter;
pub mod wrapper;
use config::*;
use formatter::*;
#[cfg(feature = "dbus")]
use wrapper::*;

thread_local! {
    static IS_VERBOSE: OnceCell<bool> = OnceCell::new();
}

macro_rules! debug {
    ($($arg:tt)*) => {
        IS_VERBOSE.with(|cell| {
            if cell.get().copied().unwrap_or_default() {
                println!("[INFO] {}", format_args!($($arg)*))
            }
        })
    }
}

struct AppState {
    client: Client,
    config: Config,
}

impl AppState {
    fn load(config_file_path: &Path, selected_config_str: Option<&str>) -> Result<Self> {
        let configs: Vec<Config> = ConfigFile::read_all()?.configs.into_iter().collect();

        debug!("Reloading config");

        let selected_config = match selected_config_str {
            Some(name) => configs
                .into_iter()
                .find(|c| c.name.as_deref() == Some(&name))
                .ok_or(eyre!(
                    "No config with name {name} found at {}",
                    config_file_path.to_string_lossy()
                )),
            None => configs.into_iter().next().ok_or(eyre!(
                "No config found at {}",
                config_file_path.to_string_lossy()
            )),
        }?;

        let state = Self {
            client: Client::new(selected_config.update_interval)?,
            config: selected_config,
        };

        Ok(state)
    }

    fn fetch_device<'a>(
        &'a self,
        device_id_override: Option<&String>,
    ) -> Result<Option<Device<'a>>> {
        let devices_res = self.client.devices();
        if let Err(error) = &devices_res {
            // This means connection to kdeconnect failed
            // In this case we should proceed as if no device was found
            if let Error::DBusError(dbus_error) = &error
                && dbus_error.kind == DBusErrorKind::UnknownObject
            {
                return Ok(None);
            }
        };
        let devices = devices_res?;

        let device = match device_id_override.or(self.config.device_id.as_ref()) {
            Some(id) => devices.into_iter().find(|d| d.id == *id),
            None => devices
                .into_iter()
                .find(|device| device.info.get().unwrap().is_reachable),
        };
        Ok(device)
    }
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
        .arg(
            arg!(
                -d --device <ID> "Override config device id"
            )
            .required(false)
            .value_parser(value_parser!(DeviceId)),
        )
        .arg(
            arg!(-v --verbose "Prints debug messages to stdout")
                .required(false)
                .action(ArgAction::SetTrue),
        )
        .arg(
            arg!(-n --no_updates "Print data only once to the stdout, powerful with jq")
                .required(false)
                .action(ArgAction::SetTrue),
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

    IS_VERBOSE
        .with(|cell| {
            cell.set(
                matches
                    .get_one::<bool>("verbose")
                    .copied()
                    .unwrap_or_default(),
            )
        })
        .expect("State already set");

    let device_id = matches.get_one::<DeviceId>("device_id");

    let no_updates = matches
        .get_one::<bool>("no_updates")
        .copied()
        .unwrap_or_default();

    let config_file_path = ConfigFile::config_file_path()?;
    let selected_config_arg = matches.get_one::<String>("config");
    let selected_config_str = selected_config_arg.map(|s| s.as_str());
    let mut state = AppState::load(&config_file_path, selected_config_str)?;

    let mut stdout_lock = stdout().lock();

    let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();

    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(&ConfigFile::dir()?, notify::RecursiveMode::NonRecursive)?;

    'main: loop {
        let device = state.fetch_device(device_id)?;
        let output = OutputFormat::format_output(device.as_ref(), &state.config)?;

        writeln!(&mut stdout_lock, "{}", serde_json::to_string(&output)?)?;

        if no_updates {
            break 'main Ok(());
        }

        match rx.recv_timeout(state.config.update_interval) {
            Ok(res) => {
                let event = res?;
                if matches!(event.kind, EventKind::Modify(_)) {
                    state = AppState::load(&config_file_path, selected_config_str)?
                }
            }
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
        let info = cache.get_device_info();

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
