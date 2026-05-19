use std::fmt::Display;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Kdeconnect wasn't found running on your system")]
    KdeConnectNotStarted,

    #[error("fail to parse DBus data: {0} ")]
    DBusParsingFail(String),

    #[error("dbus error {0}")]
    DBusError(DBusError),

    #[error("Unknown error happened")]
    Unknown(/* Box<dyn std::error::Error> */),
}

impl From<dbus::Error> for Error {
    fn from(value: dbus::Error) -> Self {
        Self::DBusError(value.into())
    }
}

impl From<dbus::Error> for DBusError {
    fn from(value: dbus::Error) -> Self {
        let message = value.message().map(|s| s.to_owned());
        let kind = match value.name() {
            Some(name) => match name {
                "org.freedesktop.DBus.Error.UnknownObject" => DBusErrorKind::UnknownObject,
                _ => DBusErrorKind::Unknown(Some(name.to_owned())),
            },
            None => DBusErrorKind::Unknown(None),
        };

        DBusError { kind, message }
    }
}

#[derive(Debug)]
pub struct DBusError {
    pub kind: DBusErrorKind,
    pub message: Option<String>,
}

impl Display for DBusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}

#[derive(Debug)]
pub enum DBusErrorKind {
    UnknownObject,
    Unknown(Option<String>),
}
