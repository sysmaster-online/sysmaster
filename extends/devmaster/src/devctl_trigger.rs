//! subcommand for devctl trigger
//!
use libdevice::{device::Device, device_action::DeviceAction};

/// subcommand for trigger a fake device action, then the kernel will report an uevent
pub fn subcommand_trigger(devices: Vec<String>, action: Option<String>) {
    if devices.is_empty() {
        todo!("Currently do not support triggering all devices")
    }

    let action = match action {
        Some(a) => a.parse::<DeviceAction>().unwrap(),
        None => DeviceAction::Change,
    };

    for d in devices {
        let mut device = Device::from_path(d).unwrap();
        device.trigger(action).unwrap();
    }
}
