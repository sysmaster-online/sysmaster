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
use nix::sys::signal::Signal;
use nix::sys::wait::{self, Id, WaitPidFlag, WaitStatus};
use std::mem;
use std::ops::Neg;
use std::rc::Rc;

pub const RUN_UNRECOVER_SIG_OFFSET: i32 = 8;
pub const RESTART_MANAGER_SIG_OFFSET: i32 = 9;
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
    pub epoll: Rc<Epoll>,
    pub signal_fd: i32,
    set: SigSet,
    oldset: SigSet,
    signals: Vec<i32>,
    pub zombie_signal: i32,
    pub restart_signal: i32,
    pub unrecover_signal: i32,
}

impl Signals {
    pub fn new(epoll: &Rc<Epoll>) -> Self {
        let signals = vec![
            libc::SIGRTMIN() + RUN_UNRECOVER_SIG_OFFSET,
            libc::SIGRTMIN() + RESTART_MANAGER_SIG_OFFSET,
            libc::SIGCHLD,
        ];

        Signals {
            epoll: epoll.clone(),
            signal_fd: INVALID_FD,
            set: SigSet::empty(),
            oldset: SigSet::empty(),
            signals,
            zombie_signal: libc::SIGCHLD,
            unrecover_signal: libc::SIGRTMIN() + RUN_UNRECOVER_SIG_OFFSET,
            restart_signal: libc::SIGRTMIN() + RESTART_MANAGER_SIG_OFFSET,
        }
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

    pub fn read_signals(&mut self) -> Result<Option<i32>, Errno> {
        let mut buffer = mem::MaybeUninit::<libc::siginfo_t>::zeroed();
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
                Ok(Some(info.si_signo))
            }
            x if x >= 0 => Ok(None),
            x => {
                let err = Errno::from_i32(x.neg() as i32);
                println!("read_signals failed err:{:?}", err);
                Err(err)
            }
        }
    }

    pub fn recycle_zombie(&mut self) -> Result<(), Errno> {
        // peek signal
        let flags = WaitPidFlag::WEXITED | WaitPidFlag::WNOHANG | WaitPidFlag::WNOWAIT;
        loop {
            // get wait information
            let wait_status = match wait::waitid(Id::All, flags) {
                Ok(status) => status,
                Err(err) => {
                    println!("Error while waiting pid: {:?}", err);
                    return Ok(());
                }
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
                    return Ok(());
                }
            };

            if pid.as_raw() <= 0 {
                println!("Ignored pid in signal: {:?}", pid);
                return Ok(());
            }

            // pop: reap the zombie
            if let Err(e) = wait::waitid(Id::Pid(pid), WaitPidFlag::WEXITED) {
                println!("Error when reap the zombie, ignoring: {:?}", e);
            } else {
                println!("reap the zombie: pid:{:?}", pid);
            }
        }
    }
}
