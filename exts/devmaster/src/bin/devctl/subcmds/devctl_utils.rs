use device::Device;
use nix::unistd::{access, AccessFlags};
use std::path::PathBuf;

type Result<T> = std::result::Result<T, nix::Error>;

/// find device by path or unit name
pub fn find_device(id: &str, prefix: &str) -> Result<Device> {
    if id.is_empty() {
        return Err(nix::Error::EINVAL);
    }
    if let Ok(device) = Device::from_path(id) {
        return Ok(device);
    }

    let mut path = PathBuf::from(id);

    if !prefix.is_empty() && !id.starts_with(prefix) {
        path = PathBuf::from(prefix.to_string() + "/" + id)
            .canonicalize()
            .unwrap();
        if let Ok(device) = Device::from_path(path.to_str().unwrap()) {
            return Ok(device);
        }
    }

    /* if a path is provided, then it cannot be a unit name. Let's return earlier. */
    if path.to_str().unwrap().contains('/') {
        return Err(nix::Error::ENODEV);
    }

    /* Check if the argument looks like a device unit name. */
    find_device_from_unit(id)
}

/// dbus and device unit is not currently implemented
fn find_device_from_unit(_unit_name: &str) -> Result<Device> {
    todo!()
}

/// check if the queue is empty
pub fn devmaster_queue_is_empty() -> Result<bool> {
    match access("/run/devmaster/queue", AccessFlags::F_OK) {
        Ok(()) => Ok(false),
        Err(err) => {
            if err == nix::Error::ENOENT {
                return Ok(true);
            }
            Err(err)
        }
    }
}
