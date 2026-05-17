use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use color_eyre::eyre::{Error, Result};
use dbus::{
    arg::{PropMap, Variant},
    blocking::{Connection, stdintf::org_freedesktop_dbus::Properties},
};

#[macro_use]
mod parsing;

pub struct Client {
    conn: Arc<Connection>,
    timeout: Arc<Duration>,
}

impl Client {
    const PATH_ROOT: &'static str = "/modules/kdeconnect";
    const INTERFACE_ROOT: &'static str = "org.kde.kdeconnect";

    pub fn new(timeout: Duration) -> Result<Self> {
        let conn = Arc::new(Connection::new_session()?);

        Ok(Self {
            conn,
            timeout: Arc::new(timeout),
        })
    }

    pub fn devices_ids(&self) -> Result<Vec<DeviceId>> {
        let proxy = self.conn.with_proxy(
            Self::INTERFACE_ROOT,
            Self::PATH_ROOT,
            self.timeout.as_ref().clone(),
        );

        let (devices,): (Vec<DeviceId>,) =
            proxy.method_call(Self::INTERFACE_ROOT.to_string() + ".daemon", "devices", ())?;
        Ok(devices)
    }

    pub fn devices(&self) -> Result<Vec<Device>> {
        Ok(self
            .devices_ids()?
            .iter()
            .map(|id| Device::new(self.conn.clone(), self.timeout.clone(), id))
            .collect())
    }
}

pub type DeviceId = String;

pub struct Device {
    id: DeviceId,
    conn: Arc<Connection>,
    timeout: Arc<Duration>,
}

impl Device {
    fn interface() -> String {
        Client::INTERFACE_ROOT.to_string() + ".device"
    }

    fn new(conn: Arc<Connection>, timeout: Arc<Duration>, id: &DeviceId) -> Self {
        Self {
            id: id.clone(),
            conn,
            timeout,
        }
    }

    fn get_path(&self) -> PathBuf {
        PathBuf::from(Client::PATH_ROOT)
            .join("devices")
            .join(&self.id)
    }

    pub fn battery_status(&self) -> Result<BatteryStatus> {
        let path = self.get_path().join("battery");
        let interface = Self::interface().to_string() + ".battery";
        let proxy = self.conn.with_proxy(
            Client::INTERFACE_ROOT,
            into_dbus_path(&path),
            self.timeout.as_ref().clone(),
        );
        let res: PropMap = proxy.get_all(&interface)?;

        Ok((&res).try_into()?)
    }
}

dbus_struct! {
    #[derive(Debug)]
    pub struct BatteryStatus {
        is_charging: bool,
        charge: i64,
    }
}

pub fn into_dbus_path(path: &Path) -> dbus::Path {
    path.to_str().unwrap().into()
}

#[test]
fn test() {
    let devices = Client::new(Duration::from_secs(5))
        .unwrap()
        .devices()
        .unwrap();
    assert!(!devices.is_empty());
    for id in devices {
        let device = Device::from(id);
        assert!(device.battery_status().unwrap().charge > 0)
    }
}
