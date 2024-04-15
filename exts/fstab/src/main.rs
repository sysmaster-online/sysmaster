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

//! fstab is used to parse /etc/fstab in the early stage of boot.
//! # Principle
//! 1. After fstab is started, it will directly parse /etc/fstab to obtain the device, mount point, and mount attributes that need to be mounted.
//! 2. According to the configuration order of /etc/fstab, try to mount in turn.
//! 3. If the corresponding device directory can be found, it will be mounted directly.
//! 4. If the relevant directory cannot be found, create an inotify detection and mount it after the device is ready.
//! # limit
//! 1. The current device only supports configuration as full directory or UUID.

use basic::mount::{is_mount_point, is_swap};
use basic::{FSTAB_PATH, MOUNT_BIN, SWAP_BIN};
use inotify::{EventMask, Inotify, WatchMask};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

pub mod fstab_item;
use fstab_item::FSTabItem;

fn mount_one(fstab_item: &FSTabItem) -> i32 {
    // -.mount is different. It has already been mounted before
    // fstab.service is started. We mount it as rw.
    let mount_status = if fstab_item.mount_point == "/" {
        Command::new(MOUNT_BIN)
            .args(["/", "--options", "remount", "-w"])
            .status()
    } else {
        /* Already mounted */
        if is_mount_point(Path::new(&fstab_item.mount_point))
            && !fstab_item.options.contains("remount")
        {
            log::debug!("{} is already mounted", fstab_item.mount_point);
            return 1;
        }
        Command::new(MOUNT_BIN)
            .args([
                &fstab_item.device_spec,
                &fstab_item.mount_point,
                "--options",
                &fstab_item.options,
                "--types",
                &fstab_item.fs_type,
            ])
            .status()
    };
    let status = match mount_status {
        Ok(status) => status,
        Err(_) => {
            log::error!("Failed to execute {}", MOUNT_BIN);
            return -1;
        }
    };
    let r = match status.code() {
        Some(r) => r,
        None => {
            log::error!("Unexpected error when mount {}", &fstab_item.device_spec);
            return -1;
        }
    };
    if r != 0 {
        log::error!(
            "Failed to mount {}, exitcode: {}",
            &fstab_item.device_spec,
            r
        );
        return -1;
    }
    log::info!("Mounted {}", &fstab_item.device_spec);
    0
}

fn swap_on(fstab_item: &FSTabItem) -> i32 {
    /* Already swapped on */
    if is_swap(Path::new(&fstab_item.device_spec)) {
        log::debug!("{} is already swapped on.", fstab_item.device_spec);
        return 1;
    }

    let status = match Command::new(SWAP_BIN)
        .args([&fstab_item.device_spec])
        .status()
    {
        Ok(status) => status,
        Err(_) => {
            log::error!("Failed to execute {}", SWAP_BIN);
            return -1;
        }
    };
    let r = match status.code() {
        Some(r) => r,
        None => {
            log::error!("Unexpected error when swapon {}", &fstab_item.device_spec);
            return -1;
        }
    };
    if r != 0 {
        log::error!(
            "Failed to swapon {}, exitcode: {}",
            &fstab_item.device_spec,
            r
        );
        return -1;
    }
    log::info!("Swapped on {}", &fstab_item.device_spec);
    0
}

fn consume_one(fstab_item: &mut FSTabItem) {
    let r = match fstab_item.fs_type.as_str() {
        "swap" => swap_on(fstab_item),
        _ => mount_one(fstab_item),
    };
    // set state to 1 if succeeded, -1 if failed.
    fstab_item.state = if r == 0 { 1 } else { -1 };
}

fn watch_devices(fstab_items: &[FSTabItem]) -> (Inotify, HashSet<String>) {
    let mut watch_set: HashSet<String> = HashSet::new();
    let mut inotify = Inotify::init().expect("Failed to init inotify.");
    for fstab_item in fstab_items {
        let file_path = Path::new(&fstab_item.device_spec);
        let dir_path = file_path.parent().unwrap();
        watch_set.insert(String::from(
            file_path.file_name().unwrap().to_str().unwrap(),
        ));
        inotify
            .add_watch(dir_path, WatchMask::CREATE | WatchMask::MOVED_TO)
            .expect("Failed to add watch.");
    }
    (inotify, watch_set)
}

