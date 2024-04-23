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

use basic::{machine, mount::MountInfoParser, IN_SET};
use nix::{
    dir::Type,
    errno::Errno,
    fcntl::{self, AtFlags, OFlag},
    mount::{self, MntFlags, MsFlags},
    sys::stat::{self, fstatat, Mode},
    unistd::{self, fchownat, FchownatFlags},
};
use std::{
    ffi::{CString, OsStr},
    fs::{self, File},
    io::Read,
    os::{linux::fs::MetadataExt, unix::prelude::AsRawFd},
    path::Path,
};

const MODE777: Mode = Mode::S_IRWXU.union(Mode::S_IRWXG).union(Mode::S_IRWXO);
const MODE755: Mode = Mode::S_IRWXU
    .union(Mode::S_IRGRP)
    .union(Mode::S_IRWXO)
    .union(Mode::S_IROTH)
    .union(Mode::S_IXOTH);
const MODE750: Mode = Mode::S_IRWXU.union(Mode::S_IRGRP).union(Mode::S_IXGRP);

pub fn switch_root(new_root: &str) -> bool {
    let old_root = "/";

    if old_root == new_root {
        return false;
    }

    if !root_is_valid(old_root) {
        eprintln!("cannot access /");
        return false;
    }

    if !root_is_valid(&new_root) {
        eprintln!("cannot access new_root:{}", new_root);
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
        None::<&str>,
        old_root,
        None::<&str>,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        None::<&str>,
    ) {
        log::error!("Failed to set \"/\" mount propagation to private: {}", e);
    }

    let mounts_path = ["/dev", "/proc", "/sys", "/run"];
    for path in mounts_path {
        mount_move(path, old_root, new_root);
    }

    base_filesystem_create(
        new_root,
        unistd::Uid::from_raw(0xFFFFFFFF),
        unistd::Gid::from_raw(0xFFFFFFFF),
    );

    if let Err(e) = unistd::chdir(new_root) {
        eprintln!("Failed to change directory to {}: {}", &new_root, e);
        return false;
    }

    let old_root_after = "/mnt";
    let put_old = new_root.to_string() + old_root_after;
    if !Path::new(&put_old).exists() {
        if let Err(e) = fs::create_dir_all(&put_old) {
            log::error!("Failed to crearte {} dir: {}", put_old, e);
        }
    }

    if unistd::pivot_root(new_root, put_old.as_str()).is_ok() {
        umount_recursive(old_root_after, MntFlags::MNT_DETACH);
    } else if let Err(e) = mount::mount(
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

    if machine::Machine::in_initrd(None) {
        remove_dir_all_by_dir(old_root_dir);
    }
    true
}

fn umount_recursive(prefix: &str, mnt_flags: MntFlags) {
    let mut mount_data = String::new();
    if let Err(err) = File::open("/proc/self/mountinfo")
        .unwrap()
        .read_to_string(&mut mount_data)
    {
        eprintln!("Failed to read /proc/self/mountinfo: {}", err);
        return;
    }

    let parser = MountInfoParser::new(mount_data);
    for mount in parser {
        let mount_path = mount.mount_point.as_str();
        if !mount_path.starts_with(prefix) {
            continue;
        }
        if let Err(e) = nix::mount::umount2(mount_path, mnt_flags | MntFlags::UMOUNT_NOFOLLOW) {
            println!("Failed to umount {}, ignoring:{}", mount_path, e);
        }
    }
}

macro_rules! LIB_ARCH_TUPLE {
    () => {
        if cfg!(target_arch = "x86_64") {
            BaseFilesystem::new(
                "lib64",
                MODE755,
                vec!["usr/sbin/x86_64-linux-gnu", "usr/lib64"],
                "ld-linux-x86-64.so.2",
                false,
            )
        } else if cfg!(target_arch = "aarch64") {
            BaseFilesystem::new(
                "lib64",
                MODE777,
                vec!["usr/sbin/x86_64-linux-gnu", "usr/lib64"],
                "ld-linux-aarch64.so.1",
                false,
            )
        } else if cfg!(target_arch = "riscv64") {
            BaseFilesystem::new(
                "lib64",
                MODE777,
                vec!["usr/sbin/riscv64-linux-gnu", "usr/lib64/lp64d"],
                "ld-linux-riscv64-lp64d.so.1",
                false,
            )
        } else {
            BaseFilesystem::new(
                "lib64",
                MODE777,
                vec!["usr/sbin/x86_64-linux-gnu", "usr/lib64"],
                "ld-linux-aarch64.so.1",
                false,
            )
        }
    };
}

struct BaseFilesystem<'a> {
    dir: &'a str,
    mode: Mode,
    target: Vec<&'a str>,
    exists: &'a str,
    ignore_failure: bool,
}

impl BaseFilesystem<'_> {
    const fn new<'a>(
        dir: &'a str,
        mode: Mode,
        target: Vec<&'a str>,
        exists: &'a str,
        ignore_failure: bool,
    ) -> BaseFilesystem<'a> {
        BaseFilesystem {
            dir,
            mode,
            target,
            exists,
            ignore_failure,
        }
    }
}

fn is_valid_uid(uid: unistd::Uid) -> bool {
    if uid == unistd::Uid::from_raw(0xFFFFFFFF) {
        return false;
    }

    if uid == unistd::Uid::from_raw(0xFFFF) {
        return false;
    }

    true
}

