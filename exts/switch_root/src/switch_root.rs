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

use nix::{
    dir::Type,
    fcntl::{AtFlags, OFlag},
    mount::{self, MntFlags, MsFlags},
    sys::{
        stat::{self, fstatat, Mode},
        statfs::{self, FsType},
    },
    unistd,
};
use std::{
    ffi::{CString, OsStr},
    fs,
    os::{linux::fs::MetadataExt, unix::prelude::AsRawFd},
    path::Path,
};

#[cfg(target_env = "musl")]
type FsTypeT = libc::c_ulong;

#[cfg(not(target_env = "musl"))]
type FsTypeT = libc::c_long;

pub fn switch_root(new_root: &str) -> bool {
    let old_root = "/";

    if old_root == new_root {
        return false;
    }

    if !root_is_vaild(old_root) {
        eprintln!("cannot access /");
        return false;
    }

    if !root_is_vaild(&new_root) {
        eprintln!("cannot access new_root:{}", new_root);
        return false;
    }

    let mounts_path = ["/dev", "/proc", "/sys", "/run"];
    for path in mounts_path {
        mount_move(path, old_root, new_root);
    }

    if let Err(e) = unistd::chdir(new_root) {
        eprintln!("Failed to change directory to {}: {}", &new_root, e);
        return false;
    }

    let old_root_dir = match nix::dir::Dir::open(old_root, OFlag::O_RDONLY, Mode::S_IXUSR) {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("open {} failed: {}", old_root, e);
            return false;
        }
    };

    if let Err(e) = mount::mount(
        Some::<&str>(new_root),
        old_root,
        None::<&str>,
        MsFlags::MS_MOVE,
        None::<&str>,
    ) {
        eprintln!("failed to mount moving {} to {}: {}", new_root, old_root, e);
        return false;
    }

    if let Err(e) = unistd::chroot(".") {
        eprintln!("failed to change root : {}", e);
        return false;
    }

    if let Err(e) = unistd::chdir(old_root) {
        eprintln!("failed to change directory to {}: {}", old_root, e);
        return false;
    }

    if in_initrd(old_root) {
        remove_dir_all_by_dir(old_root_dir);
    }
    true
}

fn remove_dir_all_by_dir(mut dir: nix::dir::Dir) {
    let dir_st_dev = match stat::fstat(dir.as_raw_fd()) {
        Ok(file_stat) => file_stat.st_dev,
        Err(e) => {
            eprintln!("stat of {:?} failed: {}", dir, e);
            return;
        }
    };

    let dir_raw_fd = dir.as_raw_fd();
    for it in dir.iter() {
        let entry = match it {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("iter dir failed: {}", e);
                continue;
            }
        };

        let dot_cstr = CString::new(".").unwrap();
        let dot_dot_cstr = CString::new("..").unwrap();
        if entry.file_name().eq(dot_cstr.as_c_str())
            || entry.file_name().eq(dot_dot_cstr.as_c_str())
        {
            continue;
        }

        let mut is_dir = false;
        if let Some(Type::Directory) = entry.file_type() {
            let entry_st_dev =
                match fstatat(dir_raw_fd, entry.file_name(), AtFlags::AT_SYMLINK_NOFOLLOW) {
                    Ok(file_stat) => file_stat.st_dev,
                    Err(e) => {
                        eprintln!("stat of {:?} failed: {}", entry, e);
                        continue;
                    }
                };

            if entry_st_dev != dir_st_dev {
                continue;
            }

            match nix::dir::Dir::openat(
                dir_raw_fd,
                entry.file_name(),
                OFlag::O_RDONLY,
                Mode::S_IXUSR,
            ) {
                Ok(dir) => remove_dir_all_by_dir(dir),
                Err(e) => {
                    eprintln!("openat {:?} failed: {}", entry, e);
                }
            };
            is_dir = true;
        }
        if let Err(e) = nix::unistd::unlinkat(
            Some(dir_raw_fd),
            entry.file_name(),
            match is_dir {
                true => unistd::UnlinkatFlags::RemoveDir,
                false => unistd::UnlinkatFlags::NoRemoveDir,
            },
        ) {
            eprintln!("unlinkat {:?} failed: {}", entry, e);
        }
    }
}

fn root_is_vaild<S: AsRef<OsStr> + ?Sized>(s: &S) -> bool {
    Path::new(s).exists()
}

fn mount_move(umount: &str, old_root: &str, new_root: &str) {
    let old_mount_path = old_root.to_string() + umount;
    if same_device_number(&old_mount_path, old_root) {
        eprintln!("same_device_number: {} , {}", old_mount_path, old_root);
        return;
    }

    let new_mount_path = new_root.to_string() + umount;
    if is_mounted(&new_mount_path, new_root) {
        eprintln!("is_mounted: {} , {}", new_mount_path, new_root);
        return;
    }

    mount(&old_mount_path, &new_mount_path);
}

fn in_initrd(path: &str) -> bool {
    let is_tmpfs = statfs::statfs(path).map_or(false, |s| {
        s.filesystem_type() == FsType(libc::TMPFS_MAGIC as FsTypeT)
    });

    let has_initrd_release = Path::new("/etc/initrd-release").exists();

    is_tmpfs && has_initrd_release
}

