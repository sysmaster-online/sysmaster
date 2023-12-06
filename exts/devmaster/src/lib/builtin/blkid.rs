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
#![allow(deprecated)]
use crate::builtin::Builtin;
use crate::error::{Error, Result};
use crate::rules::exec_unit::ExecuteUnit;
use basic::uuid::GptAttribute;
use basic::uuid::Uuid;
use basic::uuid::GPT_ESP;
use basic::uuid::GPT_ROOT_NATIVE;
use basic::uuid::GPT_XBOOTLDR;
use blkid_rs::BlkidFltr;
use blkid_rs::BlkidPartition;
#[cfg(blkid = "libblkid_2_37")]
use blkid_rs::BlkidProbPartsFlags;
use blkid_rs::BlkidProbe;
use blkid_rs::BlkidSublksFlags;
use blkid_rs::BlkidUsageFlags;
use clap::Parser;
use device::Device;
use nix::fcntl::OFlag;
use std::fs::File;
use std::io::Read;
use std::os::unix::prelude::AsRawFd;
use std::rc::Rc;

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

    #[cfg(blkid = "libblkid_2_37")]
    fn hint(&self) -> Option<&str> {
        if self.hint.is_empty() {
            None
        } else {
            Some(self.hint.as_str())
        }
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
        device: Rc<Device>,
        name: &str,
        value: &str,
        test: bool,
    ) -> Result<(), Error> {
        match name {
            "TYPE" => self.add_property(device, test, "ID_FS_TYPE", value),
            "USAGE" => self.add_property(device, test, "ID_FS_USAGE", value),
            "VERSION" => self.add_property(device, test, "ID_FS_VERSION", value),
            "UUID" => {
                self.add_property(
                    device.clone(),
                    test,
                    "ID_FS_UUID",
                    &blkid_rs::safe_string(value).unwrap(),
                )?;
                self.add_property(
                    device,
                    test,
                    "ID_FS_UUID_ENC",
                    &blkid_rs::encode_string(value).unwrap(),
                )
            }
            "UUID_SUB" => {
                self.add_property(
                    device.clone(),
                    test,
                    "ID_FS_UUID_SUB",
                    &blkid_rs::safe_string(value).unwrap(),
                )?;
                self.add_property(
                    device,
                    test,
                    "ID_FS_UUID_SUB_ENC",
                    &blkid_rs::encode_string(value).unwrap(),
                )
            }
            "LABEL" => {
                self.add_property(
                    device.clone(),
                    test,
                    "ID_FS_LABEL",
                    &blkid_rs::safe_string(value).unwrap(),
                )?;
                self.add_property(
                    device,
                    test,
                    "ID_FS_LABEL_ENC",
                    &blkid_rs::encode_string(value).unwrap(),
                )
            }
            "PTTYPE" => self.add_property(device, test, "ID_PART_TABLE_TYPE", value),
            "PTUUID" => self.add_property(device, test, "ID_PART_TABLE_UUID", value),
            "PART_ENTRY_NAME" => self.add_property(
                device,
                test,
                "ID_PART_ENTRY_NAME",
                &blkid_rs::encode_string(value).unwrap(),
            ),
            "PART_ENTRY_TYPE" => self.add_property(
                device,
                test,
                "ID_PART_ENTRY_TYPE",
                &blkid_rs::encode_string(value).unwrap(),
            ),
            start_with if start_with.starts_with("PART_ENTRY_") => {
                self.add_property(device, test, &("ID_".to_string() + start_with), value)
            }
            "SYSTEM_ID" => self.add_property(
                device,
                test,
                "ID_FS_SYSTEM_ID",
                &blkid_rs::encode_string(value).unwrap(),
            ),
            "PUBLISHER_ID" => self.add_property(
                device,
                test,
                "ID_FS_PUBLISHER_ID",
                &blkid_rs::encode_string(value).unwrap(),
            ),
            "APPLICATION_ID" => self.add_property(
                device,
                test,
                "ID_FS_APPLICATION_ID",
                &blkid_rs::encode_string(value).unwrap(),
            ),
            "BOOT_SYSTEM_ID" => self.add_property(
                device,
                test,
                "ID_FS_BOOT_SYSTEM_ID",
                &blkid_rs::encode_string(value).unwrap(),
            ),
            "VOLUME_ID" => self.add_property(
                device,
                test,
                "ID_FS_VOLUME_ID",
                &blkid_rs::encode_string(value).unwrap(),
            ),
            "LOGICAL_VOLUME_ID" => self.add_property(
                device,
                test,
                "ID_FS_LOGICAL_VOLUME_ID",
                &blkid_rs::encode_string(value).unwrap(),
            ),
            "VOLUME_SET_ID" => self.add_property(
                device,
                test,
                "ID_FS_VOLUME_SET_ID",
                &blkid_rs::encode_string(value).unwrap(),
            ),
            "DATA_PREPARER_ID" => self.add_property(
                device,
                test,
                "ID_FS_DATA_PREPARER_ID",
                &blkid_rs::encode_string(value).unwrap(),
            ),
            _ => {
                log::warn!("not match key: {}={}", name, value);
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
        #[cfg(blkid = "libblkid_2_37")]
        if let Err(e) = probe.set_partitions_flags(BlkidProbPartsFlags::ENTRY_DETAILS) {
            log::warn!("set partitions flags error: {:?}", e);
        }

        if let Err(e) = probe.enable_superblocks(true) {
            log::warn!("enable superblocks error: {:?}", e);
        }

        probe
            .do_safeprobe()
            .map_err(op_command_err!("do safeprobe error"))?;
        Ok(())
    }

    fn find_gpt_root(&self, probe: &mut BlkidProbe) -> std::result::Result<String, ()> {
        let mut partlist = match probe.get_partitions() {
            Some(partlist) => partlist,
            None => {
                log::warn!("get_partitions failed");
                return Err(());
            }
        };

        let part_num = partlist.numof_partitions().map_or(0, |_| {
            log::warn!("get numof_partitions failed.");
            0
        });
        let mut found_esp_or_xbootldr = false;
        let mut root_id = Uuid::new();
        for i in 0..part_num {
            let part = match partlist.get_partition(i) {
                Some(part) => part,
                None => continue,
            };

            let uuid = if let Some(uuid) = self.get_uuid_id128(&part) {
                uuid
            } else {
                continue;
            };

            let id128_type = if let Some(id128) = self.get_type_id128(&part) {
                id128
            } else {
                continue;
            };

            if id128_type == GPT_ESP || id128_type == GPT_XBOOTLDR {
                let esp_or_xbootldr = match self.get_esp_part_uuid() {
                    None => return Err(()),
                    Some(uuid) => uuid,
                };

                if esp_or_xbootldr == uuid {
                    found_esp_or_xbootldr = true;
                }
            } else if id128_type == GPT_ROOT_NATIVE {
                match GptAttribute::from_bits(part.get_flags()) {
                    None => continue,
                    Some(attr) => {
                        if attr.contains(GptAttribute::GPT_FLAG_NO_AUTO) {
                            continue;
                        }
                    }
                }

                if root_id.is_null() {
                    root_id = uuid;
                }
            }

            if found_esp_or_xbootldr && !root_id.is_null() {
                return Ok(root_id.to_string());
            }
        }
        Err(())
    }

    fn get_esp_part_uuid(&self) -> Option<Uuid> {
        let mut file = match File::open(
            "/sys/firmware/efi/efivars/LoaderDevicePartUUID-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f",
        ) {
            Ok(file) => file,
            Err(err) => {
                log::error!("Failed to open LoaderDevicePartUUID: {}", err);
                return None;
            }
        };
        let mut raw_data = [0; 36];
        match file.read(&mut raw_data) {
            Ok(size) => {
                if size < raw_data.len() {
                    log::error!("Failed to read size of LoaderDevicePartUUID < 64");
                    return None;
                }
            }
            Err(err) => {
                log::error!("Failed to read LoaderDevicePartUUID: {}", err);
                return None;
            }
        }

        let s: Vec<u16> = raw_data
            .chunks_exact(2)
            .map(|v| (v[1] as u16) << 8 | v[0] as u16)
            .collect();

        match String::from_utf16(s.as_ref()) {
            Err(err) => {
                log::error!("Failed to from utf16: {}", err);
                None
            }
            Ok(s) => Uuid::from_string(&s),
        }
    }

    fn get_uuid_id128(&self, part: &BlkidPartition) -> Option<Uuid> {
        let str_uuid = part.get_uuid()?;
        Uuid::from_string(&str_uuid)
    }

    fn get_type_id128(&self, part: &BlkidPartition) -> Option<Uuid> {
        let str_uuid = part.get_type_string()?;
        Uuid::from_string(&str_uuid)
    }
}

impl Builtin for Blkid {
    /// builtin command
    fn cmd(
        &self,
        exec_unit: &ExecuteUnit,
        _argc: i32,
        argv: Vec<String>,
        test: bool,
    ) -> Result<bool> {
        let device = exec_unit.get_device();
        let subsystem = device
            .get_subsystem()
            .map_err(op_command_err!("device get_subsystem error"))?;

        if subsystem != *"block" {
            let syspath = device
                .get_syspath()
                .map_err(op_command_err!("device get_syspath error"))?;
            log::warn!("blkid can only probe block devices, ignoring {}", syspath);
            return Ok(false);
        }

        let args = Args::try_parse_from(argv).map_err(op_command_err!("parse argv error"))?;
        if !args.is_set_offset() {
            return Err(Error::BuiltinCommandError {
                msg: "offset invalid!".to_string(),
            });
        }

        let mut probe = BlkidProbe::new().unwrap();

        probe
            .set_superblock_flags(
                BlkidSublksFlags::LABEL
                    | BlkidSublksFlags::UUID
                    | BlkidSublksFlags::TYPE
                    | BlkidSublksFlags::SECTYPE
                    | BlkidSublksFlags::USAGE
                    | BlkidSublksFlags::VERSION,
            )
            .map_err(op_command_err!("set superblock flags error"))?;

        if args.noraid() {
            probe
                .filter_superblock_usage(BlkidFltr::NOTIN, BlkidUsageFlags::RAID)
                .map_err(op_command_err!("filter superblock usage error"))?;
        }

        #[cfg(blkid = "libblkid_2_37")]
        if let Some(hint) = args.hint() {
            probe
                .set_hint(hint, 0)
                .map_err(op_command_err!("set_hint error"))?;
        }

        let file = device
            .open(OFlag::O_CLOEXEC | OFlag::O_RDONLY | OFlag::O_NONBLOCK)
            .map_err(op_command_err!("device open error"))?;

        probe
            .set_device(file.as_raw_fd(), args.offset(), 0)
            .map_err(op_command_err!("set device error"))?;
        self.probe_superblocks(&mut probe)?;

        let root_partition = device
            .get_property_value("ID_PART_GPT_AUTO_ROOT_UUID")
            .map_or(String::new(), |e| e);

        let num_values = probe
            .numof_values()
            .map_err(op_command_err!("numof values error"))?;

        let mut is_gpt = false;
        for i in 0..num_values {
            if let Ok(pair) = probe.get_value(i as u32) {
                if let Err(e) = self.print_property(device.clone(), &pair.0, &pair.1, test) {
                    log::error!("device add property error: {:?}", e);
                }

                if pair.0.eq("PTTYPE") && pair.1.eq("gpt") {
                    is_gpt = true;
                }

                if !root_partition.is_empty()
                    && root_partition.eq(&pair.1)
                    && pair.0.eq("PART_ENTRY_UUID")
                {
                    let _ = self.add_property(device.clone(), test, "ID_PART_GPT_AUTO_ROOT", "1");
                }
            }
            continue;
        }

        if is_gpt {
            if let Ok(uuid) = self.find_gpt_root(&mut probe) {
                let _ = self.add_property(device, test, "ID_PART_GPT_AUTO_ROOT_UUID", &uuid);
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
    use device::Device;
    use std::path;

    #[test]
    #[ignore]
    fn test_builtin_blkid() {
        log::init_log(
            "test_builtin_blkid",
            log::Level::Debug,
            vec!["console"],
            "",
            0,
            0,
            false,
        );
        let path = path::Path::new("/sys/class/block/");
        if !path.exists() {
            return;
        }
        let builtin = Blkid {};

        for entry in path.read_dir().unwrap().flatten() {
            let file_name = String::from(entry.file_name().to_str().unwrap());
            let dev_path = format!("/dev/{}", file_name);
            log::info!("{} device probe:", dev_path);
            let device = Rc::new(Device::from_devname(&dev_path).unwrap());
            let exec_unit = ExecuteUnit::new(device);

            builtin
                .cmd(&exec_unit, 0, vec!["blkid".to_string()], true)
                .unwrap();
        }
    }
}
