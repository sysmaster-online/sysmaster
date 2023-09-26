use crate::config::Config;
use mio::{unix::SourceFd, Events, Interest, Poll, Token};
use nix::sys::{
    signal::{kill, Signal},
    signalfd::{SfdFlags, SigSet, SignalFd},
    socket::{getsockopt, sockopt::PeerCredentials},
    stat,
    time::{TimeSpec, TimeValLike},
    timerfd::ClockId,
    timerfd::{Expiration, TimerFd, TimerFlags, TimerSetTimeFlags},
    wait::{waitid, Id, WaitPidFlag, WaitStatus},
};
use nix::unistd::{execv, Pid};
use std::os::unix::io::AsRawFd;
use std::{
    ffi::CString,
    fs::{self, File},
    io::{self, Read},
    os::unix::{net::UnixListener, prelude::FileTypeExt},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

#[cfg(not(test))]
pub const INIT_SOCK: &str = "/run/sysmaster/init.sock";
#[cfg(test)]
pub const INIT_SOCK: &str = "init.sock";

const ALLFD_TOKEN: Token = Token(0);
const TIMERFD_TOKEN: Token = Token(1);
const SIGNALFD_TOKEN: Token = Token(2);
const SOCKETFD_TOKEN: Token = Token(3);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitState {
    Init,
    Running,
    Reexec,
}

pub struct Runtime {
    poll: Poll,
    timerfd: TimerFd,
    signalfd: SignalFd,
    socketfd: UnixListener,
    config: Config,
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
                            Err(e) => panic!("Invalid value: {:?}", e),
                        };
                    } else {
                        panic!("Missing value for option --pid.");
                    }
                }
                _ => {
                    log::debug!("Unknown items: {}, ignored!", arg);
                }
            }
        }

        // check socket
        if let Ok(metadata) = fs::metadata(INIT_SOCK) {
            if metadata.file_type().is_socket() {
                fs::remove_file(INIT_SOCK)?;
            }
        }
        let sock_path = PathBuf::from(INIT_SOCK);
        let sock_parent = sock_path.parent().unwrap();
        if !sock_parent.exists() {
            let old_mask = stat::umask(stat::Mode::from_bits_truncate(!0o755));
            fs::create_dir_all(sock_parent)?;
            let _ = stat::umask(old_mask);
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
        let signalfd = SignalFd::with_flags(&mask, SfdFlags::SFD_CLOEXEC | SfdFlags::SFD_NONBLOCK)?;

        // set timer
        let timerfd = TimerFd::new(
            ClockId::CLOCK_MONOTONIC,
            TimerFlags::TFD_NONBLOCK | TimerFlags::TFD_CLOEXEC,
        )?;
        timerfd.set(
            Expiration::OneShot(TimeSpec::from_duration(Duration::from_nanos(1))),
            TimerSetTimeFlags::empty(),
        )?;

        // parse config
        let config = Config::load(None)?;

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
        let signalfd = self.signalfd.as_raw_fd();
        let mut signal_source = SourceFd(&signalfd);
        let timerfd = self.timerfd.as_raw_fd();
        let mut time_source = SourceFd(&timerfd);
        let socketfd = self.socketfd.as_raw_fd();
        let mut unix_source = SourceFd(&socketfd);

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
        let signalfd = self.signalfd.as_raw_fd();
        let mut signal_source = SourceFd(&signalfd);
        let timerfd = self.timerfd.as_raw_fd();
        let mut time_source = SourceFd(&timerfd);
        let socketfd = self.socketfd.as_raw_fd();
        let mut unix_source = SourceFd(&socketfd);

        match token {
            SIGNALFD_TOKEN => self.poll.registry().deregister(&mut signal_source)?,
            TIMERFD_TOKEN => self.poll.registry().deregister(&mut time_source)?,
            SOCKETFD_TOKEN => {
                self.poll.registry().deregister(&mut unix_source)?;
            }
            _ => {
                self.poll.registry().deregister(&mut signal_source)?;
                self.poll.registry().deregister(&mut time_source)?;
                self.poll.registry().deregister(&mut unix_source)?;
            }
        }

        if [SOCKETFD_TOKEN, ALLFD_TOKEN].contains(&token) && fs::metadata(INIT_SOCK).is_ok() {
            fs::remove_file(INIT_SOCK)?;
        };

        Ok(())
    }

    fn load_config(&mut self) -> std::io::Result<()> {
        self.config = match Config::load(None) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to load config, error: {:?}, ignored!", e);
                return Ok(());
            }
        };
        Ok(())
    }

    fn reap_zombies(&self) {
        // peek signal
        let flags = WaitPidFlag::WEXITED | WaitPidFlag::WNOHANG | WaitPidFlag::WNOWAIT;
        loop {
            let wait_status = match waitid(Id::All, flags) {
                Ok(status) => status,
                Err(e) => {
                    log::warn!("Error when waitid for all, {}", e);
                    continue;
                }
            };

            let si = match wait_status {
                WaitStatus::Exited(pid, code) => Some((pid, code, Signal::SIGCHLD)),
                WaitStatus::Signaled(pid, signal, _dc) => Some((pid, -1, signal)),
                _ => None, // ignore
            };

            // pop: recycle the zombie
            if let Some((pid, _, _)) = si {
                if let Err(e) = waitid(Id::Pid(pid), WaitPidFlag::WEXITED) {
                    log::error!("Error when reap the zombie({:?}), ignored: {:?}!", pid, e);
                }
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
                log::debug!("Received SIGINT for pid({:?})", sig.ssi_pid);
                self.exit(1);
            }
            Signal::SIGKILL => {
                self.kill_sysmaster();
            }
            Signal::SIGTERM => self.state = InitState::Reexec,
            Signal::SIGCHLD => self.reap_zombies(),
            _ => {
                log::debug!(
                    "Received signo {:?} for pid({:?}), ignored!",
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
                "Keepalive: we tried multiple times, and no longer start {:?}.",
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
                log::error!("Error accepting connection: {}!", e);
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
        }
        Ok(())
    }

    fn pid_is_running(&self, pid: u32) -> bool {
        let path = format!("/proc/{}/comm", pid);
        let file = Path::new(&path);
        if file.exists() {
            let mut content = String::new();
            let file = File::open(file);
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
                log::error!("Wrong command: {:?}!", self.config.bin);
                return;
            }
        };
        let args: Vec<&str> = parts.collect();
        if !Path::new(command).exists() {
            log::error!("Command {:?} does not exist!", command);
        }

        let child_process = match Command::new(command).args(&args).spawn() {
            Ok(child) => child,
            Err(_) => {
                log::error!("Failed to spawn process: {:?}.", command);
                return;
            }
        };

        self.pid = child_process.id();
        log::info!("Success to start {}({}))!", self.config.bin, self.pid);
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
        log::info!("Reloading init configuration!");
        self.load_config()?;
        Ok(())
    }

    pub fn is_reexec(&self) -> bool {
        self.state == InitState::Reexec
    }

    pub fn reexec(&mut self) {
        // Get the current executable path
        let exe = match std::env::current_exe().unwrap().file_name() {
            Some(v) => v.to_string_lossy().to_string(),
            None => "".to_string(),
        };

        for arg0 in [&exe, "/init", "/sbin/init"] {
            let argv = vec![arg0.to_string(), "--pid".to_string(), self.pid.to_string()];

            // Convert the argument and argument vector to CStrings
            let cstr_arg0 = CString::new(arg0).unwrap();
            let cstr_argv = argv
                .iter()
                .map(|arg| CString::new(arg.as_str()).unwrap())
                .collect::<Vec<_>>();

            log::info!("Reexecuting init: {:?}", argv);

            // Execute the new process
            if let Err(e) = execv(&cstr_arg0, &cstr_argv) {
                log::error!("Execv {:?} {:?} failed: {:?}", arg0, argv, e);
            }
        }
    }

    fn kill_sysmaster(&mut self) -> bool {
        if self.pid_is_running(self.pid) {
            let target_pid = Pid::from_raw(self.pid.try_into().unwrap());

            match kill(target_pid, Signal::SIGTERM) {
                Ok(_) => {
                    log::info!(
                        "Timeout, send SIGTERM to {} ({})!",
                        self.config.bin,
                        self.pid
                    );
                    return true;
                }
                Err(err) => log::error!(
                    "Timeout, failed to send SIGTERM to {} ({}), {}, ignore!",
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

impl Drop for Runtime {
    fn drop(&mut self) {
        if fs::metadata(INIT_SOCK).is_ok() {
            let _ = fs::remove_file(INIT_SOCK);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_runtime() -> std::io::Result<()> {
        use crate::runtime::*;
        let mut rt = Runtime::new()?;
        rt.set_state(InitState::Running);
        rt.config.timewait = 0;
        rt.runloop()?;
        assert_ne!(rt.timerfd.as_raw_fd(), 0);
        assert_ne!(rt.signalfd.as_raw_fd(), 0);
        assert_ne!(rt.socketfd.as_raw_fd(), 0);
        Ok(())
    }
}
