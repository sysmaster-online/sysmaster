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

//! The init daemon
use clap::Parser;
use log::{Level, LevelFilter, Log};
use nix::sys::{
    signal::Signal,
    wait::{waitid, Id, WaitPidFlag, WaitStatus},
};
use once_cell::sync::OnceCell;
use psutil::process::Process;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::UnixListener,
    process::Command,
    signal::unix::{signal, SignalKind},
    sync::RwLock,
    time::sleep,
};

struct InitLog {
    kmsg: std::sync::Mutex<File>,
    maxlevel: LevelFilter,
}

impl InitLog {
    pub fn new(filter: LevelFilter) -> InitLog {
        InitLog {
            kmsg: std::sync::Mutex::new(OpenOptions::new().write(true).open("/dev/kmsg").unwrap()),
            maxlevel: filter,
        }
    }

    pub fn init(filter: LevelFilter) {
        let klog = InitLog::new(filter);
        _ = log::set_boxed_logger(Box::new(klog));
        log::set_max_level(filter);
    }
}

impl Log for InitLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.maxlevel
    }

    fn log(&self, record: &log::Record) {
        if record.level() > self.maxlevel {
            return;
        }

        let level: u8 = match record.level() {
            Level::Error => 3,
            Level::Warn => 4,
            Level::Info => 5,
            Level::Debug => 6,
            Level::Trace => 7,
        };

        let mut buf = Vec::new();
        writeln!(
            buf,
            "<{}>{}[{}]: {}",
            level,
            record.target(),
            process::id(),
            record.args()
        )
        .unwrap();

        if let Ok(mut kmsg) = self.kmsg.lock() {
            let _ = kmsg.write(&buf);
            let _ = kmsg.flush();
        }
    }

    fn flush(&self) {}
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct InitOptions {
    /// Number of monitored and reset instances.
    #[clap(long, value_parser, default_value = "10")]
    timecnt: usize,
    /// Waiting time for monitoring and keeping alive.
    #[clap(long, value_parser, default_value = "90")]
    timewait: u64,
    /// Subcommands for init
    #[clap(long, value_parser, default_value = "/usr/lib/sysmaster/sysmaster")]
    bin: String,
    /// socket path
    #[clap(long, value_parser, default_value = "/run/sysmaster/init.sock")]
    socket: String,
    /// Other options
    args: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InitState {
    Init,
    Running,
    Reexec,
}

struct Runtime {
    options: InitOptions,
    state: InitState,
    // sysmaster pid
    pid: i32,
    debug: bool,
    // sysmaster status
    online: bool,
}

fn runtime() -> &'static RwLock<Runtime> {
    static INSTANCE: OnceCell<RwLock<Runtime>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        #[cfg(not(test))]
        let ret = RwLock::new(Runtime::new().unwrap());
        #[cfg(test)]
        let ret = RwLock::new(Runtime {
            options: InitOptions {
                timecnt: 1,
                timewait: 0,
                bin: "/usr/bin/ls".to_string(),
                socket: "/tmp/sysmaster/init.socket".to_string(),
                args: None,
            },
            state: InitState::Running,
            pid: 0,
            debug: false,
            online: false,
        });
        ret
    })
}

macro_rules! runtime_read {
    () => {
        crate::runtime().read().await
    };
}

macro_rules! runtime_write {
    () => {
        crate::runtime().write().await
    };
}

impl Runtime {
    // This is an issue with the clippy tool. The new function is used under the not(test) configuration.
    #[allow(dead_code)]
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            options: InitOptions::parse(),
            state: InitState::Init,
            pid: 0,
            debug: false,
            online: false,
        })
    }

    pub fn is_running(&self) -> bool {
        self.state == InitState::Running
    }

    pub fn socket(&self) -> String {
        self.options.socket.clone()
    }

    pub fn bin(&self) -> String {
        self.options.bin.clone()
    }

    pub fn pid(&self) -> i32 {
        self.pid
    }

    pub fn set_pid(&mut self, pid: i32) {
        self.pid = pid;
    }

    pub fn _state(&self) -> InitState {
        self.state.clone()
    }

    pub fn set_state(&mut self, state: InitState) {
        self.state = state;
    }

    pub fn debug(&self) -> bool {
        true
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    pub fn online(&self) -> bool {
        self.online
    }

    pub fn set_online(&mut self, online: bool) {
        self.online = online;
    }

    pub fn timecnt(&self) -> usize {
        self.options.timecnt
    }

    pub fn timewait(&self) -> u64 {
        self.options.timewait
    }
}

