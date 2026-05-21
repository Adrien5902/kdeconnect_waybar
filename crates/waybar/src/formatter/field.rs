use super::PATH_SEPARATOR;
use crate::{config::Config, formatter::*};
use color_eyre::eyre::{Context, Report, Result, eyre};
use kdeconnect_wrapper::{
    device::{BatteryStatus, Device, DeviceInfoData, DeviceType},
    notifications::NotificationData,
};
use std::{borrow::Cow, str::FromStr, sync::OnceLock};
use strum::EnumString;

#[derive(Debug, Clone, Copy)]
/// The different categories that can be matched in any [`GlobalFormat`]
pub enum FieldCategory {
    DeviceInfo(DeviceInfo),
    Battery(Battery),
    Notification(Notification),
}

#[derive(Debug, Clone, Copy, EnumString)]
/// Used to display the different informations related to the device
pub enum DeviceInfo {
    /// Will be replaced by [`Config::device_phone_text`] or [`Config::device_tablet_text`] depending on if the device is a phone or a tablet
    DeviceTypeText,
}

#[derive(Debug, Clone, Copy, EnumString)]
/// Use to display information about the device's notifications
pub enum Notification {
    Grouped,
    Single,
}

#[derive(Debug, Clone, Copy, EnumString)]
/// Use to display information about the device's battery
pub enum Battery {
    /// Will be replaced with how much battery the device has left
    /// (this is measured in percentage, however the percent sign '%' isn't included you may wanna add it after in your [`GlobalFormat`])
    ChargePercent,
    /// Will be replaced to [`Config::is_charging_text`] or [`Config::isnt_charging_text`] from [`Config`] depending on wherever the device is charging or not
    IsChargingText,
    /// Will be replaced to [`Config::is_charging_texts`] or [`Config::isnt_charging_texts`] from [`Config`]
    /// depends on wherever the device is charging or not and the current charge see [`Config::charge_ranges`] in [`Config`] for more information
    ChargeTexts,
}

impl FromStr for FieldCategory {
    type Err = Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split: Vec<_> = s.split(PATH_SEPARATOR).collect();

        // TODO : Add better error message
        let category = *split
            .get(0)
            .ok_or(eyre!("expected a category, Syntax: Category:Field"))
            .with_context(|| s.to_owned())?;
        let field = *split
            .get(1)
            .ok_or(eyre!("expected a field, Syntax: Category:Field"))
            .with_context(|| s.to_owned())?;

        match category {
            "Battery" => Ok(Self::Battery(field.parse()?)),
            "DeviceInfo" => Ok(Self::DeviceInfo(field.parse()?)),
            "Notification" => Ok(Self::Notification(field.parse()?)),
            // TODO : Add error message
            _ => Err(eyre!("unknown category: {}", category)),
        }
    }
}

pub fn failed_to_parse_field_kind(s: &str) -> Report {
    // TODO : Add error message
    eyre!("{}", s)
}

#[derive(Debug)]
pub struct DeviceCategoryDataCache<'a> {
    device: &'a Device<'a>,
    device_info: OnceLock<DeviceInfoData>,
    battery: OnceLock<BatteryStatus>,
    notification: OnceLock<Vec<NotificationData>>,
}

impl<'a> DeviceCategoryDataCache<'a> {
    pub fn new(device: &'a Device<'a>) -> Self {
        Self {
            device,
            device_info: OnceLock::new(),
            battery: OnceLock::new(),
            notification: OnceLock::new(),
        }
    }

    pub fn get_device_info(&self) -> Result<&DeviceInfoData> {
        Ok(self
            .device_info
            .get_or_try_init(|| self.device.get_device_info())?)
    }

    pub fn get_battery(&self) -> Result<&BatteryStatus> {
        Ok(self
            .battery
            .get_or_try_init(|| self.device.get_battery_status())?)
    }

    pub fn get_notifications(&self) -> Result<&Vec<NotificationData>> {
        Ok(self.notification.get_or_try_init(|| {
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
        })?)
    }
}

impl FieldCategory {
    pub fn get_from_device<'a>(
        &self,
        config: &'a Config,
        cache: &DeviceCategoryDataCache,
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
                            config.is_charging_texts.clone()
                        } else {
                            config.isnt_charging_texts.clone()
                        };

                        let text = texts
                            .get(index)
                            .ok_or(eyre!("No format specified for this battery range"))
                            .with_context(|| config.to_string())
                            .with_context(|| format!("{:?}", texts))?;

                        Cow::Owned(text.clone())
                    }
                }
            }
            FieldCategory::DeviceInfo(f) => match f {
                DeviceInfo::DeviceTypeText => {
                    let status = cache.get_device_info()?;
                    match status.type_ {
                        DeviceType::Phone => Cow::Borrowed(&config.device_phone_text),
                        DeviceType::Tablet => Cow::Borrowed(&config.device_tablet_text),
                    }
                }
            },
            FieldCategory::Notification(f) => {
                let notifications = cache.get_notifications()?;
                let s = f.to_string(notifications, config)?;

                Cow::Owned(s)
            }
        };

        Ok(s)
    }
}

impl FieldFormat for FieldCategory {
    fn parse(s: &str) -> Result<Self> {
        Ok(s.parse()?)
    }
}
