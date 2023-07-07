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
use mio::unix::SourceFd;
use mio::Events;
use mio::Interest;
use mio::Poll;
use mio::Token;
use nix::sys::signal;
use nix::sys::signal::kill;
use nix::sys::signal::SaFlags;
use nix::sys::signal::SigAction;
use nix::sys::signal::SigHandler;
use nix::sys::signal::Signal;
use nix::sys::signalfd::SigSet;
use nix::sys::signalfd::SignalFd;
use nix::sys::socket::getsockopt;
use nix::sys::socket::sockopt::PeerCredentials;
use nix::sys::stat;
use nix::sys::stat::umask;
use nix::sys::stat::Mode;
use nix::sys::time::TimeSpec;
use nix::sys::time::TimeValLike;
use nix::sys::timerfd::ClockId;
use nix::sys::timerfd::Expiration;
use nix::sys::timerfd::TimerFd;
use nix::sys::timerfd::TimerFlags;
use nix::sys::timerfd::TimerSetTimeFlags;
use nix::sys::wait::waitid;
use nix::sys::wait::Id;
use nix::sys::wait::WaitPidFlag;
use nix::sys::wait::WaitStatus;
use nix::unistd;
use nix::unistd::execv;
use nix::unistd::Pid;
#[allow(unused_imports)]
use nix::unistd::Uid;
use std::ffi::CString;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const ALLFD_TOKEN: Token = Token(0);
const TIMERFD_TOKEN: Token = Token(1);
const SIGNALFD_TOKEN: Token = Token(2);
const SOCKETFD_TOKEN: Token = Token(3);
#[cfg(not(test))]
const INIT_SOCK: &str = "/run/sysmaster/init.sock";
#[cfg(test)]
const INIT_SOCK: &str = "init.sock";
const INIT_CONFIG: &str = "/etc/sysmaster/init.conf";

#[derive(Debug)]
struct InitConfig {
    pub timecnt: usize,
    pub timewait: u64,
    pub bin: String,
}

impl InitConfig {
    fn parse_config_line(line: &str) -> Option<(String, String)> {
        let mut iter = line.splitn(2, '=');
        let key = iter.next()?.trim();
        let value = iter.next()?.trim();

        Some((key.to_string(), value.to_string()))
    }

    pub fn load(path: Option<String>) -> std::io::Result<Self> {
        let mut config = Self::default();
        let default_config_file = INIT_CONFIG.to_string();
        let path = path.unwrap_or(default_config_file);
        let file = Path::new(&path);
        if file.exists() {
            let mut content = String::new();
            let file = File::open(&file);
            match file.map(|mut f| f.read_to_string(&mut content)) {
                Ok(_) => (),
                Err(_) => return Ok(config),
            };

            for (_, line) in content.lines().enumerate() {
                let trimmed_line = line.trim();
                if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = InitConfig::parse_config_line(trimmed_line) {
                    match key.as_str() {
                        "timecnt" => match value.as_str().parse::<usize>() {
                            Ok(v) => config.timecnt = v,
                            Err(e) => {
                                log::warn!(
                                    "parse timecnt failed: {:?}, use default({:?})!",
                                    e,
                                    config.timecnt
                                );
                            }
                        },
                        "timewait" => match value.as_str().parse::<u64>() {
                            Ok(v) => config.timewait = v,
                            Err(e) => {
                                log::warn!(
                                    "parse timewait failed: {:?}, use default({:?})!",
                                    e,
                                    config.timewait
                                );
                            }
                        },
                        "bin" => match value.as_str().parse::<String>() {
                            Ok(v) => config.bin = v,
                            Err(e) => {
                                log::warn!(
                                    "parse bin failed: {:?}, use default({:?})!",
                                    e,
                                    config.bin
                                );
                            }
                        },
                        _ => log::warn!("parse config error, use default!"),
                    }
                }
            }
            log::debug!("{:?}", config);
        }

        Ok(config)
    }
}