async fn bootup() {
    log::info!("bootup started.");
    log::info!("bootup completed.");
}

fn rape_zombie() {
    // peek signal
    let flags = WaitPidFlag::WEXITED | WaitPidFlag::WNOHANG | WaitPidFlag::WNOWAIT;
    loop {
        let wait_status = match waitid(Id::All, flags) {
            Ok(status) => status,
            Err(_) => return,
        };

        let si = match wait_status {
            WaitStatus::Exited(pid, code) => Some((pid, code, Signal::SIGCHLD)),
            WaitStatus::Signaled(pid, signal, _dc) => Some((pid, -1, signal)),
            _ => None, // ignore
        };

        // check
        let (pid, _, _) = match si {
            Some((pid, code, sig)) => (pid, code, sig),
            None => {
                log::debug!("Ignored child signal: {:?}!", wait_status);
                return;
            }
        };

        if pid.as_raw() <= 0 {
            log::debug!("pid:{:?} is invalid! Ignored it.", pid);
            return;
        }

        // pop: recycle the zombie
        if let Err(e) = waitid(Id::Pid(pid), WaitPidFlag::WEXITED) {
            log::error!("Error when rape the zombie({:?}), ignoring: {:?}!", pid, e);
        } else {
            log::debug!("rape the zombie: pid:{:?}.", pid);
        }
    }
}

async fn signald() {
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigchld = signal(SignalKind::child()).unwrap();
    let mut sighup = signal(SignalKind::hangup()).unwrap();
    // let mut sigfault = signal(SignalKind::from_raw(9)).unwrap();

    loop {
        if !runtime_read!().is_running() {
            break;
        }

        tokio::select! {
            _ = sigterm.recv() => {
                runtime_write!().set_state(InitState::Reexec);
            }
            signal = sigint.recv() => {
                if signal.is_some() {
                println!("Received signal: SIGINT!");
                    let debug = !runtime_read!().debug();
                    runtime_write!().set_debug(debug);
                    std::process::exit(-1);
                }
            }
            _ = sigchld.recv() => {
                rape_zombie();
            }
            _ = sighup.recv() => {
                reload();
            }
        };
    }
}

async fn keepalive() {
    let rt_socket = runtime_read!().socket();
    let rt_bin = runtime_read!().bin();

    let sock_path = PathBuf::from(rt_socket.clone());
    let path = sock_path.parent().unwrap();
    if !path.exists() {
        if let Err(e) = fs::create_dir_all(path) {
            log::error!("Failed to create directory {path:?}: {e}!");
            return;
        }
    }
    if fs::metadata(sock_path.clone()).is_ok() {
        let _ = fs::remove_file(sock_path);
    }
    let listener = Arc::new(UnixListener::bind(rt_socket.clone()).unwrap());

    loop {
        if !runtime_read!().is_running() {
            break;
        }

        match listener.accept().await {
            Ok((stream, _)) => {
                // get pid of connection socket
                let cred = stream.peer_cred().unwrap();
                let pid = cred.pid().unwrap();
                let rt_pid = runtime_read!().pid();
                if pid == rt_pid {
                    // If the incoming PID is not the monitored sysmaster,
                    // do not refresh the status.
                    runtime_write!().set_online(true);
                    log::debug!(
                        "Keepalive:  receive a heartbeat from  {} ({})!",
                        rt_bin,
                        pid
                    );
                }
            }
            Err(e) => log::error!("Error accepting connection: {}!", e),
        }
    }
    let _ = fs::remove_file(rt_socket);
}

async fn watchdog() {
    let mut rt_timecnt = runtime_read!().timecnt();
    let rt_timewait = runtime_read!().timewait();
    let rt_bin = runtime_read!().bin();

    loop {
        if !runtime_read!().is_running() || rt_timecnt == 0 {
            break;
        }

        if !(runtime_read!().online()) {
            // False for two consecutive times,
            // indicating an abnormal state of sysmaster.
            check_bin().await;
            rt_timecnt -= 1;
            #[cfg(test)]
            runtime_write!().set_online(true);
        } else {
            runtime_write!().set_online(false);
        }

        sleep(Duration::from_secs(rt_timewait)).await;
    }
    log::info!(
        "Restarted {} {} times, {} seconds each time, will not continue.",
        rt_bin,
        rt_timecnt,
        rt_timewait
    );
}

