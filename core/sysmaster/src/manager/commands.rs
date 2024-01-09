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

use basic::do_entry_log;
use basic::fs::{self, is_symlink};
use cmdproto::proto::execute::ExecuterAction;
use cmdproto::proto::ProstServerStream;
use core::rel::{ReliLastFrame, Reliability};
use event::{EventType, Events, Source};
use nix::sys::{socket, stat};
use std::os::unix::io::RawFd;
use std::path::Path;
use std::{os::unix::prelude::AsRawFd, rc::Rc};

use constants::PRIVATE_SOCKET;

pub(super) struct Commands<T> {
    // associated objects
    reli: Rc<Reliability>,
    command_action: Rc<T>,

    // owned objects
    socket_fd: i32,
}

impl<T> Commands<T>
where
    T: ExecuterAction,
{
    pub(super) fn new(relir: &Rc<Reliability>, comm_action: T) -> Self {
        /* The socket is used to communicate with sctl, panic if any of the following steps fail. */
        let sctl_socket_path = Path::new(PRIVATE_SOCKET);
        let run_sysmaster = sctl_socket_path.parent().unwrap();
        if run_sysmaster.exists() {
            let _ = fs::chmod("/run/sysmaster", 0o755);
        }
        /* remove the old socket if it exists */
        if sctl_socket_path.exists() && !is_symlink(sctl_socket_path) {
            do_entry_log!(std::fs::remove_file, sctl_socket_path, "remove");
        }
        let sctl_socket_addr = socket::UnixAddr::new(Path::new(PRIVATE_SOCKET)).unwrap();
        let socket_fd = socket::socket(
            socket::AddressFamily::Unix,
            socket::SockType::Stream,
            socket::SockFlag::SOCK_CLOEXEC | socket::SockFlag::SOCK_NONBLOCK,
            None,
        )
        .unwrap();
        /* set SO_PASSCRED, we need it to check whether sctl is running under root */
        basic::socket::set_pass_cred(socket_fd, true).unwrap();
        /* create the socket with mode 666 */
        let old_mask = stat::umask(stat::Mode::from_bits_truncate(!0o666));
        match socket::bind(socket_fd, &sctl_socket_addr) {
            Err(e) => log::error!("Failed to bind {:?}: {}", sctl_socket_addr, e),
            Ok(_) => log::debug!("Successfully bind {:?}", sctl_socket_addr),
        }
        /* restore our umask */
        let _ = stat::umask(old_mask);
        /* Allow at most 4096 incoming connections can queue */
        let _ = socket::listen(socket_fd, 4096);
        Commands {
            reli: Rc::clone(relir),
            command_action: Rc::new(comm_action),
            socket_fd,
        }
    }

    pub fn dispatch_commands(&self) -> i32 {
        let client = match socket::accept(self.socket_fd) {
            Err(e) => {
                log::error!("Failed to accept connection: {}, ignoring.", e);
                return 0;
            }
            Ok(v) => v,
        };
        let cred = match socket::getsockopt(client, socket::sockopt::PeerCredentials) {
            Err(e) => {
                log::error!(
                    "Failed to get the credentials of the connection: {}, refuse any request.",
                    e
                );
                None
            }
            Ok(v) => Some(v),
        };
        let dispatch = ProstServerStream::new(client, self.command_action.clone(), cred);
        match dispatch.process() {
            Ok(_) => (),
            Err(e) => log::error!("Commands failed: {:?}", e),
        }
        basic::fd::close(client);
        0
    }
}