impl Default for InitConfig {
    fn default() -> Self {
        Self {
            timecnt: 10,
            timewait: 90,
            bin: "/usr/lib/sysmaster/sysmaster".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InitState {
    Init,
    Running,
    Reexec,
}

struct Runtime {
    poll: Poll,
    timerfd: TimerFd,
    signalfd: SignalFd,
    socketfd: UnixListener,
    config: InitConfig,
    state: InitState,
    // sysmaster pid
    pid: u32,
    // sysmaster status
    online: bool,
    deserialize: bool,
}

impl Runtime {
    pub fn new() -> std::io::Result<Self> {
        // parse arguments, --pid, Invisible to user
        let mut pid = 0u32;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--pid" => {
                    if let Some(value) = args.next() {
                        if value.starts_with('-') {
                            panic!("Missing or invalid value for option.");
                        }
                        pid = match value.parse::<u32>() {
                            Ok(v) => v,
                            Err(e) => panic!("Invalid value: {e:?}"),
                        };
                    } else {
                        panic!("Missing value for option --deserialize.");
                    }
                }
                _ => {
                    log::debug!("unknown items: {}, ignored!", arg);
                }
            }
        }

        // check socket
        let sock_path = PathBuf::from(INIT_SOCK);
        let sock_parent = sock_path.parent().unwrap();
        if !sock_parent.exists() {
            let old_mask = stat::umask(stat::Mode::from_bits_truncate(!0o755));
            let ret = fs::create_dir_all(sock_parent);
            let _ = stat::umask(old_mask);
            if let Err(e) = ret {
                return Err(e);
            }
        }
        if fs::metadata(INIT_SOCK).is_ok() {
            let _ = fs::remove_file(INIT_SOCK);
        }
        let socketfd = UnixListener::bind(INIT_SOCK)?;

        // add signal
        let mut mask = SigSet::empty();
        for sig in [
            Signal::SIGINT,
            Signal::SIGTERM,
            Signal::SIGCHLD,
            Signal::SIGHUP,
        ] {
            mask.add(sig);
        }
        mask.thread_set_mask()?;
        let signalfd = SignalFd::new(&mask)?;

        // set timer
        let timerfd = TimerFd::new(ClockId::CLOCK_MONOTONIC, TimerFlags::TFD_NONBLOCK)?;
        timerfd.set(
            Expiration::OneShot(TimeSpec::from_duration(Duration::from_nanos(1))),
            TimerSetTimeFlags::empty(),
        )?;

        // parse config
        let config = InitConfig::load(None)?;

        Ok(Self {
            poll: Poll::new()?,
            timerfd,
            signalfd,
            socketfd,
            config,
            state: InitState::Init,
            pid,
            online: false,
            deserialize: pid != 0,
        })
    }

    pub fn register(&mut self, token: Token) -> std::io::Result<()> {
        let binding = self.signalfd.as_raw_fd();
        let mut signal_source = SourceFd(&binding);
        let binding = self.timerfd.as_raw_fd();
        let mut time_source = SourceFd(&binding);
        let binding = self.socketfd.as_raw_fd();
        let mut unix_source = SourceFd(&binding);

        match token {
            SIGNALFD_TOKEN => self.poll.registry().register(
                &mut signal_source,
                SIGNALFD_TOKEN,
                Interest::READABLE,
            )?,
            TIMERFD_TOKEN => self.poll.registry().register(
                &mut time_source,
                TIMERFD_TOKEN,
                Interest::READABLE,
            )?,
            SOCKETFD_TOKEN => self.poll.registry().register(
                &mut unix_source,
                SOCKETFD_TOKEN,
                Interest::READABLE,
            )?,
            _ => {
                self.poll.registry().register(
                    &mut signal_source,
                    SIGNALFD_TOKEN,
                    Interest::READABLE,
                )?;
                self.poll.registry().register(
                    &mut time_source,
                    TIMERFD_TOKEN,
                    Interest::READABLE,
                )?;
                self.poll.registry().register(
                    &mut unix_source,
                    SOCKETFD_TOKEN,
                    Interest::READABLE,
                )?;
            }
        }

        Ok(())
    }

