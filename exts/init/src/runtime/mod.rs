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

mod comm;
mod epoll;
pub mod param;
mod signals;
mod timer;

use comm::{Comm, CommType};
use epoll::Epoll;
use nix::errno::Errno;
use nix::libc;
use nix::sys::epoll::EpollEvent;
use nix::unistd::{self, ForkResult, Pid};
use param::Param;
use signals::Signals;
use std::ffi::CString;
use std::os::unix::io::RawFd;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{exit, Command};
use std::rc::Rc;

use constants::SIG_SWITCH_ROOT_OFFSET;
const INVALID_PID: i32 = -1;
const SYSMASTER_PATH: &str = "/usr/lib/sysmaster/sysmaster";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum InitState {
    Reexec = 0,
    Run = 1,
    Unrecover = 2,
}

pub struct RunTime {
    cmd: Param,
    sysmaster_pid: Pid,
    state: InitState,
    epoll: Rc<Epoll>,
    comm: Comm,
    signals: Signals,
    need_reexec: bool,
    switching: bool,
}

impl RunTime {
    pub fn new(cmd: Param) -> Result<RunTime, Errno> {
        let ep = Epoll::new()?;
        let epoll = Rc::new(ep);
        let comm = Comm::new(&epoll, cmd.init_param.time_wait, cmd.init_param.time_cnt)?;
        let signals = Signals::new(&epoll);

        let mut run_time = RunTime {
            cmd,
            sysmaster_pid: unistd::Pid::from_raw(INVALID_PID),
            state: InitState::Reexec,
            epoll,
            comm,
            signals,
            need_reexec: false,
            switching: false,
        };

        run_time.create_sysmaster()?;
        run_time.signals.create_signals_epoll()?;

        Ok(run_time)
    }

    pub fn state(&self) -> InitState {
        self.state
    }

