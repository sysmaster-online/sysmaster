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

use basic::io::loop_read;
use nix::ioctl_write_ptr;
use std::alloc::{alloc, dealloc, Layout};
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::{env, mem, str};
use std::{
    fs::{self, read_link, File},
    io::{self, Seek, Write},
    os::unix::prelude::AsRawFd,
};

const RAMDOM_POOL_SIZE_MIN: usize = 512;
const RANDOM_SEED: &str = "/usr/lib/sysmaster/random-seed";
const RAMDOM_SEED_DIR: &str = "/usr/lib/sysmaster";
const RANDOM_POOL_SIZE_MAX: usize = 10 * 1024 * 1024;

fn read_one_line_file(path: &str) -> Result<String, ()> {
    let str = fs::read_to_string(path).unwrap_or_else(|_| String::new());

    if str.is_empty() {
        log::error!("Failed to read pool size from kernel.");
        return Result::Err(());
    }

    for line in str.lines() {
        if line.trim().is_empty() {
            continue;
        } else {
            return Result::Ok(String::from(line));
        }
    }
    Result::Err(())
}

fn random_pool_size() -> usize {
    match read_one_line_file("/proc/sys/kernel/random/poolsize") {
        Err(_) => RAMDOM_POOL_SIZE_MIN,
        Ok(str) => match str.parse::<usize>() {
            Ok(size) => {
                let mut size = size / 8;
                size = size.clamp(RAMDOM_POOL_SIZE_MIN, RANDOM_POOL_SIZE_MAX);
                size
            }
            Err(_) => RAMDOM_POOL_SIZE_MIN,
        },
    }
}

pub fn get_random(data: &mut [u8], flags: u32) -> Result<usize, ()> {
    let size;
    unsafe {
        size = libc::getrandom(data.as_mut_ptr() as *mut libc::c_void, data.len(), flags);
    }

    if size <= 0 {
        return Err(());
    }

    Result::Ok(size as usize)
}

fn chmod_and_chown(file: &mut File) -> bool {
    let file_path =
        read_link(PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()))).unwrap();
    let cstr = CString::new(file_path.to_str().unwrap()).unwrap();

    unsafe {
        let mut r = libc::chmod(cstr.as_ptr(), 0o600);
        if r != 0 {
            return false;
        }

        r = libc::chown(cstr.as_ptr(), 0, 0);
        println!("{}", r);
        r == 0
    }
}

const ENTROPY_IOCTL_BASE: u8 = b'R';
const ENTROPY_SETOPTIONS: u8 = 0x03;
#[repr(C)]
pub struct RandPoolInfo {
    entropy_count: i32,
    buf_size: i32,
    buf: [u8; 0],
}

ioctl_write_ptr!(
    rndaddentropy,
    ENTROPY_IOCTL_BASE,
    ENTROPY_SETOPTIONS,
    RandPoolInfo
);
fn random_write_entropy(random_fd: &mut File, data: &mut [u8], credit: bool) -> bool {
    assert!(!data.is_empty());

    if data.is_empty() {
        return false;
    }
    if credit {
        unsafe {
            let info_size = mem::size_of::<RandPoolInfo>();
            let layout =
                Layout::from_size_align(data.len() + info_size, mem::align_of::<u8>()).unwrap();
            let ptr = alloc(layout);
            let r_p_info = ptr as *mut RandPoolInfo;
            (*r_p_info).entropy_count = (data.len() * 8) as i32;
            (*r_p_info).buf_size = data.len() as i32;
            // *(ptr as *mut i32) = (data.len() * 8) as i32;
            // *(ptr.add(4) as *mut i32) = data.len() as i32;
            let ptr_data = ptr.add(info_size);
            for (index, value) in data.iter().enumerate() {
                *ptr_data.add(index) = *value;
            }
            let result = match rndaddentropy(random_fd.as_raw_fd(), ptr as *const RandPoolInfo) {
                Ok(_) => true,
                Err(err) => {
                    println!("{}", err);
                    false
                }
            };
            dealloc(ptr, layout);
            result
        }
    } else {
        loop_write(random_fd, data)
    }
}

