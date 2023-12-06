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

//! input_id builtin
//!

use crate::builtin::Builtin;
use crate::error::Log;
use crate::error::Result;
use crate::log_dev;
use crate::rules::exec_unit::ExecuteUnit;
use device::Device;
use input_event_codes;
use ioctls::{eviocgabs, input_absinfo};
use libc::input_id;
use nix::fcntl::OFlag;
use std::os::unix::prelude::AsRawFd;
use std::rc::Rc;

macro_rules! bits_per_long {
    () => {
        (std::mem::size_of::<u64>() * 8)
    };
}

macro_rules! nbits {
    ($x: expr) => {
        (((($x) as usize - 1) / bits_per_long!()) + 1)
    };
}

macro_rules! off {
    ($x: expr) => {
        (($x) as usize % bits_per_long!())
    };
}

macro_rules! long {
    ($x: expr) => {
        (($x) as usize / bits_per_long!())
    };
}

macro_rules! test_bit {
    ($bit: expr, $array: expr) => {
        ((($array[long!($bit)] >> off!($bit)) & 1) != 0)
    };

    ($bit: expr, $array: expr, $flag: expr) => {
        if $flag {
            ((($array[long!($bit)] >> off!($bit)) & 1) != 0)
        } else {
            ((($array[long!($bit)] >> off!($bit)) & 1) == 0)
        }
    };
}

macro_rules! flags_set {
    ($v: expr, $flags: expr) => {
        ((!($v) & ($flags)) == 0)
    };
}

/// bus type
#[allow(non_camel_case_types)]
pub enum BusType {
    /// I2C bus device
    BUS_I2C = 24,
}

struct Range {
    start: usize,
    end: usize,
}

struct Bitmasks {
    bitmask_ev: [u64; nbits!(libc::EV_MAX)],
    bitmask_abs: [u64; nbits!(libc::ABS_MAX)],
    bitmask_key: [u64; nbits!(libc::KEY_MAX)],
    bitmask_rel: [u64; nbits!(libc::REL_MAX)],
    bitmask_props: [u64; nbits!(libc::INPUT_PROP_MAX)],
}

/// input_id builtin command
pub struct InputId;

