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

//! subcommand for devctl info

use crate::subcmds::utils;
use crate::Result;
use basic::fd::{dot_or_dot_dot, xopendirat};
use device::{device_enumerator::DeviceEnumerator, Device};
use nix::dir::Dir;
use nix::fcntl::{AtFlags, OFlag};
use nix::sys::stat::fstatat;
use nix::sys::stat::Mode;
use nix::unistd::{unlinkat, UnlinkatFlags};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;

#[derive(Debug)]
enum QueryType {
    Name,
    Path,
    Symlink,
    Property,
    All,
}

struct SysAttr {
    name: String,
    value: String,
}

pub struct InfoArgs {
    query: Option<String>,
    attribute_walk: bool,
    device_id_of_file: Option<String>,
    export: bool,
    export_prefix: Option<String>,
    export_db: bool,
    cleanup_db: bool,
    root: bool,
    devices: Vec<String>,
}

impl InfoArgs {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        query: Option<String>,
        attribute_walk: bool,
        device_id_of_file: Option<String>,
        export: bool,
        export_prefix: Option<String>,
        export_db: bool,
        cleanup_db: bool,
        root: bool,
        devices: Vec<String>,
    ) -> Self {
        InfoArgs {
            query,
            attribute_walk,
            device_id_of_file,
            export,
            export_prefix,
            export_db,
            cleanup_db,
            root,
            devices,
        }
    }

    /// subcommand for query sysfs or the devmaster database
    pub fn subcommand(&self) -> Result<()> {
        let mut devs = Vec::new();

        if self.export_db {
            return export_devices();
        }
        if self.cleanup_db {
            return cleanup_db();
        }

        if let Some(name) = self.device_id_of_file.as_ref() {
            if !self.devices.is_empty() {
                log::error!("Positional arguments are not allowed with -d/--device-id-of-file.");
                return Err(nix::Error::EINVAL);
            }
            return self.stat_device(name);
        }

        devs.extend(&self.devices);
        if devs.is_empty() {
            log::error!("A device name or path is required");
            return Err(nix::Error::EINVAL);
        }

        if self.attribute_walk && devs.len() > 1 {
            log::error!("Only one device may be specified with -a/--attribute-walk");
            return Err(nix::Error::EINVAL);
        }

        let mut r: Result<()> = Ok(());
        for dev in &self.devices {
            let device = match utils::find_device(dev, "") {
                Ok(d) => d,
                Err(e) => {
                    if e == nix::Error::EINVAL {
                        log::error!("Bad argument {:?}, expected an absolute path in /dev/ or /sys/ or a unit name", dev);
                    } else {
                        log::error!("Unknown device {:?}", dev);
                    }
                    continue;
                }
            };

            if self.query.is_some() {
                r = self.query_device(device);
            } else if self.attribute_walk {
                r = print_device_chain(device);
            } else {
                r = self.query_device(device);
            }
        }

        r
    }

    fn is_export(&self) -> bool {
        self.export || self.export_prefix.is_some()
    }

    fn parse_query_type(&self) -> Result<QueryType> {
        match &self.query {
            Some(q) => {
                if q == "property" || q == "env" {
                    Ok(QueryType::Property)
                } else if q == "name" {
                    Ok(QueryType::Name)
                } else if q == "symlink" {
                    Ok(QueryType::Symlink)
                } else if q == "path" {
                    Ok(QueryType::Path)
                } else if q == "all" {
                    Ok(QueryType::All)
                } else {
                    log::error!("unknown query type");
                    Err(nix::Error::EINVAL)
                }
            }
            None => Ok(QueryType::All),
        }
    }

    fn stat_device(&self, name: &str) -> Result<()> {
        let metadata = match fs::metadata(name) {
            Ok(metadata) => metadata,
            Err(err) => {
                log::error!("Failed to get metadata:{:?} err:{:?}", name, err);
                return Err(nix::Error::EINVAL);
            }
        };

        if self.is_export() {
            match &self.export_prefix {
                Some(p) => {
                    println!("{}MAJOR={}", p, nix::sys::stat::major(metadata.dev()));
                    println!("{}MINOR={}", p, nix::sys::stat::minor(metadata.dev()));
                }
                None => {
                    println!("INFO_MAJOR={}", nix::sys::stat::major(metadata.dev()));
                    println!("INFO_MINOR={}", nix::sys::stat::minor(metadata.dev()));
                }
            }
        } else {
            println!(
                "{}:{}",
                nix::sys::stat::major(metadata.dev()),
                nix::sys::stat::minor(metadata.dev())
            );
        }

        Ok(())
    }

    fn query_device(&self, device: Device) -> Result<()> {
        let query = self.parse_query_type()?;
        match query {
            QueryType::Name => {
                let node = match device.get_devname() {
                    Ok(node) => node,
                    Err(err) => {
                        log::error!("No device node found");
                        return Err(err.get_errno());
                    }
                };

                if !self.root {
                    println!(
                        "{}",
                        Path::new(&node)
                            .strip_prefix("/dev/")
                            .unwrap()
                            .to_str()
                            .unwrap()
                    );
                } else {
                    println!("{}", node);
                }
                Ok(())
            }
            QueryType::Symlink => {
                let mut devlinks_str = String::new();
                for devlink in &device.devlink_iter() {
                    if !self.root {
                        devlinks_str += Path::new(&devlink)
                            .strip_prefix("/dev/")
                            .unwrap()
                            .to_str()
                            .unwrap();
                    } else {
                        devlinks_str += devlink;
                    }
                    devlinks_str += " ";
                }
                devlinks_str = devlinks_str.trim_end().to_string();

                println!("{}", devlinks_str);
                Ok(())
            }
            QueryType::Path => {
                let devpath = match device.get_devpath() {
                    Ok(devpath) => devpath,
                    Err(err) => {
                        log::error!("Failed to get device path");
                        return Err(err.get_errno());
                    }
                };

                println!("{}", devpath);
                Ok(())
            }
            QueryType::Property => {
                for (key, value) in &device.property_iter() {
                    if self.is_export() {
                        match &self.export_prefix {
                            Some(export_prefix) => println!("{}{}='{}'", export_prefix, key, value),
                            None => println!("{}='{}'", key, value),
                        }
                    } else {
                        println!("{}={}", key, value);
                    }
                }
                Ok(())
            }
            QueryType::All => {
                print_record(device, "");
                Ok(())
            }
        }
    }
}

