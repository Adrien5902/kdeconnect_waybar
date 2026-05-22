use crate::wrapper::{Result, client::ClientObject, device::Device, parsing::FromDBusMap};
use dbus::arg::PropMap;

pub struct Notification<'a> {
    pub id: NotificationId,
    pub device: &'a Device<'a>,
}

impl<'a> Notification<'a> {
    pub(crate) fn new(id: NotificationId, device: &'a Device<'a>) -> Self {
        Notification { id, device }
    }

    pub fn get_data(&self) -> Result<NotificationData> {
        Ok(self.get_all(&self.path(), &self.device.notifications_interface())?)
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