    pub fn deregister(&mut self, token: Token) -> std::io::Result<()> {
        let binding = self.signalfd.as_raw_fd();
        let mut signal_source = SourceFd(&binding);
        let binding = self.timerfd.as_raw_fd();
        let mut time_source = SourceFd(&binding);
        let binding = self.socketfd.as_raw_fd();
        let mut unix_source = SourceFd(&binding);

        match token {
            SIGNALFD_TOKEN => self.poll.registry().deregister(&mut signal_source)?,
            TIMERFD_TOKEN => self.poll.registry().deregister(&mut time_source)?,
            SOCKETFD_TOKEN => self.poll.registry().deregister(&mut unix_source)?,
            _ => {
                self.poll.registry().deregister(&mut signal_source)?;
                self.poll.registry().deregister(&mut time_source)?;
                self.poll.registry().deregister(&mut unix_source)?;
            }
        }

        Ok(())
    }

    fn load_config(&mut self) -> std::io::Result<()> {
        self.config = match InitConfig::load(None) {
            Ok(c) => c,
            Err(e) => {
                log::error!("failed to load config, error: {:?}, ignored!", e);
                return Ok(());
            }
        };
        Ok(())
    }

    fn reap_zombie(&self) {
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
                    log::debug!("ignored child signal: {:?}!", wait_status);
                    return;
                }
            };

            if pid.as_raw() <= 0 {
                log::debug!("pid:{:?} is invalid! ignored.", pid);
                return;
            }

            // pop: recycle the zombie
            if let Err(e) = waitid(Id::Pid(pid), WaitPidFlag::WEXITED) {
                log::error!("error when reap the zombie({:?}), ignored: {:?}!", pid, e);
            } else {
                log::debug!("reap the zombie: {:?}.", pid);
            }
        }
    }

    pub fn handle_signal(&mut self) -> std::io::Result<()> {
        let sig = match self.signalfd.read_signal()? {
            Some(s) => s,
            None => return Ok(()),
        };
        match Signal::try_from(sig.ssi_signo as i32)? {
            Signal::SIGHUP => self.reload()?,
            Signal::SIGINT => {
                log::debug!("received SIGINT for pid({:?})", sig.ssi_pid);
                self.exit(1);
            }
            Signal::SIGKILL => {
                self.kill_sysmaster();
            }
            Signal::SIGTERM => self.state = InitState::Reexec,
            Signal::SIGCHLD => self.reap_zombie(),
            _ => {
                log::debug!(
                    "received signo {:?} for pid({:?}), ignored!",
                    sig.ssi_signo,
                    sig.ssi_pid
                );
            }
        };
        Ok(())
    }

    pub fn handle_timer(&mut self) -> std::io::Result<()> {
        if self.config.timecnt == 0 {
            log::error!(
                "keepalive: we tried multiple times, and no longer start {:?}.",
                self.config.bin
            );
            self.deregister(TIMERFD_TOKEN)?;
            self.deregister(SOCKETFD_TOKEN)?;
            return Ok(());
        }

        if self.online {
            self.online = false;
        } else {
            self.start_bin();
            self.config.timecnt -= 1;
        }
        self.timerfd.set(
            Expiration::OneShot(TimeSpec::seconds(self.config.timewait as i64)),
            TimerSetTimeFlags::empty(),
        )?;
        Ok(())
    }

    pub fn handle_socket(&mut self) -> std::io::Result<()> {
        let (stream, _) = match self.socketfd.accept() {
            Ok((connection, address)) => (connection, address),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                // If we get a `WouldBlock` error we know our
                // listener has no more incoming connections queued,
                // so we can return to polling and wait for some
                // more.
                return Ok(());
            }
            Err(e) => {
                // If it was any other kind of error, something went
                // wrong and we terminate with an error.
                log::error!("error accepting connection: {}!", e);
                return Err(e);
            }
        };

        let credentials = getsockopt(stream.as_raw_fd(), PeerCredentials)?;
        let pid = credentials.pid() as u32;
        if self.pid_is_running(pid) {
            // If the incoming PID is not the monitored sysmaster,
            // do not refresh the status.
            self.online = true;
            self.pid = pid;
            log::debug!("keepalive: receive a heartbeat from pid({})!", pid);
        }
        Ok(())
    }

    fn pid_is_running(&self, pid: u32) -> bool {
        let path = format!("/proc/{}/comm", pid);
        let file = Path::new(&path);
        if file.exists() {
            let mut content = String::new();
            let file = File::open(&file);
            match file.map(|mut f| f.read_to_string(&mut content)) {
                Ok(_) => (),
                Err(_) => return false,
            };
            if content.starts_with("sysmaster") {
                return true;
            }
        }

        false
    }

    fn start_bin(&mut self) {
        // check sysmaster status, if it is running then sigterm it
        if self.pid != 0 && (self.deserialize || self.kill_sysmaster()) {
            return;
        }

        // else start the binary
        let mut parts = self.config.bin.split_whitespace();
        let command = match parts.next() {
            Some(c) => c,
            None => {
                log::error!("wrong command: {:?}!", self.config.bin);
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
                log::error!("failed to spawn process: {:?}.", command);
                return;
            }
        };

        self.pid = child_process.id();
        log::info!("success to start {}({}))!", self.config.bin, self.pid);
    }

    pub fn runloop(&mut self) -> std::io::Result<()> {
        self.register(ALLFD_TOKEN)?;
        let mut events = Events::with_capacity(16);

        // event loop.
        loop {
            if !self.is_running() {
                self.deregister(ALLFD_TOKEN)?;
                break;
            }

            self.poll.poll(&mut events, None)?;

            // Process each event.
            for event in events.iter() {
                match event.token() {
                    SIGNALFD_TOKEN => self.handle_signal()?,
                    TIMERFD_TOKEN => self.handle_timer()?,
                    SOCKETFD_TOKEN => self.handle_socket()?,
                    _ => unreachable!(),
                }
            }

            #[cfg(test)]
            self.set_state(InitState::Init);
        }

        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.state == InitState::Running
    }

    pub fn set_state(&mut self, state: InitState) {
        self.state = state;
    }

    fn reload(&mut self) -> std::io::Result<()> {
        log::info!("reloading init configuration!");
        self.load_config()?;
        Ok(())
    }

    pub fn is_reexec(&self) -> bool {
        self.state == InitState::Reexec
    }

    pub fn reexec(&mut self) {
        let exe = match std::env::current_exe().unwrap().file_name() {
            Some(v) => v.to_string_lossy().to_string(),
            None => "".to_string(),
        };
        let pid = self.pid.to_string();

        for arg0 in [&exe, "/init", "/sbin/init"] {
            let argv = vec![
                <&str>::clone(&arg0).to_string(),
                "--pid".to_string(),
                pid.clone(),
            ];
            let cstr_argv = argv
                .iter()
                .map(|str| std::ffi::CString::new(&**str).unwrap())
                .collect::<Vec<_>>();
            log::info!("reexecuting init, {:?}!", argv.as_slice());
            if let Err(e) = execv(&CString::new(<&str>::clone(&arg0)).unwrap(), &cstr_argv) {
                log::error!("execv {arg0:?} {argv:?} failed: {e}!");
            };
        }
    }

    fn kill_sysmaster(&mut self) -> bool {
        if self.pid_is_running(self.pid) {
            let target_pid = Pid::from_raw(self.pid.try_into().unwrap());

            match kill(target_pid, Signal::SIGTERM) {
                Ok(_) => {
                    log::info!(
                        "timeout, send SIGTERM to {} ({})!",
                        self.config.bin,
                        self.pid
                    );
                    return true;
                }
                Err(err) => log::error!(
                    "timeout, failed to send SIGTERM to {} ({}), {}, ignore!",
                    self.config.bin,
                    self.pid,
                    err
                ),
            }
        }
        false
    }

    fn exit(&self, i: i32) {
        std::process::exit(i);
    }
}

