//! struct Device
//!
use kobject_uevent::{ActionType, UEvent};
use nix::errno::Errno;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

/// Device
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Device {
    /// inotify handler
    pub watch_handle: i32,

    // pub parent: Option<Rc<Device>>,
    /// device type
    pub devtype: String,
    /// device name
    pub devname: String,
    // pub devnum: u32,
    /// minor number
    pub minor: u32,
    /// major number
    pub major: u32,

    /// syspath with /sys/ as prefix
    pub syspath: String,
    /// devpath with /dev/ as prefix
    pub devpath: String,
    /// sysnum
    pub sysnum: String,
    /// sysname
    pub sysname: String,

    /// device subsystem
    pub subsystem: String,
    /// only set for the 'drivers' subsystem
    pub char_subsytem: String,
    /// device driver
    pub driver: String,

    /// device id
    pub device_id: String,

    /// device initialized usec
    pub usec_initialized: u64,

    /// device mode
    pub devmode: u16,
    /// device user id
    pub devuid: u32,
    /// device group id
    pub devgid: u32,

    // only set when device is passed through netlink
    /// uevent action
    pub action: Option<ActionType>,
    /// uevent seqnum
    pub seqnum: Option<u64>,

    // pub synth_uuid: u64,
    // pub partn: u32,

    // pub parent: Weak<Device>,
    /// device properties
    pub properties: HashMap<String, String>,
    /// the subset of properties that should be written to db
    pub properties_db: HashMap<String, String>,
    /// the string of properties
    pub properties_nulstr: Vec<u8>,
    /// the length of properties nulstr
    pub properties_nulstr_len: usize,

    /// cached sysattr values
    pub sysattr_values: HashMap<String, String>,
    /// names of sysattrs
    pub sysattrs: HashSet<String>,

    /// all tags
    pub all_tags: HashSet<String>,
    /// current tags
    pub current_tags: HashSet<String>,

    /// device links
    pub devlinks: HashSet<String>,
}

impl Default for Device {
    fn default() -> Self {
        Self::new()
    }
}

impl Device {
    /// create Device instance
    pub fn new() -> Device {
        Device {
            watch_handle: -1,
            devtype: String::new(),
            devname: String::new(),
            // devnum: 0,
            minor: 0,
            major: 0,
            syspath: String::new(),
            devpath: String::new(),
            sysnum: String::new(),
            sysname: String::new(),
            subsystem: String::new(),
            char_subsytem: String::new(),
            driver: String::new(),
            device_id: String::new(),
            usec_initialized: 0,
            devmode: std::u16::MAX - 1,
            devuid: std::u32::MAX - 1,
            devgid: std::u32::MAX - 1,
            action: None,
            seqnum: None,
            properties: HashMap::new(),
            properties_db: HashMap::new(),
            properties_nulstr: vec![],
            properties_nulstr_len: 0,
            sysattr_values: HashMap::new(),
            sysattrs: HashSet::new(),
            all_tags: HashSet::new(),
            current_tags: HashSet::new(),
            devlinks: HashSet::new(),
            // parent: None,
        }
    }

    /// create Device instance from UEvent
    pub fn from_uevent(uevent: UEvent) -> Device {
        let mut device = Device::new();
        for (key, value) in uevent.env.iter() {
            match key.as_str() {
                "DEVPATH" => {
                    device.devpath = value.clone();
                }
                "ACTION" => {
                    device.action = Some(ActionType::from_str(value).unwrap());
                }
                "SUBSYSTEM" => {
                    device.subsystem = value.clone();
                }
                "DEVTYPE" => {
                    device.devtype = value.clone();
                }
                "MINOR" => {
                    device.minor = value.parse().unwrap();
                }
                "MAJOR" => {
                    device.major = value.parse().unwrap();
                }
                "PARTN" => {}
                "SYNTH_UUID" => {}
                "DEVNAME" => {
                    device.devname = value.clone();
                }
                "SEQNUM" => {
                    device.seqnum = Some(value.parse().unwrap());
                }

                _ => {}
            }

            device.properties.insert(key.clone(), value.clone());
        }

        device
    }

    /// create Device from buffer
    pub fn from_buffer(buffer: &[u8]) -> Device {
        let mut device = Device::new();
        let s = std::str::from_utf8(buffer).unwrap();
        let mut length = 0;
        for line in s.split('\0') {
            let tokens = line.split('=').collect::<Vec<&str>>();
            if tokens.len() < 2 {
                break;
            }
            length = length + line.len() + 1;
            let (key, value) = (tokens[0], tokens[1]);
            match key {
                "DEVPATH" => {
                    device.devpath = value.to_string();
                }
                "ACTION" => {
                    device.action = Some(ActionType::from_str(value).unwrap());
                }
                "SUBSYSTEM" => {
                    device.subsystem = value.to_string();
                }
                "DEVTYPE" => {
                    device.devtype = value.to_string();
                }
                "MINOR" => {
                    device.minor = value.parse().unwrap();
                }
                "MAJOR" => {
                    device.major = value.parse().unwrap();
                }
                "PARTN" => {}
                "SYNTH_UUID" => {}
                "DEVNAME" => {
                    device.devname = value.to_string();
                }
                "SEQNUM" => {
                    device.seqnum = Some(value.parse().unwrap());
                }

                _ => {}
            }

            device.properties.insert(key.to_string(), value.to_string());
        }
        device.properties_nulstr = buffer[0..length].to_vec();
        device.properties_nulstr.push(0);
        device.properties_nulstr_len = length;
        device
    }

    /// get the seqnum of Device
    pub fn get_seqnum(&self) -> Option<u64> {
        self.seqnum
    }

    /// set the syspath of Device
    pub fn set_syspath(&mut self, path: String, verify: bool) -> Result<(), Errno> {
        if !path.starts_with("/sys/") {
            return Err(Errno::EINVAL);
        }

        if verify {
            todo!();
        }

        let devpath = &path[4..];

        if !devpath.starts_with('/') {
            return Err(Errno::ENODEV);
        }

        self.devpath = String::from(devpath);

        Ok(())
    }
}
