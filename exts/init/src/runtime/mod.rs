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

mod alive;
mod epoll;
pub mod param;
mod signals;

use alive::Alive;
use epoll::Epoll;
use param::Param;
use signals::Signals;

use nix::libc;
use nix::unistd::{self, ForkResult, Pid};
use std::os::unix::process::CommandExt;
use std::process::{exit, Command};

use nix::errno::Errno;
use nix::sys::epoll::EpollFlags;
use std::rc::Rc;

const INVALID_FD: i32 = -1;
const INVALID_PID: i32 = -1;
const MANAGER_SIG_OFFSET: i32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum InitState {
    Reexec = 0,
    RunRecover = 1,
    RunUnRecover = 2,
}

pub struct RunTime {
    cmd: Param,
    sysmaster_pid: Pid,
    state: InitState,
    epoll: Rc<Epoll>,
    alive: Alive,
    signals: Signals,
    need_reexec: bool,
}

impl RunTime {
    pub fn new(mut cmd: Param) -> Result<RunTime, Errno> {
        cmd.get_opt();
        let ep = Epoll::new()?;
        let epoll = Rc::new(ep);
        let alive = Alive::new(&epoll, cmd.time_wait, cmd.time_out)?;
        let signals = Signals::new(&epoll);

        Ok(RunTime {
            cmd,
            sysmaster_pid: unistd::Pid::from_raw(INVALID_PID),
            state: InitState::Reexec,
            epoll,
            alive,
            signals,
            need_reexec: false,
        })
    }

    pub fn init(&mut self) -> Result<(), Errno> {
        self.signals.create_signals_epoll()?;

        self.alive.init()?;

        self.create_sysmaster()?;
        self.state = InitState::Reexec;
        Ok(())
    }

    pub fn reexec(&mut self) -> Result<(), Errno> {
        if self.need_reexec {
            // if the status of reexec_manager is not Reexec, return directly!
            if InitState::Reexec != self.reexec_manager()? {
                return Ok(());
            }
        }

        let events = self.epoll.wait()?;

        for event in events {
            let ep_event = event.events();
            let ep_data = event.data();
            println!("ep_event:{:?} ep_data:{:?}", ep_event, ep_data);
            if let true = self.ep_event_err_proc(ep_event, ep_data)? {
                return Ok(());
            }

            if self.alive.connect_fd as u64 == ep_data {
                let buf = self.alive.recv_buf()?;
                if self.alive.is_unmanageable(&buf) {
                    self.need_reexec = true;
                } else if self.alive.is_manageable(&buf) && !self.need_reexec {
                    self.state = InitState::RunRecover;
                    self.alive.manager_time_count = 0;
                    self.alive.alive_time_count = 0;
                } else {
                    eprintln!("recv buf is invalid! {:?}", buf);
                }
                continue;
            }
            if self.alive.time_fd as u64 == ep_data {
                self.epoll.read(self.alive.time_fd)?;
                self.alive.manager_time_count += 1;
                if self.alive.time_out <= self.alive.manager_time_count {
                    self.need_reexec = true;
                }
                continue;
            }
            if self.signals.signal_fd as u64 == ep_data {
                let res = self.signals.read_signals()?;
                match res {
                    Some(signal) => {
                        self.run_dispatch_signal(signal)?;
                    }
                    None => println!("read_signals is None!"),
                }
                continue;
            }
        }

        Ok(())
    }

    pub fn get_state(&self) -> InitState {
        self.state
    }

    pub fn run(&mut self) -> Result<(), Errno> {
        let events = self.epoll.wait()?;
        for event in events {
            let ep_event = event.events();
            let ep_data = event.data();
            if let true = self.ep_event_err_proc(ep_event, ep_data)? {
                return Ok(());
            }

            if self.alive.connect_fd as u64 == ep_data {
                let buf = self.alive.recv_buf()?;
                if self.alive.is_unmanageable(&buf) {
                    self.alive.manager_time_count = 0;
                    self.state = InitState::Reexec;
                    self.need_reexec = true;
                } else if self.alive.is_alive(&buf) {
                    self.alive.alive_time_count = 0;
                } else {
                    eprintln!("recv buf is invalid! {:?}", buf);
                }
                continue;
            }
            if self.alive.time_fd as u64 == ep_data {
                self.epoll.read(self.alive.time_fd)?;
                self.alive.alive_time_count += 1;
                if self.alive.time_out <= self.alive.alive_time_count {
                    self.state = InitState::Reexec;
                    self.need_reexec = true;
                }
                continue;
            }
            if self.signals.signal_fd as u64 == ep_data {
                let res = self.signals.read_signals()?;
                match res {
                    Some(signal) => {
                        self.run_dispatch_signal(signal)?;
                    }
                    None => println!("None"),
                }
                continue;
            }
        }
        Ok(())
    }