fn prepare_init() {
    // version
    let version = env!("CARGO_PKG_VERSION");
    log::info!("sysMaster init version: {}", version);
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&String::from("--version")) || args.contains(&String::from("-V")) {
        println!("sysMaster init version: {}!", version);
        std::process::exit(0);
    }

    // common umask
    let mode = Mode::from_bits_truncate(0o77);
    umask(umask(mode) | Mode::from_bits_truncate(0o22));

    // euid check
    #[cfg(not(test))]
    if unistd::geteuid() != Uid::from_raw(0) {
        log::error!("must be superuser.");
        std::process::exit(1);
    }

    if unistd::getpid() != Pid::from_raw(1) {
        log::info!("running in the test mode.");
    }
}

fn reset_all_signal_handlers() {
    // Create an empty signal set
    let mut sigset = SigSet::empty();

    // Add all signals to the signal set
    for sig in signal::Signal::iterator() {
        if sig == signal::Signal::SIGKILL || sig == signal::Signal::SIGSTOP {
            continue; // Do not allow ignoring SIGKILL and SIGSTOP signals
        }
        sigset.add(sig);
    }

    // Set the signal handler to be ignored
    let sig_action = SigAction::new(SigHandler::SigIgn, SaFlags::SA_RESTART, SigSet::empty());
    for sig in sigset.iter() {
        unsafe {
            signal::sigaction(sig, &sig_action).expect("failed to set signal handler!");
        }
    }
}

