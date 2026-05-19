use super::PATH_SEPARATOR;
use crate::config::Config;
use color_eyre::eyre::{Context, Report, Result, eyre};
use kdeconnect_wrapper::device::{Device, DeviceType};
use std::borrow::Cow;
use std::str::FromStr;
use strum::EnumString;

#[derive(Debug, Clone, Copy)]
pub enum FieldKind {
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
    Ids,
    Icons,
}

#[derive(Debug, Clone, Copy, EnumString)]
pub enum BatteryField {
    ChargePercent,
    IsChargingText,
    ChargeTexts,
}

impl FromStr for FieldKind {
    type Err = Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (category, field) = s
            .split_once(PATH_SEPARATOR)
            // TODO : Add error message
            .ok_or(eyre!("expected a category with "))?;

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

impl FieldKind {
    pub fn get_from_device<'a>(&self, device: &Device, config: &'a Config) -> Result<Cow<'a, str>> {
        Ok(match *self {
            FieldKind::Battery(f) => {
                let status = device.get_battery_status()?;
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
            FieldKind::DeviceInfo(f) => match f {
                DeviceInfoField::DeviceTypeText => {
                    let status = device.get_device_info()?;
                    match status.type_ {
                        DeviceType::Phone => Cow::Borrowed(&config.device_phone_text),
                        DeviceType::Tablet => Cow::Borrowed(&config.device_tablet_text),
                    }
                }
            },
            FieldKind::Notification(_f) => {
                todo!("Impl notifications")
                // let notifications: Vec<NotificationData> = device
                //     .get_notifications()?
                //     .into_iter()
                //     .map(|n| n.get_data())
                //     .collect::<Result<_>>()?;

                // match f {
                //     NotificationDataField::Icons => notifications
                //         .iter()
                //         .map(|n| n.icon_path)
                //         .collect::<Vec<String>>()
                //         .join(" "),
                //         NotificationDataField::Ids => {

                //         }
                // }
            }
        })
    }
}
