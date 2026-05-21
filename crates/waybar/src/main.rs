#![feature(once_cell_try)]

use crate::{
    config::{Config, ConfigFile},
    formatter::DeviceCategoryDataCache,
};
use clap::{Command, Parser, arg, command, value_parser};
use color_eyre::eyre::{Result, eyre};
use kdeconnect_wrapper::{
    client::Client,
    device::Device,
    error::{DBusErrorKind, Error},
};
use notify::{Event, EventKind, Watcher};
use serde::Serialize;
use std::{
    borrow::Cow,
    fs,
    io::{Write, stdout},
    sync::{Arc, mpsc},
};

pub mod config;
pub mod formatter;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the config to use
    #[arg(short, long)]
    config_name: String,
    /// Generate the config.schema.json file
    #[arg(short, long, default_value_t = false)]
    gen_schema: bool,
}

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
        let path = ConfigFile::dir()?.join("config.schema.json");
        fs::write(&path, serde_json::to_string_pretty(&ConfigFile::schema())?)?;
        println!("generated json schema at {}", path.to_str().unwrap());
        return Ok(());
    }

    if let Some(_matches) = matches.subcommand_matches("path") {
        let path = ConfigFile::config_file_path()?;
        fs::create_dir_all(path.parent().unwrap())?;
        println!("{}", path.to_str().unwrap());
        return Ok(());
    }

    let selected_config = matches.get_one::<String>("config");
    let path = ConfigFile::config_file_path()?;

    let mut configs: Vec<Arc<Config>> = Vec::new();

    let mut refresh_configs = || {
        println!("{:?}", "Reloading config");

        configs = ConfigFile::read_all()?
            .configs
            .into_iter()
            .map(|c| Arc::new(c))
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
