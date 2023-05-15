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

//! blkid builtin
//!

use crate::builtin::Builtin;
use crate::builtin::Netlink;
use crate::error::{Error, Result};
use clap::Parser;
use device::Device;
use libblkid_rs::BlkidFltr;
use libblkid_rs::BlkidProbe;
use libblkid_rs::BlkidSublks;
use libblkid_rs::BlkidSublksFlags;
use libblkid_rs::BlkidUsageFlag;
use libblkid_rs::BlkidUsageFlags;
use nix::fcntl::OFlag;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

/// blkid builtin command
pub struct Blkid;

#[derive(Parser, Debug)]
struct Args {
    /// offset
    #[clap(short('o'), long, default_value_t = 0)]
    offset: i64,

    /// hint
    #[clap(short('H'), long, default_value_t = String::new())]
    hint: String,

    /// noraid
    #[clap(short('R'), long, required = false)]
    noraid: bool,
}

impl Args {
    fn is_set_offset(&self) -> bool {
        self.offset >= 0
    }

    fn offset(&self) -> i64 {
        self.offset
    }

    fn noraid(&self) -> bool {
        self.noraid
    }
}

macro_rules! op_command_err {
    ($message:literal) => {
        |e| {
            log::error!("{}: {:?}", $message, e);
            Error::BuiltinCommandError {
                msg: $message.to_string(),
            }
        }
    };
}

impl Blkid {
    fn print_property(
        &self,
        device: Arc<Mutex<Device>>,
        name: &str,
        value: &str,
        test: bool,
    ) -> Result<(), Error> {
        match name {
            "TYPE" => self.add_property(device, test, "ID_FS_TYPE".to_string(), value.to_string()),
            "USAGE" => {
                self.add_property(device, test, "ID_FS_USAGE".to_string(), value.to_string())
            }
            "VERSION" => {
                self.add_property(device, test, "ID_FS_VERSION".to_string(), value.to_string())
            }
            "UUID" => {
                self.add_property(
                    device.clone(),
                    test,
                    "ID_FS_UUID".to_string(),
                    libblkid_rs::safe_string(value).unwrap(),
                )?;
                self.add_property(
                    device,
                    test,
                    "ID_FS_UUID_ENC".to_string(),
                    libblkid_rs::encode_string(value).unwrap(),
                )
            }
            "UUID_SUB" => {
                self.add_property(
                    device.clone(),
                    test,
                    "ID_FS_UUID_SUB".to_string(),
                    libblkid_rs::safe_string(value).unwrap(),
                )?;
                self.add_property(
                    device,
                    test,
                    "ID_FS_UUID_SUB_ENC".to_string(),
                    libblkid_rs::encode_string(value).unwrap(),
                )
            }
            "LABEL" => {
                self.add_property(
                    device.clone(),
                    test,
                    "ID_FS_LABEL".to_string(),
                    libblkid_rs::safe_string(value).unwrap(),
                )?;
                self.add_property(
                    device,
                    test,
                    "ID_FS_LABEL_ENC".to_string(),
                    libblkid_rs::encode_string(value).unwrap(),
                )
            }
            "PTTYPE" => self.add_property(
                device,
                test,
                "ID_PART_TABLE_TYPE".to_string(),
                value.to_string(),
            ),
            "PTUUID" => self.add_property(
                device,
                test,
                "ID_PART_TABLE_UUID".to_string(),
                value.to_string(),
            ),
            "PART_ENTRY_NAME" => self.add_property(
                device,
                test,
                "ID_PART_ENTRY_NAME".to_string(),
                libblkid_rs::encode_string(value).unwrap(),
            ),
            "PART_ENTRY_TYPE" => self.add_property(
                device,
                test,
                "ID_PART_ENTRY_TYPE".to_string(),
                libblkid_rs::encode_string(value).unwrap(),
            ),
            start_with if start_with.starts_with("PART_ENTRY_") => self.add_property(
                device,
                test,
                "ID_".to_string() + start_with,
                value.to_string(),
            ),
            "SYSTEM_ID" => self.add_property(
                device,
                test,
                "ID_FS_SYSTEM_ID".to_string(),
                libblkid_rs::encode_string(value).unwrap(),
            ),
            "PUBLISHER_ID" => self.add_property(
                device,
                test,
                "ID_FS_PUBLISHER_ID".to_string(),
                libblkid_rs::encode_string(value).unwrap(),
            ),
            "APPLICATION_ID" => self.add_property(
                device,
                test,
                "ID_FS_APPLICATION_ID".to_string(),
                libblkid_rs::encode_string(value).unwrap(),
            ),
            "BOOT_SYSTEM_ID" => self.add_property(
                device,
                test,
                "ID_FS_BOOT_SYSTEM_ID".to_string(),
                libblkid_rs::encode_string(value).unwrap(),
            ),
            "VOLUME_ID" => self.add_property(
                device,
                test,
                "ID_FS_VOLUME_ID".to_string(),
                libblkid_rs::encode_string(value).unwrap(),
            ),
            "LOGICAL_VOLUME_ID" => self.add_property(
                device,
                test,
                "ID_FS_LOGICAL_VOLUME_ID".to_string(),
                libblkid_rs::encode_string(value).unwrap(),
            ),
            "VOLUME_SET_ID" => self.add_property(
                device,
                test,
                "ID_FS_VOLUME_SET_ID".to_string(),
                libblkid_rs::encode_string(value).unwrap(),
            ),
            "DATA_PREPARER_ID" => self.add_property(
                device,
                test,
                "ID_FS_DATA_PREPARER_ID".to_string(),
                libblkid_rs::encode_string(value).unwrap(),
            ),
            _ => {
                log::warn!("not match key: {name}={value}");
                Ok(())
            }
        }
    }