fn print_device_chain(device: Device) -> Result<()> {
    println!(
        "\n\
        Devctl info starts with the device specified by the devpath and then\n\
        walks up the chain of parent devices. It prints for every device\n\
        found, all possible attributes in the devmaster rules key format.\n\
        A rule to match, can be composed by the attributes of the device\n\
        and the attributes from one single parent device.\n\
        "
    );

    print_all_attributes(&device, false)?;

    let mut child = device;
    while let Ok(parent) = child.get_parent() {
        print_all_attributes(&parent, true)?;

        child = parent.shallow_clone().unwrap();
    }

    Ok(())
}

fn print_all_attributes(device: &Device, is_parent: bool) -> Result<()> {
    let mut devpath = String::from("");
    let mut sysname = String::from("");
    let mut subsystem = String::from("");
    let mut driver = String::from("");

    if let Ok(value) = device.get_devpath() {
        devpath = value;
    }

    if let Ok(value) = device.get_sysname() {
        sysname = value;
    }

    if let Ok(value) = device.get_subsystem() {
        subsystem = value;
    }

    if let Ok(value) = device.get_driver() {
        driver = value;
    }

    if is_parent {
        println!("  looking at parent device '{}':", devpath);
        println!("    KERNELS=={:?}", sysname);
        println!("    SUBSYSTEMS=={:?}", subsystem);
        println!("    DRIVERS=={:?}", driver);
    } else {
        println!("  looking at device '{}':", devpath);
        println!("    KERNEL=={:?}", sysname);
        println!("    SUBSYSTEM=={:?}", subsystem);
        println!("    DRIVER=={:?}", driver);
    }

    let mut sysattrs: Vec<SysAttr> = Vec::new();

    let iter = device.sysattr_iter();
    for name in &iter {
        if skip_attribute(name) {
            continue;
        }

        let value = match device.get_sysattr_value(name) {
            Ok(value) => {
                /* skip any values that look like a path */
                if value.starts_with('/') {
                    continue;
                }

                /* skip nonprintable attributes */
                if !value
                    .chars()
                    .all(|c| 0 != unsafe { libc::isprint(c as i32) })
                {
                    continue;
                }

                value
            }
            Err(e) => {
                if e.get_errno() == nix::Error::EACCES || e.get_errno() == nix::Error::EPERM {
                    "(not readable)".to_string()
                } else {
                    continue;
                }
            }
        };

        sysattrs.push(SysAttr {
            name: name.to_string(),
            value,
        });
    }

    sysattrs.sort_by(|a, b| a.name.cmp(&b.name));

    for sysattr in sysattrs {
        if is_parent {
            println!("    ATTRS{{{}}}=={:?}", sysattr.name, sysattr.value);
        } else {
            println!("    ATTR{{{}}}=={:?}", sysattr.name, sysattr.value);
        }
    }
    println!();

    Ok(())
}

