use crate::config::ConfigFile;
use clap::{Command, Parser, arg, command, value_parser};
use color_eyre::eyre::{Result, eyre};
use kdeconnect_wrapper::{
    client::Client,
    device::Device,
    error::{DBusErrorKind, Error},
};
use serde::Serialize;
use std::{
    borrow::Cow,
    fs,
    io::{Write, stdout},
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
        .arg(arg!([name] "Optional name to operate on"))
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
        .get_matches();

    if let Some(_matches) = matches.subcommand_matches("gen_schema") {
        let path = ConfigFile::dir()?.join("config.schema.json");
        fs::write(&path, serde_json::to_string_pretty(&ConfigFile::schema())?)?;
        println!("generated json schema at {}", path.to_str().unwrap());
        return Ok(());
    }

    let selected_config = matches.get_one::<String>("config");

    let configs = ConfigFile::read_all()?.configs;
    let path = ConfigFile::config_file_path()?;
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
    }?;

    let update_interval = config.update_interval_secs;
    let client = Client::new(update_interval)?;
    let mut stdout_lock = stdout().lock();

    loop {
        let devices = match client.devices() {
            Ok(v) => Some(v),
            Err(e) => {
                let Error::DBusError(de) = &e else {
                    return Err(e.into());
                };

                match de.kind {
                    // This means connection to kdeconnect failed
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

        let output = OutputFormat::format_output(device, config)?;

        writeln!(&mut stdout_lock, "{}", serde_json::to_string(&output)?)?;

        std::thread::sleep(update_interval);
    }
}

#[derive(Default, Serialize)]
struct OutputFormat<'a> {
    text: Cow<'a, str>,
    tooltip: Option<Cow<'a, str>>,
}

impl<'a> OutputFormat<'a> {
    fn format_output(device_opt: Option<&Device>, config: &'a config::Config) -> Result<Self> {
        let Some(device) = device_opt else {
            return Ok(OutputFormat {
                text: Cow::Borrowed(&config.device_not_found_text),
                tooltip: Some(Cow::Borrowed(&config.device_not_found_tooltip_text)),
            });
        };

        let text = config.format.to_string(device, config)?;
        let tooltip = match &config.tooltip_format {
            Some(f) => Some(f.to_string(device, config)?),
            None => None,
        };

        Ok(OutputFormat {
            text: Cow::Owned(text),
            tooltip: tooltip.map(|s| Cow::Owned(s)),
        })
    }
}