fn is_valid_gid(gid: unistd::Gid) -> bool {
    is_valid_uid(unistd::Uid::from_raw(gid.as_raw()))
}

fn base_filesystem_create(new_root: &str, uid: unistd::Uid, gid: unistd::Gid) {
    let table: [BaseFilesystem; 11] = [
        BaseFilesystem::new("root", MODE750, vec![], "", false),
        BaseFilesystem::new("usr", MODE755, vec![], "", false),
        BaseFilesystem::new("var", MODE755, vec![], "", false),
        BaseFilesystem::new("etc", MODE755, vec![], "", false),
        BaseFilesystem::new("proc", MODE755, vec![], "", true),
        BaseFilesystem::new("sys", MODE755, vec![], "", true),
        BaseFilesystem::new("dev", MODE755, vec![], "", true),
        BaseFilesystem::new("bin", MODE777, vec!["usr/bin"], "", false),
        BaseFilesystem::new("lib", MODE777, vec!["usr/lib"], "", true),
        BaseFilesystem::new("sbin", MODE777, vec!["usr/sbin"], "", false),
        LIB_ARCH_TUPLE!(),
    ];

    let new_root_dirfd = match fcntl::open(
        new_root,
        OFlag::O_RDONLY
            | OFlag::O_NONBLOCK
            | OFlag::O_DIRECTORY
            | OFlag::O_CLOEXEC
            | OFlag::O_NOFOLLOW,
        stat::Mode::empty(),
    ) {
        Err(e) => {
            eprintln!("Failed to open root file system: {}", e);
            return;
        }
        Ok(fd) => fd,
    };

    for base_fs in table.iter() {
        let path = new_root.to_string() + base_fs.dir;
        if stat::lstat(path.as_str()).is_ok() {
            continue;
        }

        if !base_fs.target.is_empty() {
            let mut link_to = "";
            for target in base_fs.target.iter() {
                let path_target = new_root.to_string() + "/" + *target;
                if stat::lstat(path_target.as_str()).is_err() {
                    continue;
                }

                if !base_fs.exists.is_empty()
                    && stat::lstat((path_target + "/" + base_fs.exists).as_str()).is_err()
                {
                    continue;
                }
                link_to = *target;
                break;
            }

            if link_to.is_empty() {
                continue;
            }

            if let Err(err) = nix::unistd::symlinkat(link_to, Some(new_root_dirfd), base_fs.dir) {
                eprintln!(
                    "Failed to create symlink at: {}/{}: {}",
                    new_root, base_fs.dir, err
                );
                if IN_SET!(err, Errno::EEXIST, Errno::EROFS) || base_fs.ignore_failure {
                    continue;
                }
                return;
            }

            if is_valid_uid(uid) || is_valid_gid(gid) {
                if let Err(err) = fchownat(
                    Some(new_root_dirfd),
                    base_fs.dir,
                    Some(uid),
                    Some(gid),
                    FchownatFlags::NoFollowSymlink,
                ) {
                    eprintln!(
                        "Failed to chown symlink at {}/{}: {}",
                        new_root, base_fs.dir, err
                    );
                    return;
                }
            }
            continue;
        }

        if let Err(err) = nix::sys::stat::mkdirat(new_root_dirfd, base_fs.dir, base_fs.mode) {
            eprintln!(
                "Failed to create directory at: {}/{}: {}",
                new_root, base_fs.dir, err
            );
            if IN_SET!(err, Errno::EEXIST, Errno::EROFS) || base_fs.ignore_failure {
                continue;
            }
            return;
        }

        if is_valid_uid(uid) || is_valid_gid(gid) {
            if let Err(err) = fchownat(
                Some(new_root_dirfd),
                base_fs.dir,
                Some(uid),
                Some(gid),
                FchownatFlags::NoFollowSymlink,
            ) {
                eprintln!(
                    "Failed to chown directory at: {}/{}: {}",
                    new_root, base_fs.dir, err
                );
                return;
            }
        }
    }
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

fn root_is_valid<S: AsRef<OsStr> + ?Sized>(s: &S) -> bool {
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
            .unwrap_or_else(|_| panic!("umount {} failed", mount_move_path));
        Command::new("umount")
            .arg(old_root)
            .output()
            .unwrap_or_else(|_| panic!("umount {} failed", old_root));
        Command::new("umount")
            .arg(old_root)
            .output()
            .unwrap_or_else(|_| panic!("umount {} failed", old_root));
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
        switch_root_post(old_root, &format!("{}/{}", old_root, data.2.as_str()));
    }

    #[test]
    fn test_base_filesystem_create() {
        Command::new("mkdir")
            .arg("/tmp/new_root")
            .arg("/tmp/new_root/usr")
            .arg("/tmp/new_root/usr/lib")
            .arg("/tmp/new_root/usr/bin")
            .arg("/tmp/new_root/usr/sbin")
            .output()
            .unwrap();

        base_filesystem_create(
            "/tmp/new_root",
            unistd::Uid::from_raw(0xFFFFFFFF),
            unistd::Gid::from_raw(0xFFFFFFFF),
        );

        let dir = vec![
            "var", "sys", "root", "proc", "etc", "dev", "sbin", "lib", "bin",
        ];
        for dir_ref in dir {
            assert!(Path::new(("/tmp/new_root/".to_string() + dir_ref).as_str()).exists());
        }

        Command::new("rm")
            .arg("-rf")
            .arg("/tmp/new_root")
            .output()
            .unwrap();
    }
}
