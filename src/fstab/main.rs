use inotify::{EventMask, Inotify, WatchMask};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

pub mod fstab_item;
use fstab_item::FSTabItem;

const MOUNT_BIN: &str = "/usr/bin/mount";
const SWAP_BIN: &str = "/usr/sbin/swapon";
const FSTAB_PATH: &str = "/etc/fstab";

fn mount_one(fstab_item: &FSTabItem) -> i32 {
    let mount_status;
    // -.mount is different. It has already been mounted before
    // fstab.service is started. We mount it as rw.
    if fstab_item.mount_point == "/" {
        mount_status = Command::new(MOUNT_BIN)
            .args(["/", "--options", "remount", "-w"])
            .status()
    } else {
        mount_status = Command::new(MOUNT_BIN)
            .args([
                &fstab_item.device_spec,
                &fstab_item.mount_point,
                "--options",
                &fstab_item.options,
                "--types",
                &fstab_item.fs_type,
            ])
            .status()
    }
    match mount_status {
        Ok(status) => {
            if let Some(r) = status.code() {
                if r != 0 {
                    log::error!(
                        "Failed to mount {}, exitcode: {}",
                        &fstab_item.device_spec,
                        r
                    );
                } else {
                    log::info!("Mounted {}", &fstab_item.device_spec);
                }
                r
            } else {
                log::error!("Unexpected error when mount {}", &fstab_item.device_spec);
                -1
            }
        }
        Err(_) => {
            log::error!("Failed to executing {}!", MOUNT_BIN);
            -1
        }
    }
}

fn swap_on(fstab_item: &FSTabItem) -> i32 {
    match Command::new(SWAP_BIN)
        .args([&fstab_item.device_spec])
        .status()
    {
        Ok(status) => {
            if let Some(r) = status.code() {
                if r != 0 {
                    log::error!(
                        "Failed to swapon {}, exitcode: {}",
                        &fstab_item.device_spec,
                        r
                    );
                } else {
                    log::info!("Swapped on {}", &fstab_item.device_spec);
                }
                r
            } else {
                log::error!("Unexpected error when swapon {}", &fstab_item.device_spec);
                -1
            }
        }
        Err(_) => {
            log::error!("Failed to executing {}!", SWAP_BIN);
            -1
        }
    }
}

fn consume_one(fstab_item: &mut FSTabItem) {
    let r = match fstab_item.fs_type.as_str() {
        "swap" => swap_on(&fstab_item),
        _ => mount_one(&fstab_item),
    };
    // set state to 1 if succeeded, -1 if failed.
    fstab_item.state = if r == 0 { 1 } else { -1 };
}

fn watch_devices(fstab_items: &Vec<FSTabItem>) -> (Inotify, HashSet<String>) {
    let mut watch_set: HashSet<String> = HashSet::new();
    let mut inotify = Inotify::init().expect("Failed to init inotify.");
    for fstab_item in fstab_items {
        let file_path = Path::new(&fstab_item.device_spec);
        let dir_path = file_path.parent().unwrap();
        watch_set.insert(String::from(
            file_path.file_name().unwrap().to_str().unwrap(),
        ));
        inotify
            .add_watch(dir_path, WatchMask::CREATE)
            .expect("Failed to add watch.");
    }
    (inotify, watch_set)
}

fn main() {
    let mut fstab_items: Vec<FSTabItem> = fstab_item::parse(FSTAB_PATH);

    // inotify: monitor, watch_set: what we care.
    let (mut inotify, watch_set) = watch_devices(&fstab_items);

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
                if event.mask == EventMask::CREATE
                    && watch_set.contains(event.name.unwrap().to_str().unwrap())
                {
                    log::debug!("File created: {:?}", event.name.unwrap());
                    watch_updated = true;
                }
            }
        }
    }
}