fn skip_attribute(name: &str) -> bool {
    /* Those are either displayed separately or should not be shown at all. */
    if name.contains("uevent")
        || name.contains("dev")
        || name.contains("modalias")
        || name.contains("resource")
        || name.contains("driver")
        || name.contains("subsystem")
        || name.contains("module")
    {
        return true;
    }

    false
}

fn print_record(device: Device, prefix: &str) {
    if let Ok(devpath) = device.get_devpath() {
        println!("{}P: {}", prefix, devpath);
    }

    if let Ok(sysname) = device.get_sysname() {
        println!("{}M: {}", prefix, sysname);
    }

    if let Ok(sysnum) = device.get_sysnum() {
        println!("{}R: {}", prefix, sysnum);
    }

    let mut subsys = String::from("");
    if let Ok(subsystem) = device.get_subsystem() {
        subsys = subsystem.clone();
        println!("{}U: {}", prefix, subsystem);
    }

    if let Ok(devnum) = device.get_devnum() {
        if &subsys == "block" {
            println!(
                "{}D: b {}:{}",
                prefix,
                nix::sys::stat::major(devnum),
                nix::sys::stat::minor(devnum)
            );
        } else {
            println!(
                "{}D: c {}:{}",
                prefix,
                nix::sys::stat::major(devnum),
                nix::sys::stat::minor(devnum)
            );
        }
    }

    if let Ok(ifindex) = device.get_ifindex() {
        println!("{}I: {}", prefix, ifindex);
    }

    if let Ok(devname) = device.get_devname() {
        let val = Path::new(&devname)
            .strip_prefix("/dev/")
            .unwrap()
            .to_str()
            .unwrap();
        println!("{}N: {}", prefix, val);
        if let Ok(i) = device.get_devlink_priority() {
            println!("{}L: {}", prefix, i);
        }

        for link in &device.devlink_iter() {
            let val = Path::new(&link)
                .strip_prefix("/dev/")
                .unwrap()
                .to_str()
                .unwrap();
            println!("{}S: {}", prefix, val);
        }
    }

    if let Ok(q) = device.get_diskseq() {
        println!("{}Q: {}", prefix, q);
    }

    if let Ok(driver) = device.get_driver() {
        println!("{}V: {}", prefix, driver);
    }

    for (key, val) in &device.property_iter() {
        println!("{}E: {}={}", prefix, key, val);
    }

    if prefix.is_empty() {
        println!();
    }
}

fn export_devices() -> Result<()> {
    let mut e = DeviceEnumerator::new();

    if let Err(err) = e.allow_uninitialized() {
        log::error!("Failed to set allowing uninitialized flag");
        return Err(err.get_errno());
    }

    if let Err(err) = e.scan_devices() {
        log::error!("Failed to scan devices");
        return Err(err.get_errno());
    }

    for device in e.iter() {
        print_record(device.shallow_clone().unwrap(), "");
    }

    Ok(())
}

fn cleanup_db() -> Result<()> {
    if let Ok(mut dir1) = Dir::open("/run/devmaster/data", OFlag::O_DIRECTORY, Mode::empty()) {
        cleanup_dir(&mut dir1, libc::S_ISVTX, 1);

        if let Ok(mut dir2) = Dir::open("/run/devmaster/links", OFlag::O_DIRECTORY, Mode::empty()) {
            cleanup_dirs_after_db_cleanup(&mut dir2, &dir1);
        }

        if let Ok(mut dir3) = Dir::open("/run/devmaster/tags", OFlag::O_DIRECTORY, Mode::empty()) {
            cleanup_dirs_after_db_cleanup(&mut dir3, &dir1);
        }
    }

    /* static_node-tags is not currently implemented */
    if let Ok(mut dir) = Dir::open(
        "/run/devmaster/static_node-tags",
        OFlag::O_DIRECTORY,
        Mode::empty(),
    ) {
        cleanup_dir(&mut dir, 0, 2);
    }

    /* Do not remove /run/devmaster/watch. It will be handled by devmaster well on restart.
     * And should not be removed by external program when devmaster is running. */

    Ok(())
}

