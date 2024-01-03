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

//!

use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::result;
use std::{env, process};

static mut ARG_DEST: String = String::new();
const TTY_INFO: &str = "/sys/class/tty/console/active";

fn main() {
    log::init_log_to_console_syslog("getty-generator", log::Level::Info);
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 && args.len() != 4 {
        log::error!("{}", "This program takes one or three arguments.");
        process::exit(1);
    }

    unsafe {
        ARG_DEST.push_str(&args[1]);
    }

    if detect_container() {
        log::debug!("detect_container: true");
        match run_container() {
            Ok(_) => return,
            Err(err) => process::exit(err.raw_os_error().unwrap_or(-1)),
        }
    }

    if let Err(err) = deal_all_active_kernel_consoles(TTY_INFO) {
        process::exit(err.raw_os_error().unwrap_or(-1));
    }

    if let Err(err) = add_serial_getty_first(vec![
        "hvc0",
        "xvc0",
        "hvsi0",
        "sclp_line0",
        "ttysclp0",
        "3270!tty1",
    ]) {
        process::exit(err.raw_os_error().unwrap_or(-1));
    }

    if let Err(err) = add_vt_getty() {
        process::exit(err.raw_os_error().unwrap_or(-1));
    }
}

fn add_vt_getty() -> io::Result<()> {
    for i in 1..7 {
        if let Err(err) = create_getty("", &format!("tty{}", i)) {
            return Err(err);
        }
    }
    Ok(())
}

fn run_container() -> io::Result<()> {
    log::debug!("Automatically adding console shell.");

    if let Err(err) = add_symlink(
        &concat_from_symlink("console-getty.service"),
        &concat_to_symlink("console-getty.service"),
    ) {
        return Err(err);
    }

    let str_value = getenv_for_pid(1, "container_ttys").unwrap_or_default();

    for word in extract_words(&str_value) {
        if word.is_empty() {
            continue;
        }

        let mut str_tty = word.to_string();
        if str_tty.starts_with("/dev/") {
            str_tty = str_tty.replace("/dev/", "");
        }

        if str_tty.starts_with("pts/") {
            str_tty = str_tty.replace("pts/", "");
        }

        if str_tty.is_empty() {
            continue;
        }

        if let Err(err) = add_container_getty(word) {
            return Err(err);
        }
    }
    Ok(())
}

fn add_container_getty(tty: &str) -> io::Result<()> {
    log::debug!("Automatically adding container getty for /dev/pts/{}", tty);

    let name = "container-getty@".to_string() + tty + ".service";
    add_symlink(
        &concat_from_symlink("container-getty@.service"),
        &concat_to_symlink(&name),
    )
}

fn getenv_for_pid(pid: libc::pid_t, field: &str) -> result::Result<String, ()> {
    if pid == 0 || pid == unsafe { libc::getpid() } {
        return match env::var(field) {
            Err(_) => Err(()),
            Ok(str) => Ok(str),
        };
    }

    if pid <= 0 {
        return Err(());
    }

    let path = "/proc/".to_string() + &pid.to_string() + "/environ";
    let s = fs::read_to_string(path).unwrap_or_default();
    for str in s.split('\0') {
        if str.is_empty() {
            continue;
        }
        let str_key_value = str.to_string();
        let start = field.to_string() + "=";
        if str_key_value.starts_with(&start) {
            return Ok(str_key_value.replace(&start, ""));
        }
    }
    Err(())
}

