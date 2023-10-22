use device::Device;

type Result<T> = std::result::Result<T, nix::Error>;

/// find device by path or unit name
pub fn find_device(id: &str, prefix: &str) -> Result<Device> {
    if id.is_empty() {
        return Err(nix::Error::EINVAL);
    }
    if let Ok(device) = Device::from_path(id) {
        return Ok(device);
    }

    if !prefix.is_empty() && !id.starts_with(prefix) {
        let path = prefix.to_string() + id;

        if let Ok(device) = Device::from_path(&path) {
            return Ok(device);
        }
    }

    /* Check if the argument looks like a device unit name. */
    find_device_from_unit(id)
}

/// dbus and device unit is not currently implemented
fn find_device_from_unit(_unit_name: &str) -> Result<Device> {
    todo!()
}
