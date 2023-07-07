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

use basic::{do_entry_log, socket_util};
use cmdproto::proto::execute::ExecuterAction;
use cmdproto::proto::ProstServerStream;
use event::{EventType, Events, Source};
use nix::sys::{socket, stat};
use std::os::unix::io::RawFd;
use std::path::Path;
use std::{os::unix::prelude::AsRawFd, rc::Rc};
use sysmaster::rel::{ReliLastFrame, Reliability};

use constants::SCTL_SOCKET;

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
        let sctl_socket_path = Path::new(SCTL_SOCKET);
        /* remove the old socket if it exists */
        if sctl_socket_path.exists() && !sctl_socket_path.is_symlink() {
            do_entry_log!(std::fs::remove_file, sctl_socket_path, "remove");
        }
        let sctl_socket_addr = socket::UnixAddr::new(Path::new(SCTL_SOCKET)).unwrap();
        let socket_fd = socket::socket(
            socket::AddressFamily::Unix,
            socket::SockType::Stream,
            socket::SockFlag::SOCK_CLOEXEC | socket::SockFlag::SOCK_NONBLOCK,
            None,
        )
        .unwrap();
        /* set SO_PASSCRED, we need it to check whether sctl is running under root */
        socket_util::set_pass_cred(socket_fd, true).unwrap();
        /* create the socket with mode 666 */
        let old_mask = stat::umask(stat::Mode::from_bits_truncate(!0o666));
        match socket::bind(socket_fd, &sctl_socket_addr) {
            Err(e) => log::error!("Failed to bind {sctl_socket_addr:?}: {e}"),
            Ok(_) => log::debug!("Successfully bind {sctl_socket_addr:?}"),
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
        basic::fd_util::close(client);
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