fn detect_container() -> bool {
    /* /proc/vz exists in container and outside of the container, /proc/bc only outside of the container. */
    if Path::new("/proc/vz").exists() && !Path::new("/proc/bc").exists() {
        return true;
    }

    /* "Official" way of detecting WSL https://github.com/Microsoft/WSL/issues/423#issuecomment-221627364 */
    let str_line = read_one_line_file("/proc/sys/kernel/osrelease").unwrap_or_default();

    if str_line.is_empty() {
        log::debug!("Failed to read /proc/sys/kernel/osrelease, ignoring.");
    } else if str_line.contains("Microsoft") || str_line.contains("WSL") {
        return true;
    }

    /* proot doesn't use PID namespacing, so we can just check if we have a matching tracer for this
     * invocation without worrying about it being elsewhere.
     */
    let str_tracer_pid = get_proc_field("/proc/self/status", "TracerPid")
        .expect("Failed to read our own trace PID, ignoring.");
    log::debug!("str_tracer_pid:{}", str_tracer_pid);
    if str_tracer_pid.ne("0") {
        let pid: i32 = str_tracer_pid.parse().unwrap_or(-1);

        if pid <= 0 {
            log::debug!("Failed to parse our own tracer PID, ignoring.");
        } else {
            let str_proot = read_one_line_file(&("/proc/".to_string() + &str_tracer_pid + "/comm"))
                .unwrap_or_else(|_| {
                    log::debug!("ailed to read {}, ignoring", str_tracer_pid);
                    String::new()
                });
            if str_proot.starts_with("proot") {
                return true;
            }
        }
    }

    /* The container manager might have placed this in the /run/host/ hierarchy for us, which is best
     * because we can be consumed just like that, without special privileges. */
    let str = read_one_line_file("/run/host/container-manager").unwrap_or_default();
    log::debug!("container-manager:{}", str);
    if translate_name(&str) {
        return true;
    }

    if 1 == unsafe { libc::getpid() } {
        log::debug!("getpid == 1");
        match env::var("container") {
            Ok(str) => {
                return translate_name(&str);
            }
            Err(_) => return check_file(),
        }
    }

    /* Otherwise, PID 1 might have dropped this information into a file in /run. This is better than accessing
     * /proc/1/environ, since we don't need CAP_SYS_PTRACE for that. */
    match read_one_line_file("/run/systemd/container") {
        Ok(str) => translate_name(&str),
        Err(_) => false,
    };
    check_file()
}

fn check_file() -> bool {
    detect_container_files()
}

fn translate_name(name: &str) -> bool {
    if name.eq("oci") && detect_container_files() {
        return true;
    }
    container_from_string(name)
}

fn container_from_string(container_type: &str) -> bool {
    let vec = vec![
        "lxc",
        "lxc-libvirt",
        "systemd-nspawn",
        "docker",
        "podman",
        "rkt",
        "wsl",
        "proot",
        "pouch",
    ];

    for str in vec {
        if str.eq(container_type) {
            return true;
        }
    }
    false
}

fn detect_container_files() -> bool {
    let vec = vec!["/run/.containerenv", "/.dockerenv"];
    for file in vec {
        if Path::new(file).exists() {
            return true;
        }
    }
    false
}

fn get_proc_field(str_path: &str, key: &str) -> io::Result<String> {
    let str_content = fs::read_to_string(str_path)?;

    for line in str_content.lines() {
        if line.starts_with(key) {
            for (index, str) in line.split(':').enumerate() {
                if index == 1 {
                    return Ok(str.trim().to_string());
                }
            }
            break;
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "error"))
}

fn read_one_line_file(str_path: &str) -> io::Result<String> {
    let file = match OpenOptions::new().read(true).open(str_path) {
        Ok(file) => file,
        Err(err) => return Err(err),
    };

    let mut buff_reader = BufReader::new(file);
    let mut buff = String::new();
    let size = buff_reader.read_line(&mut buff).unwrap_or(0);

    if size == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "read_line_err"));
    }
    Ok(buff)
}

fn extract_words(str: &str) -> Vec<&str> {
    str.split(' ').collect()
}

fn tty_is_vc(str: &str) -> bool {
    let mut s = str.to_string();
    if str.find("/dev/") == Some(0) {
        s.drain(..5);
    }

    if s.find("tty") != Some(0) {
        return false;
    }

    s.drain(..3);

    let i: i32 = s.parse().unwrap_or(-1);

    if i < 0 {
        return false;
    }

    if i > 63 {
        return false;
    }

    true
}

fn deal_all_active_kernel_consoles(path: &str) -> io::Result<()> {
    let str = read_one_line_file(path).unwrap_or_else(|_| String::new());

    for word in extract_words(&str) {
        let word = word.trim();
        if word.is_empty() || tty_is_vc(word) {
            continue;
        }

        if !verify_tty(word) {
            continue;
        }

        if let Err(err) = create_getty("serial", word) {
            return Err(err);
        }
    }

    Ok(())
}