    pub fn unrecover_run(&mut self) -> Result<(), Errno> {
        let events = self.epoll.wait()?;

        for event in events {
            let ep_event = event.events();
            let ep_data = event.data();
            if let true = self.ep_event_err_proc(ep_event, ep_data)? {
                return Ok(());
            }

            if self.alive.connect_fd as u64 == ep_data {
                let buf = self.alive.recv_buf()?;
                if self.alive.is_alive(&buf) {
                    self.alive.manager_time_count = 0;
                    self.alive.alive_time_count = 0;
                }
                continue;
            }
            if self.alive.time_fd as u64 == ep_data {
                self.epoll.read(self.alive.time_fd)?;
                self.alive.alive_time_count += 1;
                if self.alive.time_out <= self.alive.alive_time_count {
                    self.alive.alive_time_count = 0;
                    println!("sysmaster is timeout!");
                }
                continue;
            }
            if self.signals.signal_fd as u64 == ep_data {
                let res = self.signals.read_signals()?;
                match res {
                    Some(signal) => {
                        self.unrecover_dispatch_signal(signal)?;
                    }
                    None => println!("None"),
                }
                continue;
            }
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.epoll.safe_close(self.alive.alive_fd);
        self.alive.alive_fd = INVALID_FD;

        self.epoll.safe_close(self.alive.connect_fd);
        self.alive.connect_fd = INVALID_FD;

        self.epoll.safe_close(self.signals.signal_fd);
        self.signals.signal_fd = INVALID_FD;

        self.epoll.clear();
    }

    fn reexec_manager(&mut self) -> Result<InitState, Errno> {
        self.need_reexec = false;
        self.alive.manager_time_count = 0;
        self.alive.alive_time_count = 0;

        if self.alive.connect_fd >= 0 {
            self.alive.del_connect_epoll()?;
        }

        let res = unsafe {
            libc::kill(
                self.sysmaster_pid.into(),
                libc::SIGRTMIN() + MANAGER_SIG_OFFSET,
            )
        };
        if let Err(err) = Errno::result(res).map(drop) {
            println!(
                "Failed to kill sysmaster:{:?}  err:{:?} change state to unrecover",
                self.sysmaster_pid, err
            );
            self.state = InitState::RunUnRecover;
            return Ok(self.state);
        }

        if self.alive.wait_connect().is_err() {
            self.state = InitState::RunUnRecover;
        }

        Ok(self.state)
    }

    fn ep_event_err_proc(&mut self, ep_flags: EpollFlags, ep_data: u64) -> Result<bool, Errno> {
        if self.epoll.event_is_err(ep_flags) {
            println!("ep_flags:{:?} ep_data:{:?} is invalid!", ep_flags, ep_data);
            // when sysmaster is killed by signal,etc.
            if self.alive.connect_fd as u64 == ep_data {
                self.alive.del_connect_epoll()?;
                self.state = InitState::RunUnRecover;
                return Ok(true);
            } else {
                return Err(Errno::EIO);
            }
        }
        Ok(false)
    }

    fn run_dispatch_signal(&mut self, signal: i32) -> Result<(), Errno> {
        match signal {
            x if x == self.signals.zombie_signal => self.signals.recycle_zombie(),
            x if x == self.signals.restart_signal => self.do_restart(),
            x if x == self.signals.unrecover_signal => self.run_to_unrecover(),
            _ => Ok(()),
        }
    }

    fn unrecover_dispatch_signal(&mut self, signal: i32) -> Result<(), Errno> {
        match signal {
            x if x == self.signals.zombie_signal => self.signals.recycle_zombie(),
            x if x == self.signals.restart_signal => {
                unsafe { libc::kill(self.sysmaster_pid.into(), libc::SIGKILL) };
                self.create_sysmaster()
            }
            _ => Ok(()),
        }
    }

    fn run_to_unrecover(&mut self) -> Result<(), Errno> {
        println!("change run state to unrecover");
        self.state = InitState::RunUnRecover;
        Ok(())
    }

    fn do_restart(&mut self) -> Result<(), Errno> {
        self.state = InitState::Reexec;
        self.need_reexec = true;
        Ok(())
    }

    fn create_sysmaster(&mut self) -> Result<(), Errno> {
        let cmd = &self.cmd.manager_args;
        if cmd.is_empty() {
            println!("cmd is empty!");
            return Err(Errno::EINVAL);
        }

        let res = unsafe { unistd::fork() };
        if let Err(err) = res {
            println!("Failed to create_sysmaster:{:?}", err);
            Err(err)
        } else if let Ok(ForkResult::Parent { child, .. }) = res {
            self.sysmaster_pid = child;
            self.alive.wait_connect()?;
            Ok(())
        } else {
            let mut command = Command::new(&cmd[0]);
            let mut argv = [].to_vec();
            if cmd.len() >= 2 {
                argv = cmd[1..].to_vec();
            }
            command.args(argv);

            let comm = command.env("MANAGER", format!("{}", unsafe { libc::getpid() }));
            let err = comm.exec();
            match err.raw_os_error() {
                Some(e) => {
                    println!("MANAGER exit err:{:?}", e);
                    exit(e);
                }
                None => exit(0),
            }
        }
    }
}