fn cleanup_dir(dir: &mut Dir, mask: libc::mode_t, depth: i32) {
    if depth <= 0 {
        return;
    }

    let dir_raw_fd = dir.as_raw_fd();
    for entry in dir.iter() {
        let dent = match entry {
            Ok(dent) => dent,
            Err(_) => continue,
        };

        if dot_or_dot_dot(dent.file_name().to_str().unwrap()) {
            continue;
        }

        let stats = match fstatat(dir_raw_fd, dent.file_name(), AtFlags::AT_SYMLINK_NOFOLLOW) {
            Ok(stats) => stats,
            Err(_) => continue,
        };

        if (stats.st_mode & mask) != 0 {
            continue;
        }

        if stats.st_mode & libc::S_IFMT == libc::S_IFDIR {
            match xopendirat(
                dir_raw_fd,
                dent.file_name().to_str().unwrap(),
                OFlag::O_NOFOLLOW,
            ) {
                Ok(mut subdir) => cleanup_dir(&mut subdir, mask, depth - 1),
                Err(e) => log::error!(
                    "Failed to open subdirectory {:?}, err{:?}, ignoring",
                    dent.file_name(),
                    e
                ),
            }
            let _ = unlinkat(Some(dir_raw_fd), dent.file_name(), UnlinkatFlags::RemoveDir);
        } else {
            let _ = unlinkat(
                Some(dir_raw_fd),
                dent.file_name(),
                UnlinkatFlags::NoRemoveDir,
            );
        }
    }
}

fn cleanup_dirs_after_db_cleanup(dir: &mut Dir, datadir: &Dir) {
    let dir_raw_fd = dir.as_raw_fd();
    for entry in dir.iter() {
        let dent = match entry {
            Ok(dent) => dent,
            Err(_) => continue,
        };

        if dot_or_dot_dot(dent.file_name().to_str().unwrap()) {
            continue;
        }

        let stats = match fstatat(dir_raw_fd, dent.file_name(), AtFlags::AT_SYMLINK_NOFOLLOW) {
            Ok(stats) => stats,
            Err(_) => continue,
        };

        if stats.st_mode & libc::S_IFMT == libc::S_IFDIR {
            match xopendirat(
                dir_raw_fd,
                dent.file_name().to_str().unwrap(),
                OFlag::O_NOFOLLOW,
            ) {
                Ok(mut subdir) => cleanup_dir_after_db_cleanup(&mut subdir, datadir),
                Err(e) => log::error!(
                    "Failed to open subdirectory {:?}, err{:?}, ignoring",
                    dent.file_name(),
                    e
                ),
            }
            let _ = unlinkat(Some(dir_raw_fd), dent.file_name(), UnlinkatFlags::RemoveDir);
        } else {
            let _ = unlinkat(
                Some(dir_raw_fd),
                dent.file_name(),
                UnlinkatFlags::NoRemoveDir,
            );
        }
    }
}

/*
 * Assume that dir is a directory with file names matching devmaster data base
 * entries for devices in /run/devmaster/data (such as "b8:16"), and removes
 * all files except those that haven't been deleted in /run/devmaster/data
 * (i.e. they were skipped during db cleanup because of the db_persist flag).
 */
fn cleanup_dir_after_db_cleanup(dir: &mut Dir, datadir: &Dir) {
    let dir_raw_fd = dir.as_raw_fd();
    for entry in dir.iter() {
        let dent = match entry {
            Ok(dent) => dent,
            Err(_) => continue,
        };

        if dot_or_dot_dot(dent.file_name().to_str().unwrap()) {
            continue;
        }

        if unsafe {
            libc::faccessat(
                datadir.as_raw_fd(),
                dent.file_name().as_ptr(),
                libc::F_OK,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } >= 0
        {
            /* The corresponding devmaster database file still exists.
             * Assuming the persistent flag is set for the database. */
            continue;
        }

        let _ = unlinkat(
            Some(dir_raw_fd),
            dent.file_name(),
            UnlinkatFlags::NoRemoveDir,
        );
    }
}