fn add_serial_getty_first(vec_name: Vec<&str>) -> io::Result<()> {
    for name in vec_name {
        let file_path = "/sys/class/tty/".to_string() + name;

        if !Path::new(&file_path).exists() {
            continue;
        }

        if !verify_tty(name) {
            continue;
        }

        if let Err(err) = create_getty("serial", name) {
            return Err(err);
        }
    }
    Ok(())
}

fn verify_tty(str_tty: &str) -> bool {
    let s = "/dev/".to_string() + str_tty;

    let file = match OpenOptions::new().read(true).write(true).open(s) {
        Ok(file) => file,
        Err(_) => return false,
    };

    unsafe {
        let i = libc::isatty(file.as_raw_fd());
        if i < 0 {
            return false;
        }
    }

    true
}

fn create_getty(tty_type: &str, tty: &str) -> io::Result<()> {
    if tty.is_empty() {
        return Err(std::io::Error::from(io::ErrorKind::InvalidInput));
    }

    let tty_type = if tty_type.is_empty() {
        tty_type.to_string()
    } else {
        format!("{}-", tty_type)
    };
    let from = format!("{}getty@.service", tty_type);
    let to = format!("{}getty@{}.service", tty_type, tty);
    add_symlink(&concat_from_symlink(&from), &concat_to_symlink(&to))
}

fn concat_from_symlink(file_name: &str) -> String {
    basic::fs::LIB_SYSTEM_PATH.to_string() + "/" + file_name
}

fn concat_to_symlink(file_name: &str) -> String {
    unsafe { ARG_DEST.clone() + "/getty.target.wants/" + file_name }
}

fn add_symlink(from_service: &str, to_where: &str) -> io::Result<()> {
    if from_service.is_empty() || to_where.is_empty() {
        return Err(std::io::Error::from(io::ErrorKind::InvalidInput));
    }

    if let Err(err) = mkdir_parents_lable(to_where) {
        return Err(err);
    }

    let e = std::os::unix::fs::symlink(&from_service, &to_where);
    if let Err(err) = e {
        if err.kind() == io::ErrorKind::AlreadyExists {
            log::debug!("symlink already exists");
            return Ok(());
        }
        log::debug!("Failed to create symlink {}: {}", to_where, err);
        return Err(err);
    }

    Ok(())
}

fn mkdir_parents_lable(path: &str) -> io::Result<()> {
    if path.is_empty() {
        return Ok(());
    }

    let r = path.rfind('/').unwrap_or(0);

    if 0 == r {
        return Ok(());
    }

    let s = &path[..r];
    fs::create_dir_all(s)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tty_is_vc_test() {
        assert!(tty_is_vc("tty0"));
        assert!(tty_is_vc("tty1"));
        assert!(tty_is_vc("tty63"));
        assert!(!tty_is_vc("tty64"));
        assert!(!tty_is_vc("ttyAMA0"));
        assert!(tty_is_vc("/dev/tty0"));
        assert!(tty_is_vc("/dev/tty63"));
        assert!(!tty_is_vc("/dev/tty64"));
    }

    #[test]
    fn getenv_for_pid_test() {
        env::set_var("pid_info", "pid_self");
        assert_eq!(
            getenv_for_pid(1, "pid_info").unwrap_or_else(|_| "none".to_string()),
            "none"
        );

        unsafe {
            let pid = libc::getpid();
            assert_eq!(getenv_for_pid(pid, "pid_info").unwrap(), "pid_self");
        }
    }

    #[test]
    fn add_serial_getty_test() {
        let path = "/tmp/tty_info";
        std::fs::write(path, "ttyAMA0 tty0\n").unwrap();
        unsafe {
            let clone = ARG_DEST.clone();
            ARG_DEST.clear();
            ARG_DEST.push_str("/tmp");
            assert!(deal_all_active_kernel_consoles(path).is_ok());
            assert!(add_vt_getty().is_ok());
            ARG_DEST.clear();
            ARG_DEST.push_str(&clone);
        }
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir_all("/tmp/getty.target.wants").unwrap();
    }
}
