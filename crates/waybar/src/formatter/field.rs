use super::PATH_SEPARATOR;
use crate::{config::Config, formatter::FieldFormat};
use color_eyre::eyre::{Context, Report, Result, eyre};
use kdeconnect_wrapper::{
    device::{BatteryStatus, Device, DeviceInfo, DeviceType},
    notifications::NotificationData,
};
use std::{borrow::Cow, str::FromStr, sync::OnceLock};
use strum::EnumString;

#[derive(Debug, Clone, Copy)]
pub enum FieldCategory {
    DeviceInfo(DeviceInfoField),
    Battery(BatteryField),
    Notification(NotificationField),
}

#[derive(Debug, Clone, Copy, EnumString)]
pub enum DeviceInfoField {
    DeviceTypeText,
}

#[derive(Debug, Clone, Copy, EnumString)]
pub enum NotificationField {
    Grouped,
    Single,
}

#[derive(Debug, Clone, Copy, EnumString)]
pub enum BatteryField {
    ChargePercent,
    IsChargingText,
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

#[derive(Debug, Default)]
pub struct DeviceCategoryDataCache {
    device_info: OnceLock<DeviceInfo>,
    battery: OnceLock<BatteryStatus>,
    notification: OnceLock<Vec<NotificationData>>,
}

impl DeviceCategoryDataCache {
    pub fn get_device_info(&self, device: &Device) -> Result<&DeviceInfo> {
        Ok(self
            .device_info
            .get_or_try_init(|| device.get_device_info())?)
    }

    pub fn get_battery(&self, device: &Device) -> Result<&BatteryStatus> {
        Ok(self
            .battery
            .get_or_try_init(|| device.get_battery_status())?)
    }

    pub fn get_notifications(&self, device: &Device) -> Result<&Vec<NotificationData>> {
        Ok(self.notification.get_or_try_init(|| {
            let mut notifications: Vec<NotificationData> = device
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
        device: &Device,
        config: &'a Config,
        cache: &DeviceCategoryDataCache,
    ) -> Result<Cow<'a, str>> {
        let s: Cow<'a, str> = match *self {
            FieldCategory::Battery(f) => {
                let status = cache.get_battery(device)?;

                match f {
                    BatteryField::ChargePercent => Cow::Owned(status.charge.to_string()),
                    BatteryField::IsChargingText => {
                        if status.is_charging {
                            Cow::Borrowed(&config.is_charging_text)
                        } else {
                            Cow::Borrowed(&config.isnt_charging_text)
                        }
                    }
                    BatteryField::ChargeTexts => {
                        let mut index: Option<usize> = None;
                        for (i, until_charge) in config.charge_ranges.iter().enumerate() {
                            if status.charge < *until_charge {
                                index = Some(i);
                                break;
                            }
                        }

                        let texts = if status.is_charging {
                            config.is_charging_texts.clone()
                        } else {
                            config.isnt_charging_texts.clone()
                        };

                        let text = texts
                            .get(
                                index
                                    .ok_or(eyre!("no charge_ranges defined in config"))
                                    .with_context(|| config.to_string())?,
                            )
                            .ok_or(eyre!("No format specified for this battery range"))
                            .with_context(|| config.to_string())
                            .with_context(|| format!("{:?}", texts))?;

                        Cow::Owned(text.clone())
                    }
                }
            }
            FieldCategory::DeviceInfo(f) => match f {
                DeviceInfoField::DeviceTypeText => {
                    let status = cache.get_device_info(device)?;
                    match status.type_ {
                        DeviceType::Phone => Cow::Borrowed(&config.device_phone_text),
                        DeviceType::Tablet => Cow::Borrowed(&config.device_tablet_text),
                    }
                }
            },
            FieldCategory::Notification(f) => {
                let notifications = cache.get_notifications(device)?;
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