fn main() {
    log::init_log_to_console("fstab", log::Level::Info);
    let mut fstab_items: Vec<FSTabItem> = fstab_item::parse(FSTAB_PATH);

    // inotify: monitor, watch_set: what we care.
    let (mut inotify, _watch_set) = watch_devices(&fstab_items);

    let mut complete_num = 0;
    loop {
        // Mount/swap what we can.
        for fstab_item in &mut fstab_items {
            if fstab_item.state != 0 || !Path::new(&fstab_item.device_spec).exists() {
                continue;
            }
            consume_one(fstab_item);
            complete_num += 1;
        }
        if complete_num >= fstab_items.len() {
            break;
        }

        // use inotify to wait device ready.
        let mut buffer = [0u8; 4096];
        let mut watch_updated = false;
        while !watch_updated {
            let events = inotify
                .read_events_blocking(&mut buffer)
                .expect("Failed to read events.");
            for event in events {
                if event.mask == EventMask::CREATE || event.mask == EventMask::MOVED_TO {
                    if let Some(name) = event.name {
                        log::debug!("File created or moved: {:?}", name);
                    }
                    watch_updated = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use inotify::EventMask;
    use nix::unistd::getuid;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use std::sync::mpsc;
    use std::thread;

    use crate::{fstab_item, mount_one, watch_devices};

    fn create_fstab_items() -> Vec<fstab_item::FSTabItem> {
        // Create fstab_items and directories.
        let src_path = Path::new("/tmp/fstab_test/src");
        let dst_path = Path::new("/tmp/fstab_test/dst");
        let fstab_str = vec![
            src_path.to_str().unwrap(),
            dst_path.to_str().unwrap(),
            "ext4",
            "bind",
            "0",
            "0",
        ];
        let fstab_items = vec![fstab_item::FSTabItem::new(fstab_str)];
        assert_eq!(fstab_items.len(), 1);
        fstab_items
    }

    fn clean() {
        // Clean
        if !Path::exists(Path::new("/tmp/fstab_test")) {
            return;
        }
        if let Err(why) = fs::remove_dir_all("/tmp/fstab_test") {
            panic!("Failed to remove {:?}: {:?}.", "/tmp/fstab_test", why);
        }
    }

    #[test]
    fn test_mount_one() {
        let fstab_items = create_fstab_items();
        assert_eq!(mount_one(&fstab_items[0]), -1);
        if !getuid().is_root() {
            println!("Mount must be run under superuser, skipping.");
            return;
        }
        let src_path = Path::new(&fstab_items[0].device_spec);
        let dst_path = Path::new(&fstab_items[0].mount_point);
        if !(Path::exists(src_path) && src_path.is_dir()) {
            if let Err(why) = fs::create_dir_all(src_path) {
                clean();
                panic!("Failed to create {:?}: {:?}", src_path, why);
            }
        }
        if !(Path::exists(dst_path) && dst_path.is_dir()) {
            if let Err(why) = fs::create_dir_all(dst_path) {
                clean();
                panic!("Failed to create {:?}: {:?}", dst_path, why);
            }
        }
        assert_eq!(mount_one(&fstab_items[0]), 0);
        // cleanup: umount and delete
        let status = Command::new("/usr/bin/umount")
            .args([dst_path.as_os_str().to_str().unwrap()])
            .status();
        let status = match status {
            Ok(status) => status,
            Err(why) => {
                panic!("Failed to execute /usr/bin/umount: {}", why);
            }
        };
        let r = match status.code() {
            Some(r) => r,
            None => {
                panic!(
                    "Unexpected error when umount {}",
                    dst_path.as_os_str().to_str().unwrap()
                );
            }
        };
        assert_eq!(r, 0);
        clean();
    }

    #[test]
    fn test_watch_devices() {
        let fstab_items = create_fstab_items();
        let src_path = String::from(&fstab_items[0].device_spec);
        let dst_path = String::from(&fstab_items[0].mount_point);
        if let Err(why) = fs::create_dir_all(&dst_path) {
            clean();
            panic!("Failed to create dir ({:?}): {:?}.", dst_path, why);
        }

        let (tx, rx) = mpsc::channel();
        // Create
        thread::spawn(move || {
            if let Err(why) = rx.recv() {
                clean();
                panic!("Failed to receive ready message: {:?}", why);
            }
            if let Err(why) = fs::File::create(&src_path) {
                clean();
                panic!("Failed to create file ({:?}): {:?}.", src_path, why);
            }
        });

        // use inotify to wait device ready.
        let (mut inotify, watch_set) = watch_devices(&fstab_items);
        let mut buffer = [0u8; 4096];
        if let Err(why) = tx.send("ready") {
            clean();
            panic!("Failed to send ready message: {:?}", why);
        }
        let events = inotify
            .read_events_blocking(&mut buffer)
            .expect("Failed to read events.");
        for event in events {
            if event.mask == EventMask::CREATE
                && watch_set.contains(event.name.unwrap().to_str().unwrap())
            {
                println!("Ok.");
            }
        }

        clean();
    }
}