extern "C" fn crash_handler(_signal: i32) {
    log::error!("crash_handler");
}

fn install_crash_handler() {
    let signals_crash_handler = [
        signal::SIGSEGV,
        signal::SIGILL,
        signal::SIGFPE,
        signal::SIGBUS,
        signal::SIGABRT,
    ];
    let sig_action = SigAction::new(
        SigHandler::Handler(crash_handler),
        SaFlags::SA_SIGINFO | SaFlags::SA_NODEFER,
        SigSet::empty(),
    );

    for sig in signals_crash_handler {
        unsafe {
            signal::sigaction(sig, &sig_action).expect("Failed to set crash signal handler!");
        }
    }
}

fn shutdown_init() {
    nix::unistd::sync();
    log::info!("shutdowning init");
}

fn main() -> std::io::Result<()> {
    match kernlog::init() {
        Ok(_) => (),
        Err(e) => panic!("Unsupported when cannot log into /dev/kmsg : {e:?}!"),
    };

    prepare_init();

    reset_all_signal_handlers();
    install_crash_handler();

    let mut rt = Runtime::new()?;
    rt.set_state(InitState::Running);

    rt.runloop()?;

    if rt.is_reexec() {
        rt.reexec();
    }
    shutdown_init();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_runtime() -> std::io::Result<()> {
        let mut rt = Runtime::new()?;
        rt.set_state(InitState::Running);
        rt.config.timewait = 0;
        rt.runloop()?;
        assert_ne!(rt.timerfd.as_raw_fd(), 0);
        assert_ne!(rt.signalfd.as_raw_fd(), 0);
        assert_ne!(rt.socketfd.as_raw_fd(), 0);
        fs::remove_file("init.sock").unwrap();
        Ok(())
    }

    #[test]
    fn test_default_config() {
        let config = InitConfig::default();
        assert_eq!(config.timecnt, 10);
        assert_eq!(config.timewait, 90);
        assert_eq!(config.bin, "/usr/lib/sysmaster/sysmaster");
    }

    #[test]
    fn test_load_fail_defconfig() {
        let config = InitConfig::load(Some("/path/to/init.conf".to_string())).unwrap();
        assert_eq!(config.timecnt, 10);
        assert_eq!(config.timewait, 90);
        assert_eq!(config.bin, "/usr/lib/sysmaster/sysmaster");
    }

    #[test]
    fn test_load_success_config() {
        let content = "
#[config(default = 10)]
timecnt = 9
#[config(default = 90)]
    timewait =1
#[config(default = \"/usr/lib/sysmaster/sysmaster\")]
bin = /bin/ls
#[config(default = \"/run/sysmaster/init.sock\")]
socket = init.sock
";
        let file_path = "./init/src/init.conf";

        if let Ok(mut file) = File::create(file_path) {
            if let Err(err) = file.write_all(content.as_bytes()) {
                eprintln!("Write file error: {}.", err);
            } else {
                println!("Success to write.");
            }
        } else {
            eprintln!("Failed to write file.");
        }
        let config = InitConfig::load(Some(file_path.to_string())).unwrap();
        assert_eq!(config.timecnt, 9);
        assert_eq!(config.timewait, 1);
        assert_eq!(config.bin, "/bin/ls");
        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_main() {
        prepare_init();

        reset_all_signal_handlers();
        install_crash_handler();
        let mut rt = Runtime::new().unwrap();
        rt.set_state(InitState::Running);

        rt.runloop().unwrap();

        if rt.is_reexec() {
            rt.reexec();
        }
        shutdown_init();
    }
}
