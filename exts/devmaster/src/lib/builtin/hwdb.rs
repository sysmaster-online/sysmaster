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

//! hwdb builtin
//!

use crate::builtin::Builtin;
use crate::error::{Error, Result};
use crate::rules::exec_unit::ExecuteUnit;
use clap::Parser;
use device::Device;
use glob::Pattern;
use hwdb::sd_hwdb::SdHwdb;
use std::cell::RefCell;
use std::rc::Rc;

/// parse program arguments
#[derive(Parser, Debug)]
struct Args {
    ///
    #[clap(short, long, value_parser)]
    filter: Option<String>,
    ///
    #[clap(short, long, value_parser)]
    device: Option<String>,
    ///
    #[clap(short, long, value_parser)]
    subsystem: Option<String>,
    ///
    #[clap(short('p'), long, value_parser)]
    lookup_prefix: Option<String>,
    ///
    #[clap(required = false, value_parser)]
    modalias: Vec<String>,
}

/// hwdb builtin command
pub struct Hwdb {
    hwdb: Rc<RefCell<Option<SdHwdb>>>,
}

impl Hwdb {
    /// create Hwdb
    pub(crate) fn new() -> Self {
        let h = match SdHwdb::new() {
            Ok(h) => Rc::new(RefCell::new(Some(h))),
            Err(e) => {
                log::error!("Failed to new hwdb:{:?}", e);
                Rc::new(RefCell::new(None))
            }
        };
        Hwdb { hwdb: h }
    }

    fn lookup(
        &self,
        dev: Rc<Device>,
        prefix: &Option<String>,
        modalias: String,
        filter: &Option<String>,
        test: bool,
    ) -> Result<i32, nix::Error> {
        if self.hwdb.borrow().is_none() {
            return Err(nix::Error::ENOENT);
        }

        let mut n = 0;
        let mut lookup = modalias;

        if prefix.is_some() {
            lookup = prefix.as_ref().unwrap().to_string() + &lookup;
        }

        let map = self
            .hwdb
            .borrow_mut()
            .as_mut()
            .unwrap()
            .get_properties(lookup)?;

        for it in map.iter() {
            let key = it.0;
            let value = it.1;
            if let Some(f) = filter {
                let pattern = Pattern::new(f).unwrap();
                if !pattern.matches(key) {
                    continue;
                }
            }

            if let Err(e) = dev.add_property(key, value) {
                return Err(e.get_errno());
            }

            if test {
                println!("{}={}", key, value);
            }

            n += 1;
        }
        Ok(n)
    }

    fn search(
        &self,
        dev: Rc<Device>,
        srcdev: Option<Device>,
        subsystem: &Option<String>,
        prefix: Option<String>,
        filter: Option<String>,
        test: bool,
    ) -> Result<i32, nix::Error> {
        let mut r = 0;
        let mut last = false;
        let mut src_dev = match srcdev {
            Some(d) => d,
            None => dev.shallow_clone().unwrap(),
        };

        loop {
            let dsubsys = match src_dev.get_subsystem() {
                Ok(str_subsystem) => str_subsystem,
                Err(_) => {
                    src_dev = match src_dev.get_parent() {
                        Ok(d) => d.shallow_clone().unwrap(),
                        Err(_) => break,
                    };
                    continue;
                }
            };

            /* look only at devices of a specific subsystem */
            if let Some(str_subsystem) = subsystem {
                if &dsubsys != str_subsystem {
                    src_dev = match src_dev.get_parent() {
                        Ok(d) => d.shallow_clone().unwrap(),
                        Err(_) => break,
                    };
                    continue;
                }
            }

            let mut modalias = String::new();
            if let Ok(m) = src_dev.get_property_value("MODALIAS") {
                modalias = m;
            }

            if dsubsys == "usb" {
                if let Ok(devtype) = src_dev.get_devtype() {
                    if devtype == "usb_device" {
                        /* if the usb_device does not have a modalias, compose one */
                        if !modalias.is_empty() {
                            modalias = modalias_usb(&src_dev);
                        }

                        /* avoid looking at any parent device, they are usually just a USB hub */
                        last = true;
                    }
                }
            }

            if modalias.is_empty() {
                src_dev = match src_dev.get_parent() {
                    Ok(d) => d.shallow_clone().unwrap(),
                    Err(_) => break,
                };
                continue;
            }
            log::debug!("hwdb modalias key: {:?}", modalias);

            if let Ok(n) = self.lookup(dev.clone(), &prefix, modalias, &filter, test) {
                r = n;
                if r > 0 {
                    break;
                }
            }

            if last {
                break;
            }

            src_dev = match src_dev.get_parent() {
                Ok(d) => d.shallow_clone().unwrap(),
                Err(_) => break,
            };
        }

        Ok(r)
    }
}

