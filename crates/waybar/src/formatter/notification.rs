use crate::{
    config::Config,
    formatter::{Chunk, FieldFormat, field::NotificationField},
};
use color_eyre::eyre::{Result, eyre};
use kdeconnect_wrapper::notifications::NotificationData;
use std::collections::HashMap;
use strum::EnumString;

impl NotificationField {
    pub fn to_string(&self, notifications: &[NotificationData], config: &Config) -> Result<String> {
        Ok(match self {
            NotificationField::Grouped => {
                let format = &config.notification_grouped_format;
                let mut map: HashMap<&str, Vec<&NotificationData>> = HashMap::new();
                for notification in notifications {
                    map.entry(&notification.app_name)
                        .or_default()
                        .push(notification);
                }

                map.iter()
                    .map(|(app_name, ns)| {
                        format
                            .chunks
                            .iter()
                            .map(|chunk| {
                                Ok(match chunk {
                                    Chunk::Field(f) => match *f {
                                        NotificationFormatField::AppName => (*app_name).to_owned(),
                                        NotificationFormatField::CustomIcon => {
                                            Self::get_custom_icon(app_name, config)
                                        }
                                        NotificationFormatField::Count => ns.len().to_string(),
                                        NotificationFormatField::CountText => config
                                            .notifications_count_text
                                            .get(&(ns.len() as i64))
                                            .or(config.notifications_count_text.get(&0))
                                            .map(|s| s.clone())
                                            .unwrap_or(ns.len().to_string()),
                                        NotificationFormatField::Content => {
                                            Err(eyre!("Not available in grouped notification"))?
                                        }
                                        NotificationFormatField::Title => {
                                            Err(eyre!("Not available in grouped notification"))?
                                        }
                                    },
                                    Chunk::Str(s) => s.clone(),
                                })
                            })
                            .collect::<Result<String>>()
                    })
                    .collect::<Result<String>>()?
            }
            NotificationField::Single => {
                let format = &config.notification_single_format;
                notifications
                    .iter()
                    .map(|n| {
                        format
                            .chunks
                            .iter()
                            .map(|chunk| {
                                Ok(match chunk {
                                    Chunk::Field(f) => match *f {
                                        NotificationFormatField::AppName => n.app_name.clone(),
                                        NotificationFormatField::CustomIcon => {
                                            Self::get_custom_icon(&n.app_name, config)
                                        }
                                        NotificationFormatField::Count => {
                                            Err(eyre!("Not available in single notification"))?
                                        }
                                        NotificationFormatField::CountText => {
                                            Err(eyre!("Not available in single notification"))?
                                        }
                                        NotificationFormatField::Content => n.text.clone(),
                                        NotificationFormatField::Title => n.title.clone(),
                                    },
                                    Chunk::Str(s) => s.clone(),
                                })
                            })
                            .collect::<Result<String>>()
                    })
                    .collect::<Result<String>>()?
            }
        })
    }

    fn get_custom_icon(app_name: &str, config: &Config) -> String {
        config
            .app_icons
            .get(&app_name.to_string())
            .or(config.app_icons.get(&String::new()))
            .map(|s| s.clone())
            .unwrap_or("?".to_string())
    }
}

#[derive(Clone, Copy, Debug, EnumString)]
pub enum NotificationFormatField {
    /// The app name e.g. Instagram, Snapchat
    AppName,
    /// see config app_icons
    CustomIcon,

    /// Available for {Notification:Single} only
    Title,
    /// Available for {Notification:Single} only
    Content,

    /// Available for {Notification:Grouped} only
    Count,
    /// Available for {Notification:Grouped} only
    CountText,
}

impl FieldFormat for NotificationFormatField {
    fn parse(s: &str) -> Result<Self> {
        Ok(s.parse()?)
    }
}
