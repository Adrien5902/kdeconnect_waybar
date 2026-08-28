use super::client::Client;
use crate::wrapper::{
    Result,
    client::ClientObject,
    notifications::{Notification, NotificationId},
    parsing::FromDBusMap,
};
use dbus::arg::PropMap;
use std::{
    cell::OnceCell,
    fmt::Debug,
    path::{Path, PathBuf},
};

pub type DeviceId = String;

pub struct Device<'a> {
    pub id: DeviceId,
    pub info: OnceCell<DeviceInfoData>,
    pub(crate) client: &'a Client,
}

impl<'a> Debug for Device<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{{id: {}}}", self.id))
    }
}

impl<'a> Device<'a> {
    pub(crate) fn interface() -> String {
        Client::INTERFACE_ROOT.to_string() + ".device"
    }

    pub(crate) fn new(client: &'a Client, id: DeviceId) -> Result<Self> {
        let device = Self {
            id,
            client,
            info: OnceCell::new(),
        };
        let info = Self::get_device_info(client, &device.path())?;
        device.info.set(info).unwrap();
        Ok(device)
    }

    pub fn get_battery_status(&self) -> Result<BatteryStatus> {
        let path = self.path().join("battery");
        let interface = Self::interface().to_string() + ".battery";
        self.get_all(&path, &interface)
    }

    fn get_device_info(client: &Client, path: &Path) -> Result<DeviceInfoData> {
        let interface = Self::interface();
        client.get_all(&path, &interface)
    }

    pub(crate) fn notifications_interface(&self) -> String {
        Self::interface() + ".notifications"
    }

    pub(crate) fn notifications_path(&self) -> PathBuf {
        self.path().join("notifications")
    }

    pub(crate) fn get_notification_ids(&self) -> Result<Vec<NotificationId>> {
        let (ids,): (Vec<NotificationId>,) = self.call_method(
            &self.notifications_path(),
            &self.notifications_interface(),
            "activeNotifications",
            (),
        )?;
        Ok(ids)
    }

    pub fn get_notifications(&'a self) -> Result<Vec<Notification<'a>>> {
        Ok(self
            .get_notification_ids()?
            .into_iter()
            .map(|id| Notification::new(id, self))
            .collect())
    }
}

dbus_struct! {
    #[derive(Debug)]
    pub struct BatteryStatus {
        is_charging: bool,
        charge: i64,
    }
}

dbus_struct! {
    #[derive(Debug)]
    pub struct DeviceInfoData {
        status_icon_name: String,
        is_paired: bool,
        is_reachable: bool,
        is_pair_requested_by_peer: bool,
        name: String,
        icon_name: String,
        active_provider_names: Vec<String>,
        is_pair_requested: bool,
        reachable_addresses: Vec<String>,
        pair_state: i64,
        supported_plugins: Vec<String>,
        type_: DeviceType, // Do not remove _, type is a rust keyword that can't be used
        verification_key: String,
    }
}

dbus_enum! {
    pub enum DeviceType {
        Phone,
        Tablet,
        Desktop,
        Laptop,
    }
}