fn fsync_full(file: &mut File) -> bool {
    let fd = file.as_raw_fd();
    assert_ne!(fd, 0);

    unsafe {
        let r = libc::fsync(fd);
        if r < 0 {
            return false;
        }
    }

    if !(file.metadata().unwrap().file_type().is_file()
        || file.metadata().unwrap().file_type().is_dir())
    {
        return false;
    }

    let file_path = read_link(PathBuf::from(format!("/proc/self/fd/{}", fd))).unwrap();

    unsafe {
        let r = libc::fsync(
            fs::OpenOptions::new()
                .read(true)
                .open(file_path.parent().unwrap())
                .unwrap()
                .as_raw_fd(),
        );
        if r < 0 {
            return false;
        }
    }
    true
}

fn sd_id128_from_string(buf: &[u8]) -> Result<[u8; 16], ()> {
    let mut bytes = [0; 16];

    let mut i = 0;
    let mut is_guid = false;
    let mut n = 0;
    while n < 16 {
        if buf[i] == b'-' {
            if i == 8 {
                is_guid = true;
            } else if i == 13 || i == 18 || i == 23 {
                if !is_guid {
                    return Err(());
                }
            } else {
                return Err(());
            }
            i += 1;
            continue;
        }

        let a = match unhexchar(buf[i]) {
            Ok(a) => a,
            Err(_) => return Err(()),
        };
        i += 1;

        let b = match unhexchar(buf[i]) {
            Ok(a) => a,
            Err(_) => return Err(()),
        };
        i += 1;

        bytes[n] = a << 4 | b;
        n += 1;
    }

    if is_guid && i != 36 {
        return Err(());
    }

    if !is_guid && i != 32 {
        return Err(());
    }

    Ok(bytes)
}

fn unhexchar(c: u8) -> Result<u8, ()> {
    if c.is_ascii_digit() {
        return Ok(c - b'0');
    } else if c.is_ascii_hexdigit() && c.is_ascii_lowercase() {
        return Ok(c - b'a' + 10);
    } else if c.is_ascii_hexdigit() && c.is_ascii_uppercase() {
        return Ok(c - b'A' + 10);
    }

    Err(())
}

fn sd_id128_get_machine() -> Result<[u8; 16], ()> {
    let mut file = match fs::OpenOptions::new().read(true).open("/etc/machine-id") {
        Ok(file) => file,
        Err(_) => return Err(()),
    };

    let mut buf = [0; 38];
    let size = match loop_read(&mut file, &mut buf) {
        Err(_) => return Err(()),
        Ok(size) => size,
    };

    let size = match size {
        12 | 13 => return Err(()),
        33 => {
            if buf[32] != b'\n' {
                return Err(());
            }
            buf[32] = 0;
            32
        }
        32 => {
            buf[32] = 0;
            32
        }
        37 => {
            if buf[36] != b'\n' {
                return Err(());
            }
            buf[36] = 0;
            36
        }
        36 => {
            buf[36] = 0;
            36
        }
        _ => return Err(()),
    };

    sd_id128_from_string(&buf[..size])
}

fn loop_write(file: &mut File, buf: &[u8]) -> bool {
    let mut write_size = 0;

    while write_size < buf.len() {
        write_size += match file.write(&buf[write_size..]) {
            Ok(size) => size,
            Err(err) => {
                println!("write err: {}", err);
                return false;
            }
        };
    }
    true
}
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum CreditEntropy {
    NoWay,
    YesPlease,
    YesForced,
}