impl Builtin for Hwdb {
    /// builtin command
    fn cmd(
        &self,
        exec_unit: &ExecuteUnit,
        _argc: i32,
        argv: Vec<String>,
        test: bool,
    ) -> Result<bool> {
        let dev = exec_unit.get_device();
        let args = match Args::try_parse_from(argv) {
            Ok(args) => args,
            Err(e) => {
                return Err(Error::Other {
                    msg: format!("Failed to parse argv {:?}", e),
                    errno: nix::Error::EINVAL,
                })
            }
        };

        if self.hwdb.borrow().is_none() {
            return Err(Error::Nix {
                source: nix::Error::EINVAL,
            });
        }

        /* query a specific key given as argument */
        if !args.modalias.is_empty() {
            match self.lookup(
                dev,
                &args.lookup_prefix,
                args.modalias[0].clone(),
                &args.filter,
                test,
            ) {
                Ok(r) => {
                    if 0 == r {
                        return Err(Error::Other {
                            msg: "No entry found from hwdb.".to_string(),
                            errno: nix::Error::ENODATA,
                        });
                    }
                    return Ok(true);
                }
                Err(e) => {
                    return Err(Error::Other {
                        msg: "Failed to look up hwdb".to_string(),
                        errno: e,
                    })
                }
            }
        }

        /* read data from another device than the device we will store the data */
        let mut srcdev = Option::None;
        if let Some(device_id) = args.device {
            srcdev = match Device::from_device_id(&device_id) {
                Ok(srcdev) => Some(srcdev),
                Err(e) => {
                    return Err(Error::Other {
                        msg: format!("Failed to create device object '{}'", device_id),
                        errno: e.get_errno(),
                    });
                }
            };
        }

        match self.search(
            dev,
            srcdev,
            &args.subsystem,
            args.lookup_prefix,
            args.filter,
            test,
        ) {
            Ok(r) => {
                if 0 == r {
                    return Err(Error::Other {
                        msg: "No entry found from hwdb.".to_string(),
                        errno: nix::Error::ENODATA,
                    });
                }
                Ok(true)
            }
            Err(e) => Err(Error::Other {
                msg: "Failed to look up hwdb".to_string(),
                errno: e,
            }),
        }
    }

    /// builtin init function
    fn init(&self) {}

    /// builtin exit function
    fn exit(&self) {}

    /// check whether builtin command should reload
    fn should_reload(&self) -> bool {
        if self.hwdb.borrow().is_some() && self.hwdb.borrow().as_ref().unwrap().should_reload() {
            log::debug!("hwdb needs reloading.");
            return true;
        }
        false
    }

    /// the help of builtin command
    fn help(&self) -> String {
        "Hardware database".to_string()
    }

    /// whether the builtin command can only run once
    fn run_once(&self) -> bool {
        false
    }
}

fn modalias_usb(dev: &Device) -> String {
    let v = match dev.get_sysattr_value("idVendor") {
        Ok(v) => v,
        Err(_) => return "".to_string(),
    };

    let p = match dev.get_sysattr_value("idProduct") {
        Ok(v) => v,
        Err(_) => return "".to_string(),
    };

    let vn: u16 = match v.parse() {
        Ok(vn) => vn,
        Err(_) => return "".to_string(),
    };

    let pn: u16 = match p.parse() {
        Ok(pn) => pn,
        Err(_) => return "".to_string(),
    };

    let mut n = String::new();
    if let Ok(value) = dev.get_sysattr_value("product") {
        n = value;
    }

    format!("usb:v{:04X}p{:04X}:{:?}", vn, pn, n)
}