impl Builtin for InputId {
    /// builtin command
    fn cmd(
        &self,
        exec_unit: &ExecuteUnit,
        _argc: i32,
        _argv: Vec<String>,
        test: bool,
    ) -> Result<bool> {
        let device = exec_unit.get_device();

        let mut bitmasks = Bitmasks {
            bitmask_ev: [0; nbits!(libc::EV_MAX)],
            bitmask_abs: [0; nbits!(libc::ABS_MAX)],
            bitmask_key: [0; nbits!(libc::KEY_MAX)],
            bitmask_rel: [0; nbits!(libc::REL_MAX)],
            bitmask_props: [0; nbits!(libc::INPUT_PROP_MAX)],
        };

        let mut pdev = Option::Some(device.clone());
        loop {
            if pdev
                .as_ref()
                .unwrap()
                .get_sysattr_value("capabilities/ev")
                .is_ok()
            {
                break;
            }

            let tmp_dev = match pdev
                .unwrap()
                .get_parent_with_subsystem_devtype("input", None)
            {
                Ok(dev) => Option::Some(dev),
                Err(_) => None,
            };

            pdev = tmp_dev;

            if pdev.is_none() {
                break;
            }
        }

        if let Some(dev) = pdev {
            let id = self.get_input_id(dev.clone());

            self.add_property(device.clone(), test, "ID_INPUT", "1")
                .unwrap_or(());

            self.get_cap_mask(
                dev.clone(),
                "capabilities/ev",
                std::mem::size_of_val(&bitmasks.bitmask_ev),
                &mut bitmasks.bitmask_ev,
                test,
            );
            self.get_cap_mask(
                dev.clone(),
                "capabilities/abs",
                std::mem::size_of_val(&bitmasks.bitmask_abs),
                &mut bitmasks.bitmask_abs,
                test,
            );
            self.get_cap_mask(
                dev.clone(),
                "capabilities/rel",
                std::mem::size_of_val(&bitmasks.bitmask_rel),
                &mut bitmasks.bitmask_rel,
                test,
            );
            self.get_cap_mask(
                dev.clone(),
                "capabilities/key",
                std::mem::size_of_val(&bitmasks.bitmask_key),
                &mut bitmasks.bitmask_key,
                test,
            );
            self.get_cap_mask(
                dev,
                "properties",
                std::mem::size_of_val(&bitmasks.bitmask_props),
                &mut bitmasks.bitmask_props,
                test,
            );

            let is_pointer = self.test_pointer(device.clone(), &id, &mut bitmasks, test);
            let is_key = self.test_key(device.clone(), &mut bitmasks, test);

            if !is_pointer
                && !is_key
                && test_bit!(input_event_codes::EV_REL!(), bitmasks.bitmask_ev)
                && (test_bit!(input_event_codes::REL_WHEEL!(), bitmasks.bitmask_rel)
                    || test_bit!(input_event_codes::REL_HWHEEL!(), bitmasks.bitmask_rel))
            {
                self.add_property(device.clone(), test, "ID_INPUT_KEY", "1")
                    .unwrap_or(());
            }
            if test_bit!(input_event_codes::EV_SW!(), bitmasks.bitmask_ev) {
                self.add_property(device.clone(), test, "ID_INPUT_SWITCH", "1")
                    .unwrap_or(());
            }
        }

        let sysname = device.get_sysname().unwrap_or_default();

        if sysname.starts_with("event") {
            self.extract_info(device, test);
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
        "Input device properties".to_string()
    }

    /// whether the builtin command can only run once
    fn run_once(&self) -> bool {
        false
    }

    fn add_property(
        &self,
        device: Rc<Device>,
        test: bool,
        key: &str,
        value: &str,
    ) -> Result<(), crate::error::Error> {
        device
            .add_property(key, value)
            .map_err(|e| crate::error::Error::BuiltinCommandError {
                msg: format!("Failed to add property '{}'='{}': ({})", key, value, e),
            })?;

        if test {
            println!("{}={}", key, value);
        }

        Ok(())
    }
}

impl InputId {
    fn get_input_id(&self, dev: Rc<Device>) -> input_id {
        let mut id = input_id {
            bustype: u16::default(),
            vendor: u16::default(),
            product: u16::default(),
            version: u16::default(),
        };

        if let Ok(v) = dev.get_sysattr_value("id/bustype") {
            id.bustype = u16::from_str_radix(&v, 16).unwrap();
        }

        if let Ok(v) = dev.get_sysattr_value("id/vendor") {
            id.vendor = u16::from_str_radix(&v, 16).unwrap();
        }

        if let Ok(v) = dev.get_sysattr_value("id/product") {
            id.product = u16::from_str_radix(&v, 16).unwrap();
        }

        if let Ok(v) = dev.get_sysattr_value("id/version") {
            id.version = u16::from_str_radix(&v, 16).unwrap();
        }

        id
    }