fn may_credit(file: &mut File) -> CreditEntropy {
    let e = match env::var("SYSTEMD_RANDOM_SEED_CREDIT") {
        Ok(str) => str,
        Err(_) => {
            log::error!("$SYSTEMD_RANDOM_SEED_CREDIT is not set, not crediting entropy");
            return CreditEntropy::NoWay;
        }
    };

    if e.eq("force") {
        log::debug!("$SYSTEMD_RANDOM_SEED_CREDIT is set to 'force', crediting entropy");
        return CreditEntropy::YesForced;
    }

    let bool_var = if let Ok(var) = parse_boolean(&e) {
        var
    } else {
        log::error!("Failed to parse $SYSTEMD_RANDOM_SEED_CREDIT, not crediting entropy");
        return CreditEntropy::NoWay;
    };

    if !bool_var {
        log::debug!(
            "Crediting entropy is turned off via $SYSTEMD_RANDOM_SEED_CREDIT,not crediting entropy"
        );
        return CreditEntropy::NoWay;
    }

    let mut str = String::new();
    match getxattr(file, &mut str) {
        Err(()) => {
            log::error!("Failed to read extended attribute, ignore.");
            return CreditEntropy::NoWay;
        }
        Ok(false) => {
            log::error!("Seed file is not marked as credittable, not crediting.");
            return CreditEntropy::NoWay;
        }
        Ok(true) => {}
    };

    match parse_boolean(&str) {
        Err(()) => {
            log::error!(
                "Failed  to parse user,random-seed-creditable extended attribute, ignoring: {}",
                str
            );
            return CreditEntropy::NoWay;
        }
        Ok(false) => {
            log::error!("Seed file is not marked as credittable, not crediting.");
            return CreditEntropy::NoWay;
        }
        Ok(true) => {}
    };

    if Path::new("/run/systemd/first-boot").exists() {
        log::debug!("Not crediting entropy, since booted in first-boot mode.");
        return CreditEntropy::NoWay;
    }

    CreditEntropy::YesPlease
}

fn get_file_path(file: &File) -> io::Result<PathBuf> {
    read_link(PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd())))
}

fn setxattr(file: &File) -> bool {
    let path = match get_file_path(file) {
        Ok(path) => path,
        Err(_) => {
            return false;
        }
    };

    match xattr::set(path, "user.random-seed-credittable", "1".as_bytes()) {
        Ok(()) => true,
        Err(_) => false,
    }
}

fn getxattr(file: &mut File, str: &mut String) -> Result<bool, ()> {
    let path = match get_file_path(file) {
        Ok(path) => path,
        Err(_) => return Err(()),
    };

    let vec = match xattr::get(path, "user.random-seed-credittable") {
        Ok(Some(vec)) => vec,
        Err(err) => {
            println!("{}", err);
            return Err(());
        }
        Ok(None) => return Ok(false),
    };

    str.push_str(str::from_utf8(&vec[..]).unwrap());
    Ok(true)
}

fn parse_boolean(str: &str) -> Result<bool, ()> {
    match str {
        "1" | "yes" | "y" | "true" | "t" | "on" => Ok(true),
        "0" | "no" | "n" | "false" | "f" | "off" => Ok(false),
        _ => Err(()),
    }
}

