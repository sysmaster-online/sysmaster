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
use std::io::ErrorKind;
use std::os::unix::io::RawFd;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{exit, Command};
use std::rc::Rc;

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

    fn reexec_custom_init(&mut self) -> bool {
        let paras = match std::fs::read(constants::INIT_PARA_PATH) {
            Err(e) => {
                if e.kind() != ErrorKind::NotFound {
                    eprintln!("Failed to read init para file, reexec self init: {}", e);
                }
                return false;
            }
            Ok(paras) => paras,
        };

        let paras: Vec<String> = match std::str::from_utf8(&paras) {
            Ok(str) => str.split('\n').map(|para| para.to_string()).collect(),
            Err(_) => {
                return false;
            }
        };

        if paras.is_empty() {
            return false;
        }

        self.exec(&paras[0], &paras)
    }

    fn exec(&mut self, init_path: &str, args: &[String]) -> bool {
        let cstr_args = args
            .iter()
            .map(|str| std::ffi::CString::new(str.clone()).unwrap())
            .collect::<Vec<_>>();

        if let Err(e) = unistd::execv(&CString::new(init_path).unwrap(), &cstr_args) {
            eprintln!("execv {init_path} failed: {e}");
            return false;
        }
        true
    }

    fn switch_root_run(&mut self) {
        self.clear();
        if !self.reexec_custom_init() {
            self.reexec_self_init();
        }
    }

    fn reexec_self_init(&mut self) {
        let mut args = Vec::new();
        let mut init_path = String::new();

        for str in std::env::args() {
            if init_path.is_empty() {
                init_path = str.clone();
            }
            args.push(str);
        }
        self.exec(&init_path, &args);
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
                _x if self.signals.is_switch_root(siginfo)
                    && self.is_sysmaster(siginfo.ssi_pid.try_into().unwrap()) =>
                {
                    self.switch_root_run()
                }
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

    fn is_sysmaster(&self, pid: i32) -> bool {
        self.sysmaster_pid.as_raw() == pid
    }
}