fn same_device_number(str_path_l: &str, str_path_r: &str) -> bool {
    let l = if let Ok(meta) = fs::metadata(str_path_l) {
        meta.st_dev()
    } else {
        return false;
    };

    let r = if let Ok(meta) = fs::metadata(str_path_r) {
        meta.st_dev()
    } else {
        return false;
    };

    l == r
}

fn is_mounted(new_mount_path: &str, new_root: &str) -> bool {
    let path = Path::new(&new_mount_path);
    if !path.exists() {
        if let Err(e) = fs::create_dir_all(new_mount_path) {
            eprintln!("create_dir {} failed:{}", new_mount_path, e);
            return true;
        }
        return false;
    }

    let mount_path_st_dev = match path.metadata() {
        Ok(meta) => meta.st_dev(),
        Err(e) => {
            eprintln!("get mount_path metadata failed: {}", e);
            return false;
        }
    };

    let new_root_st_dev = match fs::metadata(new_root) {
        Ok(meta) => meta.st_dev(),
        Err(e) => {
            eprintln!("get new_root metadata failed: {}", e);
            return false;
        }
    };

    if mount_path_st_dev != new_root_st_dev {
        if let Err(e) = mount::umount2(new_mount_path, MntFlags::MNT_DETACH) {
            eprintln!("umount2 {} failed:{}", new_mount_path, e);
        }
        return true;
    }
    false
}

fn mount(source: &str, target: &str) {
    if let Err(e) = mount::mount(
        Some::<&str>(source),
        target,
        None::<&str>,
        MsFlags::MS_MOVE,
        None::<&str>,
    ) {
        eprintln!("failed to mount moving {} to {}: {}", source, target, e);
        eprintln!("forcing unmount of {}", source);
        if let Err(e) = mount::umount2(source, MntFlags::MNT_FORCE) {
            eprintln!("umount2 force failed: {}", e);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::process::Command;

    fn switch_root_pre() -> (String, String, String) {
        let new_root = "/tmp/root/new_root/".to_string();
        let old_root = "/tmp/root/".to_string();
        let mount_move_path = "loopdev".to_string();
        let mount_move_path_all = old_root.clone() + &mount_move_path;
        let mount_move_target = new_root.clone() + &mount_move_path;
        let unlinked_file = old_root.clone() + "/unlinked_file";
        let moved_file = mount_move_path_all.clone() + "/moved_file";
        Command::new("mkdir").arg(&old_root).output().unwrap();
        Command::new("mount")
            .arg("--make-private")
            .arg("tmpfs")
            .arg(&old_root)
            .arg("-t")
            .arg("tmpfs")
            .arg("-o")
            .arg("size=1M")
            .output()
            .unwrap();
        Command::new("mkdir")
            .arg(&new_root)
            .arg(&mount_move_path_all)
            .arg(&mount_move_target)
            .output()
            .unwrap();
        Command::new("mount")
            .arg("--make-private")
            .arg("tmpfs")
            .arg(&new_root)
            .arg("-t")
            .arg("tmpfs")
            .arg("-o")
            .arg("size=1M")
            .output()
            .unwrap();
        Command::new("mount")
            .arg("--make-private")
            .arg("tmpfs")
            .arg(&mount_move_path_all)
            .arg("-t")
            .arg("tmpfs")
            .arg("-o")
            .arg("size=1M")
            .output()
            .unwrap();
        Command::new("touch").arg(&unlinked_file).output().unwrap();
        Command::new("touch").arg(&moved_file).output().unwrap();
        (new_root, old_root, mount_move_path)
    }

    fn switch_root_post(old_root: &str, mount_move_path: &str) {
        Command::new("umount")
            .arg(mount_move_path)
            .output()
            .unwrap_or_else(|_| panic!("umount {mount_move_path} failed"));
        Command::new("umount")
            .arg(old_root)
            .output()
            .unwrap_or_else(|_| panic!("umount {old_root} failed"));
        Command::new("umount")
            .arg(old_root)
            .output()
            .unwrap_or_else(|_| panic!("umount {old_root} failed"));
    }

    #[test]
    fn switch_root_test() {
        if !nix::unistd::getuid().is_root() {
            println!("Unprivileged users cannot mount move, skipping.");
            return;
        }

        let data = switch_root_pre();
        let new_root = data.0.as_str();
        let old_root = data.1.as_str();

        mount_move(&data.2, old_root, new_root);

        let old_root_dir = match nix::dir::Dir::open(old_root, OFlag::O_RDONLY, Mode::S_IXUSR) {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("open {} failed: {}", old_root, e);
                return;
            }
        };

        if let Err(e) = mount::mount(
            Some::<&str>(new_root),
            old_root,
            None::<&str>,
            MsFlags::MS_MOVE,
            None::<&str>,
        ) {
            eprintln!("failed to mount moving {} 2 {}: {}", new_root, old_root, e);
            panic!();
        }

        remove_dir_all_by_dir(old_root_dir);
        switch_root_post(old_root, &format!("{old_root}/{}", data.2.as_str()));
    }
}
