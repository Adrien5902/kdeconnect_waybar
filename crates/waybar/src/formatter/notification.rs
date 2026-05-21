use crate::{
    config::Config,
    formatter::{Chunk, FieldFormat, Format, field::NotificationField},
};
use color_eyre::eyre::{Result, eyre};
use kdeconnect_wrapper::notifications::NotificationData;
use std::collections::BTreeMap;
use strum::EnumString;

pub type NotificationFormat = Format<NotificationFormatField>;

impl NotificationField {
    pub fn to_string(&self, notifications: &[NotificationData], config: &Config) -> Result<String> {
        Ok(match self {
            NotificationField::Grouped => {
                let format = &config.notification_grouped_format;
                // We use BTree map instead of HashMap because we don't want notification order to change
                // So notifications are organized in app_name alphabetical order
                let mut map: BTreeMap<&str, Vec<&NotificationData>> = BTreeMap::new();
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
/// TODO : document
pub enum NotificationFormatField {
    /// The app name e.g. Instagram, Snapchat
    AppName,
    /// A text field corresponding to the notification icon
    /// recommended with Nerd-Font icons
    /// see config app_icons
    CustomIcon,

    /// Available for {Notification:Single} only
    /// The title of the notification, corresponds the the bigger text
    Title,
    /// Available for {Notification:Single} only
    /// The content of the notification, corresponds the the smaller text under the title
    Content,

    /// Available for {Notification:Grouped} only
    /// The amount of notifications of this app, displayed as a number
    Count,
    /// Available for {Notification:Grouped} only
    /// The amount of notifications of this app, with custom display string like icons for example
    /// see notifications_count_text in config for more details
    CountText,
}

impl FieldFormat for NotificationFormatField {
    fn parse(s: &str) -> Result<Self> {
        Ok(s.parse()?)
    }
}
