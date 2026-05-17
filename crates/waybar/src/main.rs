use color_eyre::eyre::Result;
use serde::Serialize;
use std::{
    env::args,
    io::{Write, stdout},
    time::Duration,
};
use wrapper::Client;

fn main() -> Result<()> {
    color_eyre::install()?;
    let args: Vec<String> = args().collect();
    let update_interval = Duration::from_secs(5);

    let client = Client::new(update_interval)?;

    let mut stdout_lock = stdout().lock();

    loop {
        let devices = client.devices()?;

        let output = OutputFormat {
            text: match devices.get(0) {
                Some(device) => &device.get_battery_status()?.charge.to_string(),
                None => "Device not found",
            },
            tooltip: "b",
        };

        writeln!(&mut stdout_lock, "{}", serde_json::to_string(&output)?)?;

        std::thread::sleep(update_interval);
    }
}

#[derive(Default, Serialize)]
struct OutputFormat<'t, 'u> {
    text: &'t str,
    tooltip: &'u str,
}
