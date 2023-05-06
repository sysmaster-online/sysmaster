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
use nix::sys::signal::{SigmaskHow, Signal};
use nix::sys::wait::{self, Id, WaitPidFlag, WaitStatus};
use nix::unistd;
use std::mem;
use std::ops::Neg;
use std::os::unix::io::RawFd;
use std::rc::Rc;

pub const SIG_RUN_UNRECOVER_OFFSET: i32 = 8;
pub const SIG_RESTART_MANAGER_OFFSET: i32 = 9;
pub const SIG_SWITCH_ROOT_OFFSET: i32 = 10;
const INVALID_FD: i32 = -1;

pub(crate) struct SigSet {
    sigset: libc::sigset_t,
}

impl SigSet {
    /// Initialize to include nothing.
    pub fn empty() -> SigSet {
        let mut sigset = mem::MaybeUninit::zeroed();
        let _ = unsafe { libc::sigemptyset(sigset.as_mut_ptr()) };

        unsafe {
            SigSet {
                sigset: sigset.assume_init(),
            }
        }
    }

    /// Add the specified signal to the set.
    pub fn add(&mut self, signal: libc::c_int) {
        unsafe {
            libc::sigaddset(
                &mut self.sigset as *mut libc::sigset_t,
                signal as libc::c_int,
            )
        };
    }
}

pub struct Signals {
    epoll: Rc<Epoll>,
    signal_fd: RawFd,
    set: SigSet,
    oldset: SigSet,
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
            set: SigSet::empty(),
            oldset: SigSet::empty(),
            signals,
            zombie_signal: libc::SIGCHLD,
            unrecover_signal: libc::SIGRTMIN() + SIG_RUN_UNRECOVER_OFFSET,
            restart_signal: libc::SIGRTMIN() + SIG_RESTART_MANAGER_OFFSET,
            switch_root_signal: libc::SIGRTMIN() + SIG_SWITCH_ROOT_OFFSET,
        }
    }

    pub fn is_zombie(&self, signo: i32) -> bool {
        self.zombie_signal == signo
    }

    pub fn is_restart(&self, signo: i32) -> bool {
        self.restart_signal == signo
    }

    pub fn is_unrecover(&self, signo: i32) -> bool {
        self.unrecover_signal == signo
    }

    pub fn is_switch_root(&self, signo: i32) -> bool {
        self.switch_root_signal == signo
    }

    pub fn create_signals_epoll(&mut self) -> Result<(), Errno> {
        self.reset_sigset();
        self.epoll.register(self.signal_fd)?;
        Ok(())
    }

    pub fn reset_sigset(&mut self) {
        for sig in self.signals.clone() {
            self.set.add(sig);
        }

        unsafe {
            libc::pthread_sigmask(libc::SIG_BLOCK, &self.set.sigset, &mut self.oldset.sigset);
            self.signal_fd = libc::signalfd(
                -1,
                &mut self.set.sigset as *const libc::sigset_t,
                libc::SFD_NONBLOCK,
            );
        }
    }

    pub fn read(&mut self, event: EpollEvent) -> Result<Option<libc::signalfd_siginfo>, Errno> {
        if self.epoll.is_err(event) {
            return Err(Errno::EIO);
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
                Ok(Some(info))
            }
            x if x >= 0 => Ok(None),
            x => {
                let err = Errno::from_i32(x.neg() as i32);
                eprintln!("read_signals failed err:{:?}", err);
                unistd::sleep(1);
                Ok(None)
            }
        }
    }

    pub fn recycle_zombie(&mut self, dest_pid: unistd::Pid) {
        // peek signal
        let flags = WaitPidFlag::WEXITED | WaitPidFlag::WNOHANG | WaitPidFlag::WNOWAIT;
        loop {
            // get wait information
            let mut id_flag = Id::All;
            if dest_pid.as_raw() > 0 {
                id_flag = Id::Pid(dest_pid);
            }

            let wait_status = match wait::waitid(id_flag, flags) {
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
        if fd == self.signal_fd {
            return true;
        }
        false
    }

    pub fn clear(&mut self) {
        self.epoll.safe_close(self.signal_fd);
        self.signal_fd = INVALID_FD;
        if let Err(e) = nix::sys::signal::pthread_sigmask(
            SigmaskHow::SIG_SETMASK,
            Some(&nix::sys::signal::SigSet::empty()),
            None,
        ) {
            eprintln!("reset pthread_sigmask failed: {e}");
        }
    }
}
