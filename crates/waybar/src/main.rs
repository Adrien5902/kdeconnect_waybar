use crate::config::{CONFIG_FILE, Config};
use color_eyre::eyre::{Result, eyre};
use kdeconnect_wrapper::{
    client::Client,
    device::Device,
    error::{DBusErrorKind, Error},
};
use serde::Serialize;
use std::{
    borrow::Cow,
    env::args,
    io::{Write, stdout},
};

pub mod config;
pub mod formatter;

fn main() -> Result<()> {
    color_eyre::install()?;
    let args: Vec<String> = args().collect();

    let selected_config = args.get(1);

    let configs = Config::read_all()?;
    let config = match selected_config {
        Some(name) => configs
            .iter()
            .find(|c| c.name.as_deref() == Some(name))
            .ok_or(eyre!("No config with name {name} found at {CONFIG_FILE}")),
        None => configs
            .get(0)
            .ok_or(eyre!("No config found at {CONFIG_FILE}")),
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