    fn get_cap_mask(
        &self,
        dev: Rc<Device>,
        attr: &str,
        bitmask_size: usize,
        bitmask: &mut [u64],
        test: bool,
    ) {
        let text = dev.get_sysattr_value(attr).unwrap_or_default();

        log_dev!(
            debug,
            dev,
            format!("{} raw kernel attribute: {}", attr, text)
        );

        for (i, word) in text.as_str().split_whitespace().rev().enumerate() {
            let val = match u64::from_str_radix(word, 16) {
                Err(_) => {
                    log_dev!(
                        debug,
                        dev,
                        format!("Ignoring {} block which failed to parse", attr)
                    );
                    continue;
                }
                Ok(v) => v,
            };

            if i < bitmask_size / std::mem::size_of::<u64>() {
                bitmask[i] = val;
            } else {
                log_dev!(
                    debug,
                    dev,
                    format!(
                        "Ignoring {} block {:X} which is larger than maximum size",
                        attr, val
                    )
                );
            }
        }

        if test {
            log_dev!(debug, dev, format!("{} decoded bit map:", attr));
            let mut val = bitmask_size / std::mem::size_of::<u64>();

            while val > 0 && bitmask[val - 1] == 0 {
                val -= 1;
            }

            debug_assert!(std::mem::size_of::<u64>() == 4 || std::mem::size_of::<u64>() == 8);

            for (j, bit) in bitmask.iter().enumerate() {
                if j >= val {
                    break;
                }

                if std::mem::size_of::<u64>() == 4 {
                    log_dev!(
                        debug,
                        dev,
                        format!("  bit {:4}: {:08X}", j * bits_per_long!(), bit)
                    );
                } else {
                    log_dev!(
                        debug,
                        dev,
                        format!("  bit {:4}: {:016X}", j * bits_per_long!(), bit)
                    );
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn test_pointer(
        &self,
        dev: Rc<Device>,
        id: &input_id,
        bitmasks: &mut Bitmasks,
        test: bool,
    ) -> bool {
        let has_keys = test_bit!(input_event_codes::EV_KEY!(), bitmasks.bitmask_ev);
        let has_abs_coordinates = test_bit!(input_event_codes::ABS_X!(), bitmasks.bitmask_abs)
            && test_bit!(input_event_codes::ABS_Y!(), bitmasks.bitmask_abs);
        let has_3d_coordinates =
            has_abs_coordinates && test_bit!(input_event_codes::ABS_Z!(), bitmasks.bitmask_abs);
        let mut is_accelerometer = test_bit!(
            input_event_codes::INPUT_PROP_ACCELEROMETER!(),
            bitmasks.bitmask_props
        );

        if !has_keys && has_3d_coordinates {
            is_accelerometer = true;
        }

        if is_accelerometer {
            self.add_property(dev, test, "ID_INPUT_ACCELEROMETER", "1")
                .unwrap_or(());
            return true;
        }

        let mut is_pointing_stick = test_bit!(
            input_event_codes::INPUT_PROP_POINTING_STICK!(),
            bitmasks.bitmask_props
        );
        let has_stylus = test_bit!(input_event_codes::BTN_STYLUS!(), bitmasks.bitmask_key);
        let has_pen = test_bit!(input_event_codes::BTN_TOOL_PEN!(), bitmasks.bitmask_key);
        let finger_but_no_pen =
            test_bit!(input_event_codes::BTN_TOOL_FINGER!(), bitmasks.bitmask_key)
                && test_bit!(
                    input_event_codes::BTN_TOOL_PEN!(),
                    bitmasks.bitmask_key,
                    false
                );

        let mut has_mouse_button = false;
        for button in input_event_codes::BTN_MOUSE!()..input_event_codes::BTN_JOYSTICK!() {
            if !has_mouse_button {
                has_mouse_button = test_bit!(button, bitmasks.bitmask_key);
            }
        }

        let has_rel_coordinates = test_bit!(input_event_codes::EV_REL!(), bitmasks.bitmask_ev)
            && test_bit!(input_event_codes::REL_X!(), bitmasks.bitmask_rel)
            && test_bit!(input_event_codes::REL_Y!(), bitmasks.bitmask_rel);
        let mut has_mt_coordinates = test_bit!(
            input_event_codes::ABS_MT_POSITION_X!(),
            bitmasks.bitmask_abs
        ) && test_bit!(
            input_event_codes::ABS_MT_POSITION_Y!(),
            bitmasks.bitmask_abs
        );

        if has_mt_coordinates
            && test_bit!(input_event_codes::ABS_MT_SLOT!(), bitmasks.bitmask_abs)
            && test_bit!(input_event_codes::ABS_MT_SLOT!() - 1, bitmasks.bitmask_abs)
        {
            has_mt_coordinates = false;
        }

        let is_direct = test_bit!(
            input_event_codes::INPUT_PROP_DIRECT!(),
            bitmasks.bitmask_props
        );
        let has_touch = test_bit!(input_event_codes::BTN_TOUCH!(), bitmasks.bitmask_key);
        let has_pad_buttons = test_bit!(input_event_codes::BTN_0!(), bitmasks.bitmask_key)
            && test_bit!(input_event_codes::BTN_1!(), bitmasks.bitmask_key)
            && !has_pen;
        let has_wheel = test_bit!(input_event_codes::EV_REL!(), bitmasks.bitmask_ev)
            && (test_bit!(input_event_codes::REL_WHEEL!(), bitmasks.bitmask_rel)
                || test_bit!(input_event_codes::REL_HWHEEL!(), bitmasks.bitmask_rel));

        let mut num_joystick_buttons = 0;
        if test_bit!(
            input_event_codes::BTN_JOYSTICK!() - 1,
            bitmasks.bitmask_key,
            false
        ) {
            for button in input_event_codes::BTN_JOYSTICK!()..input_event_codes::BTN_DIGI!() {
                if test_bit!(button, bitmasks.bitmask_key) {
                    num_joystick_buttons += 1;
                }
            }
            for button in
                input_event_codes::BTN_TRIGGER_HAPPY1!()..input_event_codes::BTN_TRIGGER_HAPPY40!()
            {
                if test_bit!(button, bitmasks.bitmask_key) {
                    num_joystick_buttons += 1;
                }
            }
            for button in input_event_codes::BTN_DPAD_UP!()..input_event_codes::BTN_DPAD_RIGHT!() {
                if test_bit!(button, bitmasks.bitmask_key) {
                    num_joystick_buttons += 1;
                }
            }
        }
        let mut num_joystick_axes = 0;
        for axis in input_event_codes::ABS_RX!()..input_event_codes::ABS_PRESSURE!() {
            if test_bit!(axis, bitmasks.bitmask_abs) {
                num_joystick_axes += 1;
            }
        }

        let mut is_tablet = false;
        let mut is_touchpad = false;
        let mut is_abs_mouse = false;
        let mut is_touchscreen = false;
        let mut is_joystick = false;
        if has_abs_coordinates {
            if has_stylus || has_pen {
                is_tablet = true;
            } else if finger_but_no_pen && !is_direct {
                is_touchpad = true;
            } else if has_mouse_button {
                is_abs_mouse = true;
            } else if has_touch || is_direct {
                is_touchscreen = true;
            } else if num_joystick_buttons > 0 || num_joystick_axes > 0 {
                is_joystick = true;
            }
        } else if num_joystick_buttons > 0 || num_joystick_axes > 0 {
            is_joystick = true;
        }

        if has_mt_coordinates {
            if has_stylus || has_pen {
                is_tablet = true;
            } else if finger_but_no_pen && !is_direct {
                is_touchpad = true;
            } else if has_touch || is_direct {
                is_touchscreen = true;
            }
        }

        let mut is_tablet_pad = false;
        if is_tablet && has_pad_buttons {
            is_tablet_pad = true;
        }

        if has_pad_buttons && has_wheel && !has_rel_coordinates {
            is_tablet = true;
            is_tablet_pad = true;
        }

        let mut is_mouse = false;
        if !is_tablet
            && !is_touchpad
            && !is_joystick
            && has_mouse_button
            && (has_rel_coordinates || !has_abs_coordinates)
        {
            is_mouse = true;
        }

        if is_mouse && id.bustype == BusType::BUS_I2C as u16 {
            is_pointing_stick = true;
        }

        if is_joystick {
            let well_known_keyboard_keys = [
                input_event_codes::KEY_LEFTCTRL!(),
                input_event_codes::KEY_CAPSLOCK!(),
                input_event_codes::KEY_NUMLOCK!(),
                input_event_codes::KEY_INSERT!(),
                input_event_codes::KEY_MUTE!(),
                input_event_codes::KEY_CALC!(),
                input_event_codes::KEY_FILE!(),
                input_event_codes::KEY_MAIL!(),
                input_event_codes::KEY_PLAYPAUSE!(),
                input_event_codes::KEY_BRIGHTNESSDOWN!(),
            ];

            let mut num_well_known_keys = 0;

            if has_keys {
                for key in well_known_keyboard_keys {
                    if test_bit!(key, bitmasks.bitmask_key) {
                        num_well_known_keys += 1;
                    }
                }
            }

            if num_well_known_keys >= 4 || num_joystick_buttons + num_joystick_axes < 2 {
                log_dev!(debug, dev,
                    format!("Input device has {} joystick buttons and {} axes but also {} keyboard key sets, \
                             assuming this is a keyboard, not a joystick.",
                            num_joystick_buttons, num_joystick_axes, num_well_known_keys));
                is_joystick = false;
            }

            if has_wheel && has_pad_buttons {
                log_dev!(
                    debug,
                    dev,
                    format!(
                        "Input device has {} joystick buttons as well as tablet pad buttons, \
                         assuming this is a tablet pad, not a joystick.",
                        num_joystick_buttons
                    )
                );
                is_joystick = false;
            }
        }

        if is_pointing_stick {
            let _ = self
                .add_property(dev.clone(), test, "ID_INPUT_POINTINGSTICK", "1")
                .log_error("input_id error");
        }
        if is_mouse || is_abs_mouse {
            let _ = self
                .add_property(dev.clone(), test, "ID_INPUT_MOUSE", "1")
                .log_error("input_id error");
        }
        if is_touchpad {
            let _ = self
                .add_property(dev.clone(), test, "ID_INPUT_TOUCHPAD", "1")
                .log_error("input_id error");
        }
        if is_touchscreen {
            let _ = self
                .add_property(dev.clone(), test, "ID_INPUT_TOUCHSCREEN", "1")
                .log_error("input_id error");
        }
        if is_joystick {
            let _ = self
                .add_property(dev.clone(), test, "ID_INPUT_JOYSTICK", "1")
                .log_error("input_id error");
        }
        if is_tablet {
            let _ = self
                .add_property(dev.clone(), test, "ID_INPUT_TABLET", "1")
                .log_error("input_id error");
        }
        if is_tablet_pad {
            let _ = self
                .add_property(dev, test, "ID_INPUT_TABLET_PAD", "1")
                .log_error("input_id error");
        }

        is_tablet
            || is_mouse
            || is_abs_mouse
            || is_touchpad
            || is_touchscreen
            || is_joystick
            || is_pointing_stick
    }

    fn test_key(&self, dev: Rc<Device>, bitmasks: &mut Bitmasks, test: bool) -> bool {
        if test_bit!(input_event_codes::EV_KEY!(), bitmasks.bitmask_ev, false) {
            log_dev!(debug, dev, "test_key: no EV_KEY capability".to_string());
            return false;
        }

        let mut found = false;
        for (i, bit) in bitmasks.bitmask_key.iter().enumerate() {
            if i < input_event_codes::BTN_MISC!() / bits_per_long!() && !found {
                if *bit != 0 {
                    found = true;
                }
                log_dev!(
                    debug,
                    dev,
                    format!(
                        "test_key: checking bit block {} for any keys; found={}",
                        i * bits_per_long!(),
                        if found { "yes" } else { "no" }
                    )
                );
            } else {
                break;
            }
        }

        let high_key_blocks = [
            Range {
                start: input_event_codes::KEY_OK!(),
                end: input_event_codes::BTN_DPAD_UP!(),
            },
            Range {
                start: input_event_codes::KEY_ALS_TOGGLE!(),
                end: input_event_codes::BTN_TRIGGER_HAPPY!(),
            },
        ];

        for block in high_key_blocks.iter() {
            if found {
                break;
            }
            for i in block.start..block.end {
                if test_bit!(i, bitmasks.bitmask_key) && !found {
                    log_dev!(
                        debug,
                        dev,
                        format!("test_key: Found key {} in high block", i)
                    );
                    found = true;
                }
            }
        }

        if found {
            let _ = self
                .add_property(dev.clone(), test, "ID_INPUT_KEY", "1")
                .log_error("input_id error");
        }

        if flags_set!(bitmasks.bitmask_key[0], 0xFFFFFFFE) {
            let _ = self
                .add_property(dev, test, "ID_INPUT_KEYBOARD", "1")
                .log_error("input_id error");
            return true;
        }

        found
    }

    fn abs_size_mm(absinfo: &input_absinfo) -> i32 {
        (absinfo.maximum - absinfo.minimum) / absinfo.resolution
    }

    fn extract_info(&self, dev: Rc<Device>, test: bool) {
        let mut xabsinfo = input_absinfo {
            ..Default::default()
        };
        let mut yabsinfo = input_absinfo {
            ..Default::default()
        };

        let fd = match dev
            .open(OFlag::O_RDONLY | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK | OFlag::O_NOCTTY)
        {
            Ok(fd) => fd,
            Err(_) => return,
        };

        unsafe {
            if eviocgabs(fd.as_raw_fd(), input_event_codes::ABS_X!(), &mut xabsinfo) < 0
                || eviocgabs(fd.as_raw_fd(), input_event_codes::ABS_Y!(), &mut yabsinfo) < 0
            {
                return;
            }
        }

        if xabsinfo.resolution <= 0 || yabsinfo.resolution <= 0 {
            return;
        }

        let _ = self
            .add_property(
                dev.clone(),
                test,
                "ID_INPUT_WIDTH_MM",
                InputId::abs_size_mm(&xabsinfo).to_string().as_str(),
            )
            .log_error("input_id error");
        let _ = self
            .add_property(
                dev,
                test,
                "ID_INPUT_HEIGHT_MM",
                InputId::abs_size_mm(&yabsinfo).to_string().as_str(),
            )
            .log_error("input_id error");
    }
}

#[cfg(test)]
#[cfg(debug_assertions)]
mod tests {
    use super::InputId;
    use crate::{builtin::Builtin, rules::exec_unit::ExecuteUnit};
    use device::device_enumerator::DeviceEnumerator;

    #[test]
    fn test_builtin_input_id() {
        log::init_log_to_console_syslog("test_builtin_input_id", log::Level::Debug);

        let mut enumerator = DeviceEnumerator::new();

        for device in enumerator.iter().filter(|d| d.get_devpath().is_ok()) {
            let exec_unit = ExecuteUnit::new(device);
            let builtin = InputId {};
            let _ = builtin.cmd(&exec_unit, 0, vec![], true);
        }
    }
}
