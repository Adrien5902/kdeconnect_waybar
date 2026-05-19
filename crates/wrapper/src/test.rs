use crate::{client::Client, device::Device};

#[test]
pub(crate) fn test() {
    use std::time::Duration;
    let client = Client::new(Duration::from_secs(5)).unwrap();
    let devices = client.devices().unwrap();
    assert!(!devices.is_empty());
    for id in devices {
        let device = Device::from(id);
        assert!(device.get_battery_status().unwrap().charge > 0);
        assert!(device.get_device_info().unwrap().is_reachable);
        let notifications = device.get_notifications().unwrap();
        assert!(!notifications.is_empty());
        for notification in notifications {
            let data = notification.get_data().unwrap();
            assert!(!data.text.is_empty())
        }
    }
}