pub fn run(arg: &str) -> Result<(), String> {
    let mut buf_size = random_pool_size();

    if !Path::new(RAMDOM_SEED_DIR).exists() {
        if let Err(err) = fs::create_dir_all(RAMDOM_SEED_DIR) {
            return Err(format!("Failed to create directory:{}", err));
        }
    }

    let mut seed_fd = match fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(RANDOM_SEED)
    {
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                return Ok(());
            }
            return Err(format!("open random-seed failed: {}", err));
        }
        Ok(file) => file,
    };

    let mut random_fd = match fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/urandom")
    {
        Err(err) => {
            return Err(format!("Failed to open /dev/urandom, err:{}", err));
        }
        Ok(file) => file,
    };

    let size = seed_fd.metadata().unwrap().len();
    // let mut synchronous: bool = false;
    let write_seed_file: bool;
    let read_seed_file: bool;
    match arg {
        "load" => {
            // synchronous = true;
            read_seed_file = true;
            write_seed_file = false;
            if size == 0 {
                return Ok(());
            }
        }
        "save" => {
            read_seed_file = false;
            write_seed_file = true;
            // synchronous = false;
        }
        _ => {
            return Err("Unknown verb".to_string());
        }
    }

    if size > buf_size as u64 {
        let buf_size_u64 = std::cmp::min(size, RANDOM_POOL_SIZE_MAX as u64);
        buf_size = buf_size_u64 as usize;
    };

    let mut buf = vec![0u8; buf_size];

    if read_seed_file {
        if let Ok(buf) = sd_id128_get_machine() {
            if !loop_write(&mut random_fd, &buf) {
                log::debug!("Failed to write machine ID to /dev/urandom.");
            }
        } else {
            log::debug!("Failed to get machine ID.");
        }

        let size_seed = match loop_read(&mut seed_fd, &mut buf) {
            Ok(size) => size,
            Err(_) => {
                return Err(format!("Failed to read seed from {}", RANDOM_SEED));
            }
        };

        if size_seed == 0 {
            log::debug!(
                "Seed file \"{} \" not yet initialized, proceeding.",
                RANDOM_SEED
            );
        }

        if let Err(err) = seed_fd.rewind() {
            return Err(format!("Failed to rewind start: {}", err));
        }

        let mut lets_credit = may_credit(&mut seed_fd);

        match xattr::remove(RANDOM_SEED, "user.random-seed-credittable") {
            Ok(()) => {
                if !fsync_full(&mut seed_fd) {
                    log::error!("Failed to synchronize seed to disk, not crediting entropy");
                    if lets_credit == CreditEntropy::YesPlease {
                        lets_credit = CreditEntropy::NoWay;
                    }
                }
            }
            Err(err) => {
                log::error!("Failed to remove extended attribute, ignoring: {}", err);
            }
        };

        let is_credit =
            lets_credit == CreditEntropy::YesForced || lets_credit == CreditEntropy::YesPlease;
        // let mut vec = buf.to_vec();
        // vec.resize(size_seed, 0);
        if !random_write_entropy(&mut random_fd, &mut buf, is_credit) {
            return Err("Failed to write seed to /dev/urandom.".to_string());
        }
    }

    if write_seed_file {
        let mut getrandom_worked = false;
        if !chmod_and_chown(&mut seed_fd) {
            return Err("Failed to adjust seed file ownership and access mode.".to_string());
        }

        let mut get_size = match get_random(&mut buf, 0x0001) {
            Err(_) => {
                log::error!(
                    "Failed to read random data with getrandom(), falling back to /dev/random."
                );
                0
            }
            Ok(size) => {
                if size < buf_size {
                    log::debug!("Short read from getrandom(), falling back to /dev/urandom.");
                } else {
                    getrandom_worked = true;
                }
                size
            }
        };

        // let mut read_size = 0;
        if !getrandom_worked {
            get_size = match loop_read(&mut random_fd, &mut buf) {
                Ok(size) => {
                    if size == 0 {
                        log::error!("Failed to read new seed from /dev/urandom.");
                    }
                    size
                }
                Err(_) => {
                    return Err("Got EOF while reading from /dev/urandom.".to_string());
                }
            }
        }

        if !loop_write(&mut seed_fd, &buf[..get_size]) {
            return Err("Failed to write new random seed file.".to_string());
        }

        unsafe {
            if libc::ftruncate(seed_fd.as_raw_fd(), get_size as libc::off_t) < 0 {
                return Err("Failed to truncate random seed file.".to_string());
            }
        }

        if !fsync_full(&mut seed_fd) {
            return Err("Failed to synchronize seed file.".to_string());
        }

        if getrandom_worked && !setxattr(&seed_fd) {
            log::error!("setxattr err");
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    fn is_root() -> bool {
        let uid = unsafe { libc::geteuid() };
        uid == 0
    }
    #[test]
    fn sd_id128_from_string_test() {
        let buf = b"e57446f87c3f4f978a7eca30ff7197d3";
        assert_eq!(
            [229, 116, 70, 248, 124, 63, 79, 151, 138, 126, 202, 48, 255, 113, 151, 211],
            sd_id128_from_string(buf).unwrap()
        );
        let buf1 = b"E57446f8-7c3f-4f97-8a7e-ca30ff7197d3";
        assert_eq!(
            [229, 116, 70, 248, 124, 63, 79, 151, 138, 126, 202, 48, 255, 113, 151, 211],
            sd_id128_from_string(buf1).unwrap()
        );
        let buf2 = b"e57446f8+7c3f-4f97-8a7e-ca30ff7197d3";
        assert_eq!(Err(()), sd_id128_from_string(buf2));
    }

    #[test]
    fn sd_id128_get_machine_test() {
        assert_ne!(Err(()), sd_id128_get_machine());
    }

    #[test]
    fn loop_write_test() {
        let buf = b"abcdefg";
        let mut file = fs::File::create("loop_write_test.txt").unwrap();
        assert!(loop_write(&mut file, buf));
        fs::remove_file("loop_write_test.txt").unwrap();
    }

    #[test]
    fn fsync_full_test() {
        let mut file = fs::File::create("fsync_full_test.txt").unwrap();
        assert!(fsync_full(&mut file));
        fs::remove_file("fsync_full_test.txt").unwrap();
    }

    #[test]
    fn random_pool_size_test() {
        assert!(random_pool_size() >= RAMDOM_POOL_SIZE_MIN);
        assert!(random_pool_size() <= RANDOM_POOL_SIZE_MAX);
        assert_eq!(read_one_line_file(""), Err(()));
    }

    #[test]
    fn may_credit_test() {
        let mut seed_fd = fs::File::create("seed_fd.txt").unwrap();
        xattr::set(
            "seed_fd.txt",
            "user.random-seed-credittable",
            "1".as_bytes(),
        )
        .unwrap();
        env::set_var("SYSTEMD_RANDOM_SEED_CREDIT", "true");
        assert_eq!(CreditEntropy::YesPlease, may_credit(&mut seed_fd));
        fs::remove_file("seed_fd.txt").unwrap();
    }

    #[test]
    fn random_write_entropy_test() {
        let mut data = vec![0u8; 512];
        let get_size = get_random(&mut data, 0x0001).unwrap();
        let mut writer = fs::File::create("writer.txt").unwrap();
        let mut vecdata = data[..get_size].to_vec();
        assert!(random_write_entropy(&mut writer, &mut vecdata, false));
        let mut random_fd = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/urandom")
            .unwrap();
        if is_root() {
            assert!(random_write_entropy(&mut random_fd, &mut vecdata, true));
        }
        chmod_and_chown(&mut writer);
        fs::remove_file("writer.txt").unwrap();
    }

    #[test]
    fn get_random_test() {
        let mut data1 = vec![0u8; 4096];
        let get_size = get_random(&mut data1, 0x0001).unwrap();
        println!("get_random_size:{}", get_size);
        println!("get_random_size:{:?}", data1);
        assert!(get_size >= 512);
        let mut file = fs::File::create("random-seed").unwrap();
        assert!(loop_write(&mut file, &data1));
        fs::remove_file("random-seed").unwrap();
        // assert_eq!(data1, data.to_vec());
    }

    #[test]
    fn xattr_test() {
        let mut file = fs::File::create("xattr_test.txt").unwrap();
        assert!(setxattr(&file));
        let mut str = String::new();
        assert!(getxattr(&mut file, &mut str).unwrap());
        assert_eq!(str, "1");
        xattr::remove("xattr_test.txt", "user.random-seed-credittable").unwrap();
        assert!(!getxattr(&mut file, &mut str).unwrap());
        fs::remove_file("xattr_test.txt").unwrap();
    }

    #[test]
    fn run_test() {
        if is_root() {
            assert_eq!(run("load"), Ok(()));
            assert_eq!(run("save"), Ok(()));
        }
    }
}
