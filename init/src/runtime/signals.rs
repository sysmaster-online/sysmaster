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

use super::epoll::Epoll;
use nix::errno::Errno;
use nix::sys::epoll::EpollEvent;
use nix::sys::signal::{self, SigSet, SigmaskHow, Signal};
use nix::sys::signalfd::{self, SfdFlags};
use nix::sys::wait::{self, Id, WaitPidFlag, WaitStatus};
use nix::unistd;
use std::mem;
use std::ops::Neg;
use std::os::unix::io::RawFd;
use std::rc::Rc;

use constants::INVALID_FD;
use constants::{SIG_RESTART_MANAGER_OFFSET, SIG_RUN_UNRECOVER_OFFSET, SIG_SWITCH_ROOT_OFFSET};

pub struct Signals {
    epoll: Rc<Epoll>,
    pub signal_fd: RawFd,
    signals: Vec<i32>,
    zombie_signal: i32,
    restart_signal: i32,
    unrecover_signal: i32,
    switch_root_signal: i32,
}

impl Signals {
    pub fn new(epoll: &Rc<Epoll>) -> Self {
        let signals = (1..=libc::SIGRTMAX()).collect();

        Signals {
            epoll: epoll.clone(),
            signal_fd: INVALID_FD,
            signals,
            zombie_signal: libc::SIGCHLD,
            unrecover_signal: libc::SIGRTMIN() + SIG_RUN_UNRECOVER_OFFSET,
            restart_signal: libc::SIGRTMIN() + SIG_RESTART_MANAGER_OFFSET,
            switch_root_signal: libc::SIGRTMIN() + SIG_SWITCH_ROOT_OFFSET,
        }
    }

    pub fn is_zombie(&self, siginfo: libc::signalfd_siginfo) -> bool {
        self.zombie_signal as u32 == siginfo.ssi_signo
    }

    pub fn is_restart(&self, siginfo: libc::signalfd_siginfo) -> bool {
        self.restart_signal as u32 == siginfo.ssi_signo
    }

    pub fn is_unrecover(&self, siginfo: libc::signalfd_siginfo) -> bool {
        if 0 == siginfo.ssi_uid {
            return self.unrecover_signal as u32 == siginfo.ssi_signo;
        }
        false
    }

    pub fn is_switch_root(&self, siginfo: libc::signalfd_siginfo) -> bool {
        self.switch_root_signal as u32 == siginfo.ssi_signo
    }

    pub fn create_signals_epoll(&mut self) -> Result<(), Errno> {
        self.signal_fd = self.reset_sigset()?;
        self.epoll.register(self.signal_fd)?;
        Ok(())
    }

    pub fn reset_sigset(&mut self) -> Result<RawFd, Errno> {
        let mut sigset = SigSet::empty();
        for sig in self.signals.clone() {
            let signum: Signal = unsafe { std::mem::transmute(sig) };
            sigset.add(signum);
        }
        signal::pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&sigset), None)?;
        signalfd::signalfd(-1, &sigset, SfdFlags::SFD_CLOEXEC | SfdFlags::SFD_NONBLOCK)
    }

    pub fn read(&mut self, event: EpollEvent) -> Option<libc::signalfd_siginfo> {
        if self.epoll.is_err(event) {
            return self.recover();
        }
        let mut buffer = mem::MaybeUninit::<libc::signalfd_siginfo>::zeroed();

        let size = mem::size_of_val(&buffer);
        let res = unsafe {
            libc::read(
                self.signal_fd,
                buffer.as_mut_ptr() as *mut libc::c_void,
                size,
            )
        };

        match res {
            x if x == size as isize => {
                let info = unsafe { buffer.assume_init() };
                Some(info)
            }
            x if x >= 0 => None,
            x => {
                let err = Errno::from_i32(x.neg() as i32);
                eprintln!("read_signals failed err:{:?}", err);
                unistd::sleep(1);
                None
            }
        }
    }

    pub fn recycle_zombie(&mut self) {
        // peek signal
        let flags = WaitPidFlag::WEXITED | WaitPidFlag::WNOHANG | WaitPidFlag::WNOWAIT;
        loop {
            let wait_status = match wait::waitid(Id::All, flags) {
                Ok(status) => status,
                Err(_) => return,
            };

            let si = match wait_status {
                WaitStatus::Exited(pid, code) => Some((pid, code, Signal::SIGCHLD)),
                WaitStatus::Signaled(pid, signal, _dc) => Some((pid, -1, signal)),
                WaitStatus::StillAlive => None, // nothing to wait
                _ => None,                      // ignore it
            };

            // check
            let (pid, _, _) = match si {
                Some((pid, code, sig)) => (pid, code, sig),
                None => {
                    println!("Ignored child signal: {:?}", wait_status);
                    return;
                }
            };

            if pid.as_raw() <= 0 {
                println!("pid:{:?} is invalid! Ignored it.", pid);
                return;
            }

            // pop: recycle the zombie
            if let Err(e) = wait::waitid(Id::Pid(pid), WaitPidFlag::WEXITED) {
                println!("Error when recycle the zombie, ignoring: {:?}", e);
            } else {
                println!("recycle the zombie: pid:{:?}", pid);
            }
        }
    }

    pub fn is_fd(&self, fd: RawFd) -> bool {
        fd == self.signal_fd
    }

    fn recover(&mut self) -> Option<libc::signalfd_siginfo> {
        match self.reset_sigset() {
            Ok(signal_fd) => match self.epoll.register(signal_fd) {
                Ok(_) => {
                    self.epoll.safe_close(self.signal_fd);
                    self.signal_fd = signal_fd;
                    eprintln!("signals recover");
                }
                Err(e) => {
                    eprintln!("Failed to register signal_fd:{:?}", e);
                }
            },
            Err(e) => {
                eprintln!("Failed to create_signals_epoll:{:?}", e);
            }
        }
        None
    }
}

impl Drop for Signals {
    fn drop(&mut self) {
        self.epoll.safe_close(self.signal_fd);
        self.signal_fd = INVALID_FD;
        reset_signal_mask();
    }
}

pub fn reset_signal_mask() {
    let set = SigSet::empty();
    if let Err(e) = signal::pthread_sigmask(SigmaskHow::SIG_SETMASK, Some(&set), None) {
        eprintln!("reset signal mask failed: {e}");
    }
}
