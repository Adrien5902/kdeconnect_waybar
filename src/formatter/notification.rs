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
    pub fn to_string<'a>(
        &self,
        notifications: &'a [NotificationData],
        config: &'a Config,
    ) -> Result<String> {
        let mut res: String = String::new();
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
                            Chunk::Str(s) => res.push_str(s),
                            Chunk::Field(field) => {
                                let cow: Cow<'a, str> =
                                    field.grouped_to_str(app_name, notifications, config)?;

                                res.push_str(&cow);
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
                            Chunk::Str(s) => res.push_str(s),
                            Chunk::Field(field) => {
                                res.push_str(&field.single_to_str(notification, config)?)
                            }
                        }
                    }
                }
            }
        }

        Ok(res)
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

fn sanitizate_html_tags(s: &str) -> String {
    // New sanitizated is at least longer string size
    let mut sanitizated = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '<' => sanitizated.push_str("&lt;"),
            '>' => sanitizated.push_str("&gt;"),
            '&' => sanitizated.push_str("&amp;"),
            '\"' => sanitizated.push_str("&quot;"),
            '\'' => sanitizated.push_str("&apos;"),
            other => sanitizated.push(other),
        }
    }
    sanitizated
}

impl NotificationFormatField {
    pub fn grouped_to_str<'a>(
        &self,
        app_name: &'a str,
        notifications: &[&'a NotificationData],
        config: &'a Config,
    ) -> Result<Cow<'a, str>> {
        let s = match *self {
            NotificationFormatField::AppName => Cow::Borrowed(app_name),
            NotificationFormatField::CustomIcon => {
                Cow::Borrowed(Notification::get_custom_icon(app_name, config))
            }
            NotificationFormatField::Count => Cow::Owned(notifications.len().to_string()),
            NotificationFormatField::CountText => config
                .notifications_count_text
                .get(&(notifications.len() as i64))
                .or(config.notifications_count_text.get(&0))
                .map(|s| Cow::<'_, str>::Borrowed(s))
                .unwrap_or(Cow::Owned(notifications.len().to_string())),
            NotificationFormatField::Content => {
                Err(eyre!("Not available in grouped notification"))?
            }
            NotificationFormatField::Title => Err(eyre!("Not available in grouped notification"))?,
        };
        Ok(s)
    }

    pub fn single_to_str<'a>(
        &self,
        notification: &'a NotificationData,
        config: &'a Config,
    ) -> Result<Cow<'a, str>> {
        let s: Cow<'a, str> = match *self {
            NotificationFormatField::AppName => Cow::Borrowed(&notification.app_name),
            NotificationFormatField::CustomIcon => Cow::Borrowed(Notification::get_custom_icon(
                &notification.app_name,
                config,
            )),
            NotificationFormatField::Count => Err(eyre!("Not available in single notification"))?,
            NotificationFormatField::CountText => {
                Err(eyre!("Not available in single notification"))?
            }
            NotificationFormatField::Content => {
                // Notification text can contain unsanitized contetn
                // If notification is a group conversation sanitization is done by kdeconnect
                // If not were doing it ourselves
                if !notification.is_group_conversation {
                    Cow::Owned(sanitizate_html_tags(&notification.text))
                } else {
                    Cow::Borrowed(&notification.text)
                }
            }
            NotificationFormatField::Title => Cow::Owned(sanitizate_html_tags(&notification.title)),
        };
        Ok(s)
    }
}