impl<T> Source for Commands<T>
where
    T: ExecuterAction,
{
    fn event_type(&self) -> EventType {
        EventType::Io
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn dispatch(&self, _e: &Events) -> i32 {
        self.reli.set_last_frame1(ReliLastFrame::CmdOp as u32);
        /* NOTE: we must call clear_last_frame before return. If we don't do this, the allocated
         * vector "frame" in reli.set_frame will be leaked, and become larger and larger until
         * triggers a panic. */
        let ret = self.dispatch_commands();
        self.reli.clear_last_frame();
        ret
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn fd(&self) -> RawFd {
        self.socket_fd.as_raw_fd()
    }

    fn priority(&self) -> i8 {
        0i8
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{Read, Write};
    use std::os::unix::prelude::FromRawFd;
    use std::process::exit;
    use std::sync::{Arc, Mutex};
    use std::{os::unix::net::UnixStream, rc::Rc};

    use cmdproto::error::Result;
    use cmdproto::proto::{execute::ExecuterAction, unit_comm};
    use cmdproto::proto::{CommandRequest, ProstClientStream};
    use constants::PRIVATE_SOCKET;
    use core::rel::{ReliConf, Reliability};
    use event::{EventState, Events};
    use nix::unistd;

    use crate::manager::RELI_HISTORY_MAX_DBS;

    use super::Commands;

    struct TestExecAction {}

    impl ExecuterAction for TestExecAction {
        type Error = nix::Error;
        type Status = nix::Error;

        fn start(&self, _unit_name: &str) -> Result<(), Self::Error> {
            Err(nix::Error::ENOENT)
        }

        fn stop(&self, _unit_name: &str) -> Result<(), Self::Error> {
            Ok(())
        }

        fn restart(&self, _unit_name: &str) -> Result<(), Self::Error> {
            Ok(())
        }

        fn reload(&self, _unit_name: &str) -> Result<(), Self::Error> {
            Ok(())
        }

        fn isolate(&self, _unit_name: &str) -> Result<(), Self::Error> {
            Ok(())
        }

        fn reset_failed(&self, _unit_name: &str) -> Result<(), Self::Error> {
            Ok(())
        }

        fn status(&self, _unit_name: &str) -> Result<Self::Status, Self::Error> {
            Ok(nix::Error::EINVAL)
        }

        fn list_units(&self) -> Result<String, Self::Error> {
            Ok(String::new())
        }

        fn suspend(&self) -> Result<i32, Self::Error> {
            Ok(0)
        }

        fn poweroff(&self) -> Result<i32, Self::Error> {
            Ok(0)
        }

        fn reboot(&self) -> Result<i32, Self::Error> {
            Ok(0)
        }

        fn halt(&self) -> Result<i32, Self::Error> {
            Ok(0)
        }

        fn disable(&self, _unit_name: &str) -> Result<(), Self::Error> {
            Ok(())
        }

        fn enable(&self, _unit_name: &str) -> Result<(), Self::Error> {
            Ok(())
        }

        fn mask(&self, _unit_name: &str) -> Result<(), Self::Error> {
            Ok(())
        }

        fn unmask(&self, _unit_name: &str) -> Result<(), Self::Error> {
            Ok(())
        }

        fn daemon_reload(&self) {}

        fn daemon_reexec(&self) {}

        fn switch_root(&self, _init: &[String]) -> Result<(), Self::Error> {
            Ok(())
        }

        fn start_transient_unit(
            &self,
            _job_mode: &str,
            _unit_config: &cmdproto::proto::transient_unit_comm::UnitConfig,
            _aux_units: &[cmdproto::proto::transient_unit_comm::UnitConfig],
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn test_sctl_socket_process() {
        if !nix::unistd::getuid().is_root() {
            println!(
                "We will create a socket under '/run', so we must be root to run this testcase."
            );
            return;
        }

        let pipe_msg = unistd::pipe().unwrap();
        let pipe_sync = unistd::pipe().unwrap();

        let ret = unsafe { unistd::fork() }.unwrap();
        if ret.is_parent() {
            unistd::close(pipe_msg.1).unwrap();
            unistd::close(pipe_sync.0).unwrap();

            let exec_action = TestExecAction {};
            let reli = Rc::new(Reliability::new(
                ReliConf::new().set_max_dbs(RELI_HISTORY_MAX_DBS),
            ));
            /* This will remove the /run/sysmaster/sctl on the compiling environment. */
            let command = Rc::new(Commands::new(&reli, exec_action));
            let e = Events::new().unwrap();
            e.add_source(command.clone()).unwrap();
            e.set_enabled(command, EventState::On).unwrap();

            /* Write something to trigger the child process continue */
            let mut file = unsafe { File::from_raw_fd(pipe_sync.1) };
            write!(&mut file, "go!").unwrap();
            unistd::close(pipe_sync.1).unwrap();

            /* We have 2 events to dispatch, but are not sure if we can get these events after waiting
             * 300ms, 400ms. So we wait one more time to make sure all events are dispatched. */
            e.run(300).unwrap();
            e.run(400).unwrap();
            e.run(300).unwrap();

            /* Got the result from child use pipe_msg */
            let mut file = unsafe { File::from_raw_fd(pipe_msg.0) };
            let mut res = String::new();
            file.read_to_string(&mut res).unwrap();
            assert_eq!(res, "Ok");
            unistd::close(pipe_msg.0).unwrap();
        } else {
            /* child only writes message */
            unistd::close(pipe_msg.0).unwrap();
            unistd::close(pipe_sync.1).unwrap();

            /* Wait until the parent process tells "go!" */
            let mut file = unsafe { File::from_raw_fd(pipe_sync.0) };
            let mut buf = String::new();
            file.read_to_string(&mut buf).unwrap();
            assert_eq!(buf, "go!");
            unistd::close(pipe_sync.0).unwrap();

            /* 1. Start a service as root. */
            let stream = UnixStream::connect(PRIVATE_SOCKET).unwrap();
            let mut client = ProstClientStream::new(stream);

            let req = CommandRequest::new_unitcomm(
                unit_comm::Action::Start,
                vec!["foo.service".to_string()],
            );
            let data = client.execute(req).unwrap();
            if !data
                .message
                .eq("Failed to start foo.service: ENOENT: No such file or directory")
            {
                return;
            }
            /* 2. Start a service as an unprivileged user. */
            let _ = nix::unistd::setuid(nix::unistd::Uid::from_raw(1000));
            let stream = UnixStream::connect(PRIVATE_SOCKET).unwrap();
            let mut client = ProstClientStream::new(stream);

            let req = CommandRequest::new_unitcomm(
                unit_comm::Action::Start,
                vec!["foo.service".to_string()],
            );
            let data = client.execute(req).unwrap();
            if !data
                .message
                .eq("Failed to execute your command: Operation not permitted.")
            {
                return;
            }

            /* Everything goes well, tell the parent "Ok" */
            let mut file = unsafe { File::from_raw_fd(pipe_msg.1) };
            write!(&mut file, "Ok").unwrap();
            unistd::close(pipe_msg.1).unwrap();
            exit(0);
        }
    }

    #[test]
    fn test_sctl_socket_thread() {
        if !nix::unistd::getuid().is_root() {
            println!(
                "We will create a socket under '/run', so we must be root to run this testcase."
            );
            return;
        }

        let mutex_main = Arc::new(Mutex::new(0));
        let mutex_child = Arc::clone(&mutex_main);
        let temp = mutex_main.lock().unwrap();

        std::thread::spawn(move || {
            /* Wait until the main thread are ready to process our request. */
            let mut test_ok = mutex_child.lock().unwrap();
            /* 1. Start a service as root. */
            let stream = UnixStream::connect(PRIVATE_SOCKET).unwrap();
            let mut client = ProstClientStream::new(stream);

            let req = CommandRequest::new_unitcomm(
                unit_comm::Action::Start,
                vec!["foo.service".to_string()],
            );
            let data = client.execute(req).unwrap();
            if !data
                .message
                .eq("Failed to start foo.service: ENOENT: No such file or directory")
            {
                return;
            }

            *test_ok = 1;
        });

        let exec_action = TestExecAction {};
        let reli = Rc::new(Reliability::new(
            ReliConf::new().set_max_dbs(RELI_HISTORY_MAX_DBS),
        ));
        /* This will remove the /run/sysmaster/sctl on the compiling environment. */
        let command = Rc::new(Commands::new(&reli, exec_action));
        let e = Events::new().unwrap();
        e.add_source(command.clone()).unwrap();
        e.set_enabled(command.clone(), EventState::On).unwrap();
        /* Drop temp, let the child thread go. */
        drop(temp);
        /* We have 2 events to dispatch, but are not sure if we can get these events after waiting
         * 300ms, 400ms. So we wait one more time to make sure all events are dispatched. */
        e.run(300).unwrap();
        e.run(400).unwrap();
        e.run(300).unwrap();
        let test_ok = mutex_main.lock().unwrap();
        /* The value should be set to 1 in the child process, if everything works well. */
        assert_eq!(*test_ok, 1);
        e.del_source(command).unwrap();
    }
}
