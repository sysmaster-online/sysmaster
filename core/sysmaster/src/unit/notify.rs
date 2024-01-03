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

use super::datastore::UnitDb;
use super::entry::UnitX;
use super::rentry::UnitRe;
use crate::job::JobManager;
use basic::fd;
use core::error::*;
use core::rel::{ReStation, ReliLastFrame, Reliability};
use event::{EventState, EventType, Events, Source};
use nix::cmsg_space;
use nix::errno::Errno;
use nix::sys::socket::{
    self, sockopt, AddressFamily, MsgFlags, RecvMsg, SockFlag, SockType, UnixAddr, UnixCredentials,
};
use nix::sys::stat::{self, Mode};
use nix::unistd::Pid;
use std::{
    cell::RefCell, collections::HashMap, fs, io::IoSliceMut, os::unix::prelude::RawFd,
    path::PathBuf, rc::Rc,
};

const NOTIFY_SOCKET: &str = "/run/sysmaster/notify";

pub(super) struct NotifyManager {
    // associated objects
    events: Rc<Events>,
    jm: Rc<JobManager>,

    // owned objects
    config: Rc<NotifyConfig>,
    notify: Rc<Notify>,
}

impl ReStation for NotifyManager {
    // input: do nothing

    // compensate
    fn db_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        if let Some(unit_id) = lunit {
            // merge to trigger
            self.jm.rentry_trigger_merge(unit_id, true);
        }
    }

    fn do_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        if let Some(unit_id) = lunit {
            // re-run
            self.jm.trigger_unit(&unit_id.to_string());
        }
    }

    // data
    fn db_map(&self, _reload: bool) {
        self.notify.db_map();
    }

    fn db_insert(&self) {
        self.notify.db_insert();
    }

    // reload
    fn register_ex(&self) {
        // open socket
        self.notify.open_socket().unwrap();
        self.db_update();

        // event
        let notify = Rc::clone(&self.notify);
        self.events.add_source(notify).unwrap();
        let notify = Rc::clone(&self.notify);
        self.events.set_enabled(notify, EventState::On).unwrap();
    }
}

impl NotifyManager {
    pub(super) fn new(
        eventr: &Rc<Events>,
        relir: &Rc<Reliability>,
        rentryr: &Rc<UnitRe>,
        dbr: &Rc<UnitDb>,
        jmr: &Rc<JobManager>,
    ) -> NotifyManager {
        let notify_config = Rc::new(NotifyConfig::new());
        notify_config.set_notify_sock(PathBuf::from(NOTIFY_SOCKET));
        let _notify = Rc::new(Notify::new(relir, rentryr, dbr, &notify_config));
        let nm = NotifyManager {
            events: Rc::clone(eventr),
            jm: Rc::clone(jmr),
            config: Rc::clone(&notify_config),
            notify: Rc::clone(&_notify),
        };

        nm.db_insert();
        nm
    }

    pub(super) fn notify_sock(&self) -> Option<PathBuf> {
        self.config.notify_sock()
    }
}

use constants::INVALID_FD;
const NOTIFY_INVALID_PID: libc::pid_t = -1;

struct Notify {
    // associated objects
    reli: Rc<Reliability>,
    rentry: Rc<UnitRe>,
    db: Rc<UnitDb>,
    config: Rc<NotifyConfig>,

    // owned objects
    fd: RefCell<i32>,
}

impl Notify {
    pub(self) fn new(
        relir: &Rc<Reliability>,
        rentryr: &Rc<UnitRe>,
        dbr: &Rc<UnitDb>,
        configr: &Rc<NotifyConfig>,
    ) -> Notify {
        Notify {
            reli: Rc::clone(relir),
            rentry: Rc::clone(rentryr),
            db: Rc::clone(dbr),
            config: Rc::clone(configr),
            fd: RefCell::new(INVALID_FD),
        }
    }

    pub(self) fn db_map(&self) {
        if let Some(fd) = self.rentry.notify_get() {
            *self.fd.borrow_mut() = self.reli.fd_take(fd);
        }
    }

    // process reentrant
    pub(super) fn open_socket(&self) -> Result<(), Errno> {
        let sock_path = self.config.notify_sock().unwrap();

        // process reentrant protection
        if self.rawfd() >= 0 {
            // debug: process reentrant
            return Ok(());
        }

        let fd = socket::socket(
            AddressFamily::Unix,
            SockType::Datagram,
            SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK,
            None,
        )?;

        log::debug!("notify listend fd is: {}", fd);
        let parent_path = sock_path.as_path().parent();
        fs::create_dir_all(parent_path.unwrap()).map_err(|_e| Errno::EINVAL)?;

        let unix_addr = UnixAddr::new(&sock_path)?;
        if let Err(e) = nix::unistd::unlink(&sock_path) {
            log::warn!("unlink path failed: {:?}, error: {}", sock_path, e);
        }

        // create '/run/sysmaster/notify' with mode 666
        let old_mask = stat::umask(Mode::from_bits_truncate(!0o666));
        let ret = socket::bind(fd, &unix_addr);
        let _ = stat::umask(old_mask);
        if let Err(e) = ret {
            log::error!("Failed to bind socket {:?}: {}", sock_path, e);
            return Err(e);
        }
        socket::setsockopt(fd, sockopt::PassCred, &true)?;
        if let Err(e) = basic::socket::set_receive_buffer(fd, 1024 * 1014 * 8) {
            log::error!("Failed to set the notify socket receive buffer: {}", e);
        }

        log::debug!("set event fd is: {}", fd);
        self.reli.fd_cloexec(fd, false)?;
        *self.fd.borrow_mut() = fd;

        Ok(())
    }