    fn probe_superblocks(&self, probe: &mut BlkidProbe) -> Result<()> {
        let fd = probe
            .get_fd()
            .map_err(op_command_err!("probe get fd error"))?;
        let file_stat = nix::sys::stat::fstat(fd).map_err(op_command_err!("fstat err"))?;
        probe
            .enable_partitions(true)
            .map_err(op_command_err!("enable partitions error"))?;

        if !nix::sys::stat::SFlag::from_bits_truncate(file_stat.st_mode)
            .contains(nix::sys::stat::SFlag::S_IFCHR)
            && probe.get_size() <= 1024 * 1440
            && probe.is_wholedisk()
        {
            if let Err(e) = probe.enable_superblocks(false) {
                log::warn!("enable superblocks error: {:?}", e);
            }
            probe
                .do_fullprobe()
                .map_err(op_command_err!("do fullprobe error"))?;

            if probe.lookup_value("PTTYPE").is_ok() {
                return Ok(());
            }
        }

        if let Err(e) = probe.enable_superblocks(true) {
            log::warn!("enable superblocks error: {:?}", e);
        }

        probe
            .do_safeprobe()
            .map_err(op_command_err!("do safeprobe error"))?;
        Ok(())
    }
}

impl Builtin for Blkid {
    /// builtin command
    fn cmd(
        &self,
        device: Arc<Mutex<Device>>,
        _ret_rtnl: &mut RefCell<Option<Netlink>>,
        _argc: i32,
        argv: Vec<String>,
        test: bool,
    ) -> Result<bool> {
        let args = Args::try_parse_from(argv).map_err(op_command_err!("parse argv error"))?;
        if !args.is_set_offset() {
            return Err(Error::BuiltinCommandError {
                msg: "offset invalid!".to_string(),
            });
        }

        let mut probe =
            BlkidProbe::new().map_err(op_command_err!("create libblkid probe error!"))?;

        probe
            .set_superblock_flags(BlkidSublksFlags::new(vec![
                BlkidSublks::Label,
                BlkidSublks::Uuid,
                BlkidSublks::Type,
                BlkidSublks::Sectype,
                BlkidSublks::Usage,
                BlkidSublks::Version,
            ]))
            .map_err(op_command_err!("set superblock flags error"))?;

        if args.noraid() {
            probe
                .filter_superblock_usage(
                    BlkidFltr::Notin,
                    BlkidUsageFlags::new(vec![BlkidUsageFlag::Raid]),
                )
                .map_err(op_command_err!("filter superblock usage error"))?;
        }

        let fd = device
            .lock()
            .map_err(op_command_err!("device lock error"))?
            .open(OFlag::O_CLOEXEC | OFlag::O_RDONLY | OFlag::O_NONBLOCK)
            .map_err(op_command_err!("device open error"))?;

        probe
            .set_device(fd, args.offset(), 0)
            .map_err(op_command_err!("set device error"))?;
        self.probe_superblocks(&mut probe)?;

        let root_partition = device
            .lock()
            .map_err(op_command_err!("device lock error"))?
            .get_property_value("ID_PART_GPT_AUTO_ROOT_UUID".to_string())
            .map_or(String::new(), |e| e);

        let num_values = probe
            .numof_values()
            .map_err(op_command_err!("numof values error"))?;

        for i in 0..num_values {
            if let Ok(pair) = probe.get_value(i as u32) {
                if let Err(e) = self.print_property(device.clone(), &pair.0, &pair.1, test) {
                    log::error!("device add property error: {:?}", e);
                }
                if !root_partition.is_empty()
                    && root_partition.eq(&pair.1)
                    && pair.0.eq("PART_ENTRY_UUID")
                {
                    let _ = device
                        .lock()
                        .map_err(op_command_err!("device lock error"))?
                        .add_property("ID_PART_GPT_AUTO_ROOT".to_string(), "1".to_string());
                }
            }
            continue;
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
        "Filesystem and partition probing".to_string()
    }

    /// whether the builtin command can only run once
    fn run_once(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use basic::logger;
    use device::Device;
    use std::path;

    #[test]
    #[ignore]
    fn test_builtin_example() {
        logger::init_log_to_console("test_builtin_example", log::LevelFilter::Debug);
        let path = path::Path::new("/sys/class/block/");
        if !path.exists() {
            return;
        }
        let mut rtnl = RefCell::<Option<Netlink>>::from(None);
        let builtin = Blkid {};

        for entry in path.read_dir().unwrap().flatten() {
            let file_name = String::from(entry.file_name().to_str().unwrap());
            let dev_path = format!("/dev/{}", file_name);
            log::info!("{} device probe:", dev_path);
            let device = Arc::new(Mutex::new(Device::from_devname(dev_path.clone()).unwrap()));
            builtin
                .cmd(device, &mut rtnl, 0, vec!["blkid".to_string()], true)
                .unwrap();
        }
    }
}
