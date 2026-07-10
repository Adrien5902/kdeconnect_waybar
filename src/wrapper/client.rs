use crate::wrapper::{
    Result,
    device::{Device, DeviceId},
    notifications::Notification,
    parsing::FromDBusMap,
};
use dbus::{
    arg::{AppendAll, PropMap, ReadAll},
    blocking::{Connection, Proxy as DBusProxy, stdintf::org_freedesktop_dbus::Properties},
};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub type Proxy<'a> = DBusProxy<'a, &'a Connection>;

pub struct Client {
    pub conn: Connection,
    pub timeout: Duration,
}

impl Client {
    pub const PATH_ROOT: &'static str = "/modules/kdeconnect";
    pub const INTERFACE_ROOT: &'static str = "org.kde.kdeconnect";

    pub fn new(timeout: Duration) -> Result<Self> {
        let conn = Connection::new_session()?;

        Ok(Self { conn, timeout })
    }

    pub fn devices_ids(&self) -> Result<Vec<DeviceId>> {
        let proxy = self
            .conn
            .with_proxy(Self::INTERFACE_ROOT, Self::PATH_ROOT, self.timeout);

        let (devices,): (Vec<DeviceId>,) =
            proxy.method_call(Self::INTERFACE_ROOT.to_string() + ".daemon", "devices", ())?;
        Ok(devices)
    }

    pub fn devices<'a>(&'a self) -> Result<Vec<Device<'a>>> {
        Ok(self
            .devices_ids()?
            .into_iter()
            .map(|id| Device::new(self, id))
            .collect())
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
            client.timeout,
        )
    }

    fn get_all<T>(&'c self, path: &'p Path, interface: &str) -> Result<T>
    where
        T: FromDBusMap,
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

pub fn into_dbus_path(path: &Path) -> dbus::Path<'_> {
    path.to_str().unwrap().into()
}
