use super::PATH_SEPARATOR;
use crate::wrapper::{
    device::{BatteryStatus, Device, DeviceInfoData, DeviceType},
    notifications::NotificationData,
};
use crate::{config::Config, formatter::*};
use color_eyre::eyre::{Context, Report, Result, eyre};
use std::{borrow::Cow, cell::OnceCell, str::FromStr};
use strum::EnumString;

#[derive(Debug, Clone, Copy)]
/// The different categories that can be matched in any [`GlobalFormat`]
///
/// Refer to each category to see all its available fields
pub enum FieldCategory {
    DeviceInfo(DeviceInfo),
    Battery(Battery),
    Notification(Notification),
}

#[derive(Debug, Clone, Copy, EnumString)]
/// Used to display the different informations related to the device
pub enum DeviceInfo {
    /// The local ip address of the device
    Address,
    /// The device name, configurable in the device's KDE Connect app
    DeviceName,
    /// Will be replaced by [`Config::device_phone_text`] or [`Config::device_tablet_text`] depending on if the device is a phone or a tablet
    DeviceTypeText,
}

#[derive(Debug, Clone, Copy, EnumString)]
/// Use to display information about the device's notifications
pub enum Notification {
    /// Device's current notifications grouped by application name
    ///
    /// Will be replaced with [`Config::notification_grouped_format`]
    Grouped,
    /// Device's current individual notifications
    ///
    /// Will be replaced with [`Config::notification_single_format`]
    Single,
}

#[derive(Debug, Clone, Copy, EnumString)]
/// Use to display information about the device's battery
pub enum Battery {
    /// Will be replaced with how much battery the device has left
    ///
    /// (this is measured in percentage, however the percent sign `%` isn't included you may wanna add it after in your [`GlobalFormat`])
    ChargePercent,
    /// Will be replaced to [`Config::is_charging_text`] or [`Config::isnt_charging_text`] depending on wherever the device is charging or not
    IsChargingText,
    /// Will be replaced to [`Config::is_charging_texts`] or [`Config::isnt_charging_texts`]
    ///
    /// depends on wherever the device is charging or not and the current charge see [`Config::charge_ranges`] for more information
    ChargeTexts,
}

impl FromStr for FieldCategory {
    type Err = Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(PATH_SEPARATOR);

        let category = split
            .next()
            .ok_or_else(|| eyre!("expected a category, Syntax: Category::Field"))
            .with_context(|| s.to_owned())?;

        let field = split
            .next()
            .ok_or_else(|| eyre!("expected a field, Syntax: Category::Field"))
            .with_context(|| s.to_owned())
            .with_context(|| format!("After {category}"))?;

        match category {
            "Battery" => Ok(Self::Battery(field.parse()?)),
            "DeviceInfo" => Ok(Self::DeviceInfo(field.parse()?)),
            "Notification" => Ok(Self::Notification(field.parse()?)),
            // TODO : Add error message
            _ => Err(eyre!("unknown category: {}", category)),
        }
    }
}

#[doc(hidden)]
pub fn failed_to_parse_field_kind(s: &str) -> Report {
    // TODO : Add error message
    eyre!("{}", s)
}

#[doc(hidden)]
#[derive(Debug)]
/// Used not to fetch the device data twice
pub struct DeviceCategoryDataCache<'a> {
    device: &'a Device<'a>,
    battery: OnceCell<BatteryStatus>,
    notification: OnceCell<Vec<NotificationData>>,
}

// Helper function to remove nightly feature dependency once_cell_try
pub fn get_or_try_init<T, F, E>(cell: &OnceCell<T>, f: F) -> Result<&T, E>
where
    F: FnOnce() -> Result<T, E>,
{
    if let Some(value) = cell.get() {
        return Ok(value);
    }

    // This should always be ok since if the code continued nothing was set
    let _ = cell.set(f()?);
    // This shouldn't panic as we just set it's content
    Ok(cell.get().unwrap())
}

impl<'a> DeviceCategoryDataCache<'a> {
    pub fn new(device: &'a Device<'a>) -> Self {
        Self {
            device,
            battery: OnceCell::new(),
            notification: OnceCell::new(),
        }
    }

    pub fn get_device_info(&self) -> &DeviceInfoData {
        self.device.info.get().unwrap()
    }

    pub fn get_battery(&self) -> Result<&BatteryStatus> {
        Ok(get_or_try_init(&self.battery, || {
            self.device.get_battery_status()
        })?)
    }

    pub fn get_notifications(&self) -> Result<&Vec<NotificationData>> {
        get_or_try_init(&self.notification, || {
            let mut notifications: Vec<NotificationData> = self
                .device
                .get_notifications()?
                .into_iter()
                .map(|n| {
                    let d = n.get_data()?;
                    Ok(d)
                })
                .collect::<Result<_, Report>>()?;
            notifications.sort_by(|a, b| a.app_name.cmp(&b.app_name));

            Ok::<Vec<NotificationData>, Report>(notifications)
        })
    }
}

impl FieldCategory {
    pub fn get_from_device<'a>(
        &self,
        config: &'a Config,
        cache: &'a DeviceCategoryDataCache,
    ) -> Result<Cow<'a, str>> {
        let s: Cow<'a, str> = match *self {
            FieldCategory::Battery(f) => {
                let status = cache.get_battery()?;

                match f {
                    Battery::ChargePercent => Cow::Owned(status.charge.to_string()),
                    Battery::IsChargingText => {
                        if status.is_charging {
                            Cow::Borrowed(&config.is_charging_text)
                        } else {
                            Cow::Borrowed(&config.isnt_charging_text)
                        }
                    }
                    Battery::ChargeTexts => {
                        let mut index = 0;
                        for (i, until_charge) in config.charge_ranges.iter().enumerate() {
                            if status.charge <= *until_charge {
                                break;
                            }
                            index = i + 1;
                        }

                        let texts = if status.is_charging {
                            &config.is_charging_texts
                        } else {
                            &config.isnt_charging_texts
                        };

                        let text = texts
                            .get(index)
                            .ok_or_else(|| eyre!("No format specified for this battery range"))
                            .with_context(|| config.to_string())
                            .with_context(|| format!("{:?}", texts))?;

                        Cow::Borrowed(text)
                    }
                }
            }
            FieldCategory::DeviceInfo(f) => {
                let info = cache.get_device_info();
                match f {
                    DeviceInfo::Address => Cow::Borrowed(
                        info
                            .reachable_addresses
                            .first()
                            .ok_or_else(|| eyre!("Ip address not found for device"))?,
                    ),
                    DeviceInfo::DeviceName => Cow::Borrowed(&info.name),
                    DeviceInfo::DeviceTypeText => match info.type_ {
                        DeviceType::Phone => Cow::Borrowed(&config.device_phone_text),
                        DeviceType::Tablet => Cow::Borrowed(&config.device_tablet_text),
                        DeviceType::Desktop => Cow::Borrowed(&config.device_desktop_text),
                        DeviceType::Laptop => Cow::Borrowed(&config.device_laptop_text),
                    },
                }
            }
            FieldCategory::Notification(f) => {
                let notifications = cache.get_notifications()?;
                Cow::Owned(f.to_string(notifications, config)?)
            }
        };

        Ok(s)
    }
}

impl FieldFormat for FieldCategory {
    fn parse(s: &str) -> Result<Self> {
        s.parse()
    }
}
