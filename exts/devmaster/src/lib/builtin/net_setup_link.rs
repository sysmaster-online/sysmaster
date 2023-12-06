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

//! net_setup_link builtin
//!

use std::{
    rc::Rc,
    sync::{Arc, RwLock},
};

use crate::{
    builtin::Builtin, config::*, error::Result, error::*, framework::devmaster::Cache, log_dev,
    rules::exec_unit::ExecuteUnit,
};
use basic::naming_scheme::naming_scheme_enabled;
use basic::ResultExt;
use device::{Device, DeviceAction};

/// net_setup_link builtin command
pub struct NetSetupLink {
    pub(crate) cache: Arc<RwLock<Cache>>,
}

struct NetifLink<'a> {
    netif_cfg: &'a NetifConfig,
    netif: Rc<Device>,
    new_name: String,
}

impl<'a> NetifLink<'a> {
    fn new(netif_cfg: &'a NetifConfig, netif: Rc<Device>) -> NetifLink<'a> {
        NetifLink {
            netif_cfg,
            netif,
            new_name: "".to_string(),
        }
    }

    fn apply_cfg(&mut self) -> Result<()> {
        let action = self.netif.get_action().context(DeviceSnafu)?;

        if ![DeviceAction::Add, DeviceAction::Bind, DeviceAction::Move].contains(&action) {
            log_dev!(
                debug,
                &self.netif,
                format!("Skipping to apply .link on '{}' uevent", action)
            );

            self.new_name = self.netif.get_sysname().context(DeviceSnafu)?;
            return Ok(());
        }

        self.generate_new_name()?;

        Ok(())
    }

    fn generate_new_name(&mut self) -> Result<()> {
        let mut new_name = String::new();
        if naming_scheme_enabled() && self.netif_cfg.inner.Link.NamePolicy.is_some() {
            for policy in self
                .netif_cfg
                .inner
                .Link
                .NamePolicy
                .as_ref()
                .unwrap()
                .iter()
            {
                match policy.as_str() {
                    "kernel" => {
                        // todo
                        continue;
                    }
                    "keep" => {
                        // todo
                        continue;
                    }
                    "database" => {
                        match self.netif.get_property_value("ID_NET_NAME_FROM_DATABASE") {
                            Ok(v) => {
                                new_name = v;
                            }
                            Err(_) => {
                                continue;
                            }
                        }
                    }

                    "onboard" => match self.netif.get_property_value("ID_NET_NAME_ONBOARD") {
                        Ok(v) => {
                            new_name = v;
                        }
                        Err(_) => {
                            continue;
                        }
                    },

                    "slot" => match self.netif.get_property_value("ID_NET_NAME_SLOT") {
                        Ok(v) => {
                            new_name = v;
                        }
                        Err(_) => {
                            continue;
                        }
                    },
                    "path" => match self.netif.get_property_value("ID_NET_NAME_PATH") {
                        Ok(v) => {
                            new_name = v;
                        }
                        Err(_) => {
                            continue;
                        }
                    },
                    "mac" => match self.netif.get_property_value("ID_NET_NAME_MAC") {
                        Ok(v) => {
                            new_name = v;
                        }
                        Err(_) => {
                            continue;
                        }
                    },
                    _ => {
                        debug_assert!(false);
                    }
                }

                self.new_name = new_name;
                return Ok(());
            }
        }

        Ok(())
    }
}

impl Builtin for NetSetupLink {
    /// builtin command
    fn cmd(
        &self,
        exec_unit: &ExecuteUnit,
        _argc: i32,
        _argv: Vec<String>,
        test: bool,
    ) -> Result<bool> {
        let device = exec_unit.get_device();

        let netif_cfg_ctx = &self.cache.read().unwrap().netif_cfg_ctx;

        let cfg = match netif_cfg_ctx.get_config(device.clone()) {
            Some(cfg) => cfg,
            None => return Ok(false),
        };

        let mut link = NetifLink::new(cfg, device.clone());

        if let Err(e) = link.apply_cfg() {
            if e.get_errno() == nix::Error::ENODEV {
                log_dev!(
                    debug,
                    device,
                    "Link vanished while applying network configuration"
                );
            } else {
                log_dev!(warn, device, "Could not apply network configuration")
            }
        }

        /* For compatibility with udev, choose the "ID_NET_LINK_FILE" as the property key. */
        self.add_property(
            device.clone(),
            test,
            "ID_NET_LINK_FILE",
            &link.netif_cfg.abs_path,
        )?;

        if !link.new_name.is_empty() {
            self.add_property(device, test, "ID_NET_NAME", &link.new_name)?;
        }

        // todo: dropins

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
        "Configure network link".to_string()
    }

    /// whether the builtin command can only run once
    fn run_once(&self) -> bool {
        false
    }
}