    fn notify_dispatch(&self) -> Result<i32> {
        let flags = MsgFlags::MSG_DONTWAIT | MsgFlags::MSG_CMSG_CLOEXEC | MsgFlags::MSG_TRUNC;

        // peek
        let pid = notify_peek_pid(self.rawfd(), flags)?;
        //  record + pop + action
        let ret;
        if let Some(unit) = self.db.get_unit_by_pid(Pid::from_raw(pid)) {
            self.reli.set_last_unit(&unit.id());
            ret = self.notify_recv_dispatch(flags, pid, Some(unit));
            self.reli.clear_last_unit();
        } else {
            ret = self.notify_recv_dispatch(flags, pid, None);
        }

        ret
    }

    fn notify_recv_dispatch(
        &self,
        flags: MsgFlags,
        pid: libc::pid_t,
        unit: Option<Rc<UnitX>>,
    ) -> Result<i32> {
        let mut buffer = [0u8; 4096];
        let mut iov = [IoSliceMut::new(&mut buffer)];
        let mut space = cmsg_space!(libc::ucred, RawFd);

        // pop
        let msgs = socket::recvmsg::<()>(self.rawfd(), &mut iov, Some(&mut space), flags)
            .context(NixSnafu)?;

        if msgs.flags.contains(MsgFlags::MSG_CTRUNC) {
            return Err(Error::Nix {
                source: nix::Error::EXFULL,
            });
        }

        // check: peek == pop
        let (received_cred, received_fds) = notify_trans_recvmsg(&msgs);
        if get_pid_from_cred(&received_cred) != pid {
            log::error!("the received notify message has been destroyed");
            return Err(Error::Other {
                msg: "the received notify message has been destroyed".to_string(),
            });
        }

        // build input
        let ucred = received_cred.unwrap();
        let contents = String::from_utf8(buffer.to_vec()).unwrap();
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
        log::debug!("[notify] ucred: {:?}, messages: {:?}", &ucred, messages,);

        // action
        if let Some(u) = unit {
            log::debug!("[notify] unit: {:?}", u.id());
            let _ = u.notify_message(&ucred, &messages, received_fds);
        }

        Ok(0)
    }

    fn rawfd(&self) -> RawFd {
        *self.fd.borrow() as RawFd
    }

    fn db_insert(&self) {
        let fd = self.rawfd();
        self.rentry.notify_insert(fd);
    }
}

fn notify_peek_pid(fd: RawFd, flags: MsgFlags) -> Result<libc::pid_t> {
    let mut buffer = [0u8; 4096];
    let mut iov = [IoSliceMut::new(&mut buffer)];
    let mut space = cmsg_space!(libc::ucred, RawFd);

    // peek
    let peek_flags = flags | MsgFlags::MSG_PEEK;
    let msgs =
        socket::recvmsg::<()>(fd, &mut iov, Some(&mut space), peek_flags).context(NixSnafu)?;

    if msgs.flags.contains(MsgFlags::MSG_CTRUNC) {
        return Err(Error::Nix {
            source: nix::Error::EXFULL,
        });
    }

    // get message information
    let (received_cred, received_fds) = notify_trans_recvmsg(&msgs);
    for fd in received_fds.iter() {
        fd::close(*fd);
    }

    // check
    let pid = get_pid_from_cred(&received_cred);
    if pid < 0 {
        log::error!("there is no credentials in the received notify message");
        return Err(Error::Other {
            msg: "no credentials in the received notify message".to_string(),
        });
    }

    Ok(pid)
}

fn notify_trans_recvmsg(msgs: &RecvMsg<()>) -> (Option<UnixCredentials>, Vec<i32>) {
    let mut received_fds = Vec::new();
    let mut received_cred = None;

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

    (received_cred, received_fds)
}

fn get_pid_from_cred(cred: &Option<UnixCredentials>) -> libc::pid_t {
    match cred {
        Some(c) => c.pid(),
        None => NOTIFY_INVALID_PID,
    }
}

impl Source for Notify {
    fn fd(&self) -> RawFd {
        self.rawfd()
    }

    fn event_type(&self) -> EventType {
        EventType::Io
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn priority(&self) -> i8 {
        -8i8
    }

    fn dispatch(&self, _e: &Events) -> i32 {
        self.reli.set_last_frame1(ReliLastFrame::Notify as u32);
        let ret = self.notify_dispatch();
        self.reli.clear_last_frame();

        ret.unwrap_or(-1)
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

struct NotifyConfig {
    data: RefCell<NotifyConfigData>,
}

impl NotifyConfig {
    pub(super) fn new() -> NotifyConfig {
        NotifyConfig {
            data: RefCell::new(NotifyConfigData::new()),
        }
    }

    pub(super) fn set_notify_sock(&self, socket: PathBuf) {
        self.data.borrow_mut().set_notify_sock(socket)
    }

    pub(super) fn notify_sock(&self) -> Option<PathBuf> {
        self.data.borrow().notify_sock()
    }
}

pub(self) struct NotifyConfigData {
    notify_sock: Option<PathBuf>,
}

impl NotifyConfigData {
    fn new() -> NotifyConfigData {
        NotifyConfigData { notify_sock: None }
    }

    pub(self) fn set_notify_sock(&mut self, socket: PathBuf) {
        self.notify_sock = Some(socket);
    }

    pub(super) fn notify_sock(&self) -> Option<PathBuf> {
        self.notify_sock.as_ref().cloned()
    }
}
