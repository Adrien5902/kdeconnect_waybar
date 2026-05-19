use std::time::Duration;

pub fn default_update_interval() -> Duration {
    Duration::from_secs(5)
}

pub fn default_device_not_found_text() -> String {
    "".into()
}

pub fn default_device_not_found_tooltip_text() -> String {
    "Device not found make sure kdeconnect is running and phone is connected".into()
}

pub fn default_is_charging_text() -> String {
    "󰂄 Charging ...".into()
}

pub fn default_isnt_charging_text() -> String {
    "󱟩 Not charging".into()
}

// Battery
pub fn default_charge_ranges() -> Vec<i64> {
    vec![10, 20, 30, 40, 50, 60, 70, 80, 90]
}

pub fn default_is_charging_texts() -> Vec<String> {
    vec![
        "󰢜".into(),
        "󰂆".into(),
        "󰂇".into(),
        "󰂈".into(),
        "󰢝".into(),
        "󰂉".into(),
        "󰢞".into(),
        "󰂊".into(),
        "󰂋".into(),
        "󰂅".into(),
    ]
}

pub fn default_isnt_charging_texts() -> Vec<String> {
    vec![
        "󰁺".into(),
        "󰁻".into(),
        "󰁼".into(),
        "󰁽".into(),
        "󰁾".into(),
        "󰁿".into(),
        "󰂀".into(),
        "󰂁".into(),
        "󰂂".into(),
        "󰁹".into(),
    ]
}

// Device type
pub fn default_device_phone_text() -> String {
    "Phone ".into()
}

pub fn default_device_tablet_text() -> String {
    "Tablet ".into()
}
