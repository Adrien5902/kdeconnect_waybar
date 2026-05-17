use crate::parsing::FromDbusMap;
use color_eyre::eyre::Result;
use dbus::{
    arg::{AppendAll, PropMap, ReadAll},
    blocking::{Connection, Proxy as DBusProxy, stdintf::org_freedesktop_dbus::Properties},
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

#[macro_use]
mod parsing;

type Proxy<'a> = DBusProxy<'a, &'a Connection>;

pub struct Client {
    conn: Connection,
    timeout: Duration,
}

impl Client {
    const PATH_ROOT: &'static str = "/modules/kdeconnect";
    const INTERFACE_ROOT: &'static str = "org.kde.kdeconnect";

    pub fn new(timeout: Duration) -> Result<Self> {
        let conn = Connection::new_session()?;

        Ok(Self { conn, timeout })
    }

    pub fn devices_ids(&self) -> Result<Vec<DeviceId>> {
        let proxy =
            self.conn
                .with_proxy(Self::INTERFACE_ROOT, Self::PATH_ROOT, self.timeout.clone());

        let (devices,): (Vec<DeviceId>,) =
            proxy.method_call(Self::INTERFACE_ROOT.to_string() + ".daemon", "devices", ())?;
        Ok(devices)
    }

    pub fn devices<'a>(&'a self) -> Result<Vec<Device<'a>>> {
        Ok(self
            .devices_ids()?
            .iter()
            .map(|id| Device::new(self, id))
            .collect())
    }
}

pub type DeviceId = String;

pub struct Device<'a> {
    id: DeviceId,
    client: &'a Client,
}

impl<'a> Device<'a> {
    fn interface() -> String {
        Client::INTERFACE_ROOT.to_string() + ".device"
    }

    fn new(client: &'a Client, id: &DeviceId) -> Self {
        Self {
            id: id.clone(),
            client,
        }
    }

    pub fn get_battery_status(&self) -> Result<BatteryStatus> {
        let path = self.path().join("battery");
        let interface = Self::interface().to_string() + ".battery";
        self.get_all(&path, &interface)
    }

    pub fn get_device_info(&self) -> Result<DeviceInfo> {
        let path = self.path();
        let interface = Self::interface();
        self.get_all(&path, &interface)
    }

    fn notifications_interface(&self) -> String {
        Self::interface() + ".notifications"
    }

    fn notifications_path(&self) -> PathBuf {
        self.path().join("notifications")
    }

    fn get_notification_ids(&self) -> Result<Vec<NotificationId>> {
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
pub struct Notification<'a> {
    pub id: NotificationId,
    pub device: &'a Device<'a>,
}

impl<'a> Notification<'a> {
    fn new(id: NotificationId, device: &'a Device<'a>) -> Self {
        Notification { id, device }
    }

    pub fn get_data(&self) -> Result<NotificationData> {
        Ok(self.get_all(&self.path(), &self.device.notifications_interface())?)
    }
}

pub trait ClientObject<'c: 'p, 'p> {
    fn client(&'c self) -> &'c Client;
    fn path(&self) -> PathBuf;
    fn make_proxy(&'c self, path: &'p Path) -> Proxy<'p> {
        let client = self.client();
        client.conn.with_proxy(
            Client::INTERFACE_ROOT,
            into_dbus_path(&path),
            client.timeout.clone(),
        )
    }

    fn get_all<T>(&'c self, path: &'p Path, interface: &str) -> Result<T>
    where
        T: FromDbusMap,
    {
        let proxy = self.make_proxy(&path);
        let props: PropMap = proxy.get_all(interface)?;
        let res = T::from_props(props)?;
        Ok(res)
    }

    fn call_method<T, A>(
        &'c self,
        path: &'p Path,
        interface: &str,
        method_name: &str,
        args: A,
    ) -> Result<T>
    where
        T: ReadAll,
        A: AppendAll,
    {
        let proxy = self.make_proxy(path);
        let read: T = proxy.method_call(interface, method_name, args)?;
        Ok(read)
    }
}

impl<'c: 'p, 'p> ClientObject<'c, 'p> for Client {
    fn path(&self) -> PathBuf {
        PathBuf::from(Client::PATH_ROOT)
    }

    fn client(&'c self) -> &'c Client {
        self
    }
}

impl<'c: 'p, 'p> ClientObject<'c, 'p> for Device<'c> {
    fn path(&self) -> PathBuf {
        self.client.path().join("devices").join(&self.id)
    }

    fn client(&self) -> &'c Client {
        self.client
    }
}

impl<'c: 'p, 'p> ClientObject<'c, 'p> for Notification<'c> {
    fn path(&self) -> PathBuf {
        self.device.notifications_path().join(&self.id)
    }

    fn client(&self) -> &'c Client {
        self.device.client
    }
}

dbus_struct! {
    #[derive(Debug)]
    pub struct NotificationData{
        has_icon: bool,
        internal_id: String,
        is_conversation: bool,
        app_name: String,
        group_name: String,
        is_group_conversation: bool,
        reply_id: String,
        icon_path: String,
        silent: bool,
        text: String,
        dismissable: bool,
        ticker: String,
        title: String
    }
}

pub type NotificationId = String;

dbus_struct! {
    #[derive(Debug)]
    pub struct BatteryStatus {
        is_charging: bool,
        charge: i64,
    }
}

dbus_struct! {
    #[derive(Debug)]
    pub struct DeviceInfo {
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
    }
}

pub fn into_dbus_path(path: &Path) -> dbus::Path<'_> {
    path.to_str().unwrap().into()
}

#[test]
fn test() {
    let client = Client::new(Duration::from_secs(5)).unwrap();
    let devices = client.devices().unwrap();
    assert!(!devices.is_empty());
    for id in devices {
        let device = Device::from(id);
        assert!(device.get_battery_status().unwrap().charge > 0);
        assert!(device.get_device_info().unwrap().is_reachable);
        let notifications = device.get_notifications().unwrap();
        assert!(!notifications.is_empty());
        for notification in notifications {
            let data = notification.get_data().unwrap();
            assert!(!data.text.is_empty())
        }
    }
}
