use crate::wrapper::notifications::NotificationData;
use crate::{config::Config, formatter::*};
use color_eyre::eyre::{Result, eyre};
use std::collections::BTreeMap;
use strum::EnumString;

/// A kind of [`Format`]
/// used in [`Config::notification_grouped_format`] and [`Config::notification_single_format`] e.g. `"-{AppName}\n"`
///
/// Can be either grouped or single, see [`Notification`]
///
/// see [`NotificationFormatField`] for all the different fields available
pub type NotificationFormat = Format<NotificationFormatField>;

impl Notification {
    pub fn format<'a>(
        &self,
        f: &mut String,
        notifications: &'a [NotificationData],
        config: &'a Config,
    ) -> Result<()> {
        match *self {
            Notification::Grouped => {
                let format = config.notification_grouped_format
                    .as_ref()
                    .ok_or_else(|| eyre!("Use of Notification::Grouped but no notification_grouped_format were defubed in config"))?;

                // We use BTree map instead of HashMap because we don't want notification order to change
                // So notifications are organized in app_name alphabetical order
                let mut map: BTreeMap<&'a str, Vec<&'a NotificationData>> = BTreeMap::new();
                for notification in notifications {
                    map.entry(&notification.app_name)
                        .or_default()
                        .push(notification);
                }

                for (app_name, notifications) in &map {
                    for chunk in &format.chunks {
                        match chunk {
                            Chunk::Str(s) => f.push_str(s),
                            Chunk::Field(field) => {
                                field.format_grouped(f, app_name, notifications, config)?;
                            }
                        }
                    }
                }
            }

            Notification::Single => {
                let format = config.notification_single_format
                    .as_ref()
                    .ok_or_else(|| eyre!("Use of Notification::Single but no notification_single_format were defubed in config"))?;

                for notification in notifications {
                    for chunk in &format.chunks {
                        match chunk {
                            Chunk::Str(s) => f.push_str(s),
                            Chunk::Field(field) => {
                                field.format_single(f, notification, config)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    const DEFAULT_ICON: &'static str = "?";

    fn get_custom_icon<'a>(app_name: &str, config: &'a Config) -> &'a str {
        config
            .app_icons
            .get(app_name)
            .or(config.app_icons.get(&String::new()))
            .map(|a| a.as_str())
            .unwrap_or(Self::DEFAULT_ICON)
    }
}

#[derive(Clone, Copy, Debug, EnumString)]
/// All the fields than can be used in a [`NotificationFormat`], see [`Config::notification_grouped_format`] and [`Config::notification_single_format`]
///
/// ⚠️ Caution: Some fields are only available in grouped or single mode
pub enum NotificationFormatField {
    /// The app name e.g. `Instagram`, `Snapchat`
    AppName,
    /// A text field corresponding to the notification's app icon,
    /// see [`Config::app_icons`]
    ///
    /// ℹ️ Recommended with Nerd-Font icons,
    CustomIcon,

    /// ⚠️ Available for {[`Notification::Single`]} only
    ///
    /// The title of the notification, corresponds the the bigger text
    Title,
    /// ⚠️ Available for {[`Notification::Single`]} only
    ///
    /// The content of the notification, corresponds the the smaller text under the title
    Content,

    /// ⚠️ Available for {[`Notification::Grouped`]} only
    ///
    /// The amount of notifications of this app, displayed as a number
    Count,
    /// ⚠️ Available for {[`Notification::Grouped`]} only
    ///
    /// The amount of notifications of this app, with custom display strings like icons for example,
    ///
    /// see [`Config::notifications_count_text`] for more details
    CountText,
}

impl FieldFormat for NotificationFormatField {
    fn parse(s: &str) -> Result<Self> {
        Ok(s.parse()?)
    }
}

fn sanitizate_html_tags(f: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '<' => f.push_str("&lt;"),
            '>' => f.push_str("&gt;"),
            '&' => f.push_str("&amp;"),
            '\"' => f.push_str("&quot;"),
            '\'' => f.push_str("&apos;"),
            other => f.push(other),
        }
    }
}

impl NotificationFormatField {
    pub fn format_grouped<'a>(
        &self,
        f: &mut String,
        app_name: &'a str,
        notifications: &[&'a NotificationData],
        config: &'a Config,
    ) -> Result<()> {
        match *self {
            NotificationFormatField::AppName => f.write_str(app_name)?,
            NotificationFormatField::CustomIcon => {
                f.write_str(Notification::get_custom_icon(app_name, config))?
            }
            NotificationFormatField::Count => f.write_str(&notifications.len().to_string())?,
            NotificationFormatField::CountText => match &config
                .notifications_count_text
                .get(&(notifications.len() as i64))
                .or_else(|| config.notifications_count_text.get(&0))
            {
                Some(s) => f.write_str(s)?,
                None => f.write_str(&notifications.len().to_string())?,
            },
            NotificationFormatField::Content => {
                Err(eyre!("Not available in grouped notification"))?
            }
            NotificationFormatField::Title => Err(eyre!("Not available in grouped notification"))?,
        };
        Ok(())
    }

    pub fn format_single<'a>(
        &self,
        f: &mut String,
        notification: &'a NotificationData,
        config: &'a Config,
    ) -> Result<()> {
        match *self {
            NotificationFormatField::AppName => f.write_str(&notification.app_name)?,
            NotificationFormatField::CustomIcon => f.write_str(Notification::get_custom_icon(
                &notification.app_name,
                config,
            ))?,
            NotificationFormatField::Count => Err(eyre!("Not available in single notification"))?,
            NotificationFormatField::CountText => {
                Err(eyre!("Not available in single notification"))?
            }
            NotificationFormatField::Content => {
                // Notification text can contain unsanitized contetn
                // If notification is a group conversation sanitization is done by kdeconnect
                // If not were doing it ourselves
                if !notification.is_group_conversation {
                    sanitizate_html_tags(f, &notification.text);
                } else {
                    f.write_str(&notification.text.replace("<br/>", "\n"))?;
                }
            }
            NotificationFormatField::Title => sanitizate_html_tags(f, &notification.title),
        };
        Ok(())
    }
}