    pub fn reexec(&mut self) -> Result<(), Errno> {
        if self.need_reexec {
            self.reexec_manager();
        }

        let event = self.epoll.wait_one();
        let fd = event.data() as RawFd;
        match fd {
            _x if self.comm.is_fd(fd) => self.reexec_comm_dispatch(event)?,
            _x if self.signals.is_fd(fd) => self.reexec_signal_dispatch(event)?,
            _ => self.epoll.safe_close(fd),
        }
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Errno> {
        let event = self.epoll.wait_one();
        let fd = event.data() as RawFd;
        match fd {
            _x if self.comm.is_fd(fd) => self.run_comm_dispatch(event)?,
            _x if self.signals.is_fd(fd) => self.run_signal_dispatch(event)?,
            _ => self.epoll.safe_close(fd),
        }
        Ok(())
    }

    pub fn unrecover(&mut self) -> Result<(), Errno> {
        let event = self.epoll.wait_one();
        let fd = event.data() as RawFd;
        match fd {
            _x if self.comm.is_fd(fd) => self.unrecover_comm_dispatch(event),
            _x if self.signals.is_fd(fd) => self.unrecover_signal_dispatch(event)?,
            _ => self.epoll.safe_close(fd),
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.comm.clear();
        self.signals.clear();
        self.epoll.clear();
    }

    pub fn reexec_self(&mut self) {
        self.clear();
        let mut args = Vec::new();
        let mut init_path = CString::new("/usr/bin/init").unwrap();
        if let Some(str) = std::env::args().next() {
            init_path = CString::new(str).unwrap();
            args.push(init_path.clone());
        }
        for str in self.cmd.manager_param.iter() {
            args.push(std::ffi::CString::new(str.to_string()).unwrap());
        }

        let cstr_args = args
            .iter()
            .map(|cstring| cstring.as_c_str())
            .collect::<Vec<_>>();

        if let Err(e) = unistd::execv(&init_path, &cstr_args) {
            eprintln!("execv failed: {e}");
        }
    }

    fn reexec_manager(&mut self) {
        self.need_reexec = false;

        self.comm.finish();

        unsafe { libc::kill(self.sysmaster_pid.into(), libc::SIGABRT) };
    }

    fn reexec_comm_dispatch(&mut self, event: EpollEvent) -> Result<(), Errno> {
        match self.comm.proc(event)? {
            CommType::PipON => self.state = InitState::Run,
            CommType::PipTMOUT => self.need_reexec = true,
            _ => {}
        }
        Ok(())
    }

    fn run_comm_dispatch(&mut self, event: EpollEvent) -> Result<(), Errno> {
        match self.comm.proc(event)? {
            CommType::PipOFF => self.state = InitState::Reexec,
            CommType::PipTMOUT => {
                self.state = InitState::Reexec;
                self.need_reexec = true;
            }
            _ => {}
        }
        Ok(())
    }

    fn unrecover_comm_dispatch(&mut self, event: EpollEvent) {
        _ = self.comm.proc(event);
    }

    fn reexec_signal_dispatch(&mut self, event: EpollEvent) -> Result<(), Errno> {
        if let Some(siginfo) = self.signals.read(event)? {
            match siginfo {
                _x if self.signals.is_zombie(siginfo) => self.do_recycle(),
                _x if self.signals.is_restart(siginfo) => self.do_reexec(),
                _x if self.signals.is_unrecover(siginfo) => self.change_to_unrecover(),
                _ => {}
            }
        }
        Ok(())
    }

    fn run_signal_dispatch(&mut self, event: EpollEvent) -> Result<(), Errno> {
        if let Some(siginfo) = self.signals.read(event)? {
            match siginfo {
                _x if self.signals.is_zombie(siginfo) => self.do_recycle(),
                _x if self.signals.is_restart(siginfo) => self.do_reexec(),
                _x if self.signals.is_switch_root(siginfo) => self.send_switch_root_signal(),
                _ => {}
            }
        }
        Ok(())
    }

    fn unrecover_signal_dispatch(&mut self, event: EpollEvent) -> Result<(), Errno> {
        if let Some(siginfo) = self.signals.read(event)? {
            match siginfo {
                _x if self.signals.is_zombie(siginfo) => {
                    self.signals.recycle_zombie();
                    if self.is_sysmaster(siginfo.ssi_pid as i32) && self.switching {
                        self.reexec_self()
                    }
                }
                _x if self.signals.is_restart(siginfo) => self.do_recreate(),
                _ => {}
            }
        }
        Ok(())
    }

    fn change_to_unrecover(&mut self) {
        println!("change run state to unrecover");
        self.state = InitState::Unrecover;
        self.signals.recycle_zombie();
    }

    fn do_reexec(&mut self) {
        self.need_reexec = true;
        self.state = InitState::Reexec;
    }

    fn do_recreate(&mut self) {
        self.comm.finish();
        unsafe { libc::kill(self.sysmaster_pid.into(), libc::SIGKILL) };
        if let Err(err) = self.create_sysmaster() {
            eprintln!("Failed to create_sysmaster{:?}", err);
        }
    }

    fn do_recycle(&mut self) {
        self.signals.recycle_zombie();
    }

    fn create_sysmaster(&mut self) -> Result<(), Errno> {
        if !Path::new(SYSMASTER_PATH).exists() {
            eprintln!("{:?} does not exest!", SYSMASTER_PATH);
            return Err(Errno::ENOENT);
        }

        let res = unsafe { unistd::fork() };
        if let Err(err) = res {
            eprintln!("Failed to create_sysmaster:{:?}", err);
            Err(err)
        } else if let Ok(ForkResult::Parent { child, .. }) = res {
            self.sysmaster_pid = child;
            Ok(())
        } else {
            let mut command = Command::new(SYSMASTER_PATH);
            command.args(self.cmd.manager_param.to_vec());

            let comm = command.env("MANAGER", format!("{}", unsafe { libc::getpid() }));
            let err = comm.exec();
            match err.raw_os_error() {
                Some(e) => {
                    eprintln!("MANAGER exit err:{:?}", e);
                    exit(e);
                }
                None => exit(0),
            }
        }
    }

    fn send_switch_root_signal(&mut self) {
        let res = unsafe {
            libc::kill(
                self.sysmaster_pid.into(),
                libc::SIGRTMIN() + SIG_SWITCH_ROOT_OFFSET,
            )
        };
        if let Err(err) = Errno::result(res).map(drop) {
            eprintln!(
                "Failed to send sysmaster switch-root signal:{:?}  err:{:?} change state to switch_root",
                self.sysmaster_pid, err
            );
        }
        self.state = InitState::Unrecover;
        self.switching = true;
    }

    fn is_sysmaster(&self, pid: i32) -> bool {
        self.sysmaster_pid.as_raw() == pid
    }
}
