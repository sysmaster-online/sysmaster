// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! keyboard builtin
//!

use crate::builtin::Builtin;
use crate::error::{DeviceSnafu, Log, Result};
use crate::rules::exec_unit::ExecuteUnit;
use device::Device;
use input_event_codes_rs::{get_input_event_key, input_event_codes};
use ioctls::{eviocgabs, eviocgbit, eviocskeycode, input_absinfo};
use nix::fcntl::OFlag;
use snafu::ResultExt;
use std::mem;
use std::os::unix::prelude::AsRawFd;
use std::rc::Rc;

/// keyboard builtin command
pub struct Keyboard;
struct Map {
    scan: u32,
    key: u32,
}

impl Keyboard {
    fn map_keycode(fd: i32, scancode: u32, keycode: String) {
        let mut map = Map { scan: 0, key: 0 };

        let mut keycode_lookup = String::new();
        if keycode.starts_with("btn_") {
            keycode_lookup = keycode.to_uppercase();
        } else {
            if keycode.starts_with("key_") {
                keycode_lookup = "key_".to_owned() + &keycode;
            }
            keycode_lookup = keycode_lookup.to_uppercase();
        }

        map.scan = scancode;
        map.key = get_input_event_key::get_input_event_keycode(&keycode_lookup);

        log::debug!(
            "keyboard: mapping scan code {} 0x{:x} to key code {} 0x{:x}",
            map.scan,
            map.scan,
            map.key,
            map.key
        );

        unsafe {
            eviocskeycode(fd.as_raw_fd(), &[map.scan, map.key] as *const [u32; 2]);
        };
    }

    fn force_release(device: Rc<Device>, release: [u32; 1024], release_count: usize) {
        let atkbd = device
            .get_parent_with_subsystem_devtype("serio", None)
            .unwrap();
        let current = atkbd.get_sysattr_value("force_release").unwrap();
        let mut codes = current;
        for i in release.iter().take(release_count) {
            if !codes.is_empty() {
                codes += ",";
            }
            codes += &release[*i as usize].to_string();
        }
        let _ = atkbd.set_sysattr_value("force_release", Some(&codes));
    }

    unsafe fn eviocsabs(
        fd: ::std::os::raw::c_int,
        abs: u32,
        buf: *mut input_absinfo,
    ) -> ::std::os::raw::c_int {
        ioctl_sys::ioctl(
            fd,
            ioctl_sys::iow!(b'E', 0xc0 + abs, ::std::mem::size_of::<input_absinfo>())
                as ::std::os::raw::c_ulong,
            buf,
        )
    }

    fn override_abs(fd: i32, evcode: u32, value: &str) {
        // EVDEV_ABS_<axis>=<min>:<max>:<res>:<fuzz>:<flat>
        let mut absinfo = input_absinfo {
            value: 0,
            minimum: 0,
            maximum: 0,
            fuzz: 0,
            flat: 0,
            resolution: 0,
        };
        let mut absinfo_v = [0; 5];
        unsafe {
            eviocgabs(fd, evcode, &mut absinfo as *mut input_absinfo);
        }

        for (i, str) in value.split(':').enumerate() {
            absinfo_v[i] = str.parse().unwrap();
        }

        absinfo.minimum = absinfo_v[0];
        absinfo.maximum = absinfo_v[1];
        absinfo.resolution = absinfo_v[2];
        absinfo.fuzz = absinfo_v[3];
        absinfo.flat = absinfo_v[4];

        unsafe {
            Keyboard::eviocsabs(fd, evcode, &mut absinfo as *mut input_absinfo);
        }
    }

    fn set_trackpoint_sensitivity(device: Rc<Device>, value: &str) {
        let pdev = device
            .get_parent_with_subsystem_devtype("serio", None)
            .unwrap();
        if value.parse::<i32>().unwrap() < 0 || value.parse::<i32>().unwrap() > 255 {
            return;
        }
        let _ = pdev.set_sysattr_value("sensitivity", Some(value));
    }
}

impl Builtin for Keyboard {
    /// builtin command
    fn cmd(
        &self,
        exec_unit: &ExecuteUnit,
        _argc: i32,
        _argv: Vec<String>,
        _test: bool,
    ) -> Result<bool> {
        let mut fd = -1;
        let mut release = [0; 1024];
        let mut release_count = 0;
        let mut has_abs = -1;
        let device = exec_unit.get_device();
        let devname = device
            .get_devname()
            .context(DeviceSnafu)
            .log_dev_error(&device, "Failed to get devname!")?;

        for (key, value) in &device.property_iter() {
            // KEYBOARD_KEY_<hex scan code>=<key code identifier>
            if value.starts_with("KEYBOARD_KEY_") {
                let mut keycode: String = value.to_string();
                let scancode: u32 = u32::from_str_radix(&key[13..], 16).unwrap();

                if keycode.starts_with('!') {
                    keycode = keycode[1..].to_string();
                    release[release_count] = scancode;
                    if release_count < 1023 {
                        release_count += 1;
                    }

                    if keycode.starts_with('\0') {
                        continue;
                    }
                }

                if fd < 0 {
                    let file = Device::from_devname(&devname)
                        .unwrap()
                        .open(
                            OFlag::O_RDWR | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK | OFlag::O_NOCTTY,
                        )
                        .context(DeviceSnafu)
                        .log_error("Failed to open device!")?;
                    fd = file.as_raw_fd();
                }
                Keyboard::map_keycode(fd, scancode, keycode);
            } else if value.starts_with("EVDEV_ABS_") {
                // EVDEV_ABS_<axis>=<min>:<max>:<res>:<fuzz>:<flat>
                let evcode: u32 = u32::from_str_radix(&key[10..], 16).unwrap();
                if fd < 0 {
                    let file = Device::from_devname(&devname)
                        .unwrap()
                        .open(
                            OFlag::O_RDWR | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK | OFlag::O_NOCTTY,
                        )
                        .context(DeviceSnafu)
                        .log_error("Failed to open device!")?;
                    fd = file.as_raw_fd();
                }

                if has_abs == -1 {
                    let bits: u64 = 0;
                    unsafe {
                        eviocgbit(
                            fd.as_raw_fd(),
                            evcode,
                            mem::size_of_val(&bits).try_into().unwrap(),
                            bits as *mut u8,
                        );
                    };
                    has_abs = ((bits & 1 << input_event_codes::EV_ABS) != 0) as i32;
                }

                if has_abs != 0 {
                    continue;
                }
                Keyboard::override_abs(fd, evcode, value);
            } else if key == "POINTINGSTICK_SENSITIVITY" {
                Keyboard::set_trackpoint_sensitivity(device.clone(), value);
            }

            if release_count > 0 {
                Keyboard::force_release(device.clone(), release, release_count);
            }
        }
        Ok(true)
    }

    /// builtin init function
    fn init(&self) {}

    /// builtin exit function
    fn exit(&self) {}

    /// check whether builtin command should reload
    fn should_reload(&self) -> bool {
        false
    }

    /// the help of builtin command
    fn help(&self) -> String {
        "Keyboard scancode mapping and touchpad/pointingstick characteristics".to_string()
    }

    /// whether the builtin command can only run once
    fn run_once(&self) -> bool {
        false
    }
}
