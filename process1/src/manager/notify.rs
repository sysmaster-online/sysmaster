use std::{
    cell::RefCell, collections::HashMap, fs, io::IoSliceMut, os::unix::prelude::RawFd,
    path::PathBuf, rc::Rc,
};

use event::{EventType, Events, Source};
use libc;
use nix::{
    cmsg_space,
    errno::Errno,
    sys::socket::{self, sockopt, AddressFamily, MsgFlags, SockFlag, SockType, UnixAddr},
};
use utils::Error;

use super::{manager_config::ManagerConfig, Manager};

const NOTIFY_SOCKET: &str = "/run/process1/notify";

pub(super) struct NotifyEvent {
    fd: RefCell<i32>,
    manager: Rc<Manager>,
    config: Rc<ManagerConfig>,
}

impl NotifyEvent {
    pub(super) fn new(mr: &Rc<Manager>, configm: &Rc<ManagerConfig>) -> NotifyEvent {
        NotifyEvent {
            fd: RefCell::new(-1),
            manager: mr.clone(),
            config: configm.clone(),
        }
    }

    pub(super) fn fd(&self) -> RawFd {
        *self.fd.borrow_mut() as RawFd
    }

    pub(super) fn open_socket(&self) -> Result<(), Errno> {
        let fd = socket::socket(
            AddressFamily::Unix,
            SockType::Datagram,
            SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK,
            None,
        )?;

        log::debug!("notify listend fd is: {}", fd);
        let sock_path = PathBuf::from(NOTIFY_SOCKET);

        self.config.set_notify_sock(sock_path.clone());
        let parent_path = sock_path.as_path().parent();

        fs::create_dir_all(parent_path.unwrap()).map_err(|_e| Errno::EINVAL)?;

        let unix_addr = UnixAddr::new(&sock_path)?;

        if let Err(e) = nix::unistd::unlink(&sock_path) {
            log::warn!("unlink path failed: {:?}, error: {}", sock_path, e);
        }

        socket::bind(fd, &unix_addr)?;

        socket::setsockopt(fd, sockopt::PassCred, &true)?;

        log::debug!("set event fd is: {}", fd);
        *self.fd.borrow_mut() = fd;

        Ok(())
    }
}

impl Source for NotifyEvent {
    fn fd(&self) -> RawFd {
        self.fd()
    }

    fn event_type(&self) -> EventType {
        EventType::Io
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn dispatch(&self, _e: &Events) -> Result<i32, Error> {
        log::debug!("begin to dispatch notify event");
        let mut received_fds = Vec::new();
        let mut received_cred = None;
        let mut buffer = [0u8; 4096];
        let mut iov = [IoSliceMut::new(&mut buffer)];
        let mut space = cmsg_space!(libc::ucred, RawFd);

        let msgs = socket::recvmsg::<()>(
            self.fd(),
            &mut iov,
            Some(&mut space),
            MsgFlags::MSG_DONTWAIT | MsgFlags::MSG_CMSG_CLOEXEC | MsgFlags::MSG_TRUNC,
        )
        .map_err(|_e| Error::Other {
            msg: "failed to receive notify message",
        })?;

        for msg in msgs.cmsgs() {
            match msg {
                socket::ControlMessageOwned::ScmRights(fds) => {
                    received_fds = fds;
                }
                socket::ControlMessageOwned::ScmCredentials(cred) => {
                    received_cred = Some(cred);
                }
                _ => {
                    log::debug!("unexpected control message");
                }
            }
        }

        if received_cred.is_none() || received_cred.unwrap().pid() < 0 {
            log::error!("there is no credentials in the received notify message");
            return Err(Error::Other {
                msg: "there is no credentials in the received notify message",
            });
        }

        let contents = String::from_utf8(buffer.to_vec()).map_err(Error::from)?;
        let mut messages = HashMap::new();

        for line in contents.lines() {
            let content: Vec<&str> = line
                .split('=')
                .map(|s| s.trim_end_matches(char::from(0)))
                .collect();
            if content.len() != 2 {
                continue;
            }

            messages.insert(content[0], content[1].trim_end());
        }

        log::debug!(
            "read ucred from notify listening socket: {:?}",
            received_cred.unwrap()
        );
        log::debug!("read messages from notify listening socket: {:?}", messages);

        self.manager
            .notify_message(&received_cred.unwrap(), &messages, &received_fds)?;

        Ok(0)
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