async fn check_bin() {
    // check sysmaster status, if it is running then sigterm it
    let rt_pid = runtime_read!().pid();
    let rt_bin = runtime_read!().bin();
    if rt_pid != 0 {
        if let Ok(process) = Process::new(rt_pid as u32) {
            if process.is_running() {
                _ = process.terminate();
                log::info!("Timeout: send SIGTERM to {} ({})!", rt_bin, rt_pid);
                return;
            }
        }
    }

    // else start the binary
    let mut parts = rt_bin.split_whitespace();
    let command = match parts.next() {
        Some(c) => c,
        None => {
            log::error!("Wrong command: {:?}!", rt_bin);
            return;
        }
    };
    let args: Vec<&str> = parts.collect();
    if !Path::new(command).exists() {
        log::error!("{:?} does not exest!", command);
    }

    let child_process = match Command::new(command).args(&args).spawn() {
        Ok(child) => child,
        Err(_) => {
            log::error!("Failed to spawn process: {:?}.", command);
            return;
        }
    };

    if let Some(pid) = child_process.id() {
        runtime_write!().set_pid(pid as i32);
        log::info!("Startup: start {}({}))!", rt_bin, pid);
    }
}

fn prepare_init() {}
fn reset_all_signal_handlers() {}
fn install_crash_handler() {}
fn reexec() {}
fn reload() {}
fn shutdown_init() {
    nix::unistd::sync();
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    InitLog::init(LevelFilter::Info);
    prepare_init();
    reset_all_signal_handlers();
    install_crash_handler();

    runtime_write!().set_state(InitState::Running);
    let bootup_handle = tokio::spawn(async move { bootup().await });
    let signald_handle = tokio::spawn(async move { signald().await });
    let keepalive_handle = tokio::spawn(async move { keepalive().await });
    let watchdog_handle = tokio::spawn(async move { watchdog().await });

    _ = tokio::join!(
        bootup_handle,
        signald_handle,
        keepalive_handle,
        watchdog_handle
    );

    log::info!("all tasks completed!");

    reexec();

    shutdown_init();
    Ok(())
}

#[cfg(test)]
mod test {
    use std::{os::unix::net::UnixStream, time::Duration};

    use nix::{
        sys::signal::{kill, Signal},
        unistd::Pid,
    };
    use tokio::time::sleep;

    use crate::{keepalive, watchdog};
    use crate::{signald, InitState};

    #[tokio::test]
    async fn test_watchdog() {
        runtime_write!().set_state(InitState::Running);
        runtime_write!().set_online(false);
        watchdog().await;
        assert!(runtime_read!().online());
    }

    #[tokio::test]
    // In tests, signal handling is controlled and cannot be easily tested.
    async fn test_signald() {
        runtime_write!().set_state(InitState::Running);
        let handle1 = tokio::spawn(async move {
            loop {
                sleep(Duration::from_millis(100)).await;

                match kill(Pid::this(), Signal::SIGTERM) {
                    Ok(_) => log::info!("Signal sent successfully."),
                    Err(err) => log::error!("Failed to send signal: {}", err),
                }
            }
        });
        let handle2 = tokio::spawn(async move { signald().await });
        tokio::select! {
            _ = handle1 => { }
            _ = handle2 => { }
        };
        assert_eq!(runtime_read!().state, InitState::Reexec);
    }

    #[tokio::test]
    async fn test_keepalive() {
        runtime_write!().set_state(InitState::Running);
        let client = tokio::task::spawn(async move {
            #[allow(unused_assignments)]
            let mut connected = false;
            loop {
                sleep(Duration::from_millis(100)).await;
                match UnixStream::connect(runtime_read!().socket()) {
                    Ok(_) => {
                        connected = true;
                        break;
                    }
                    Err(_) => continue,
                };
            }
            assert!(connected);
        });

        let server = tokio::spawn(async move { keepalive().await });
        tokio::select! {
            _ = server => { }
            _ = client => { }
        }
    }
}
