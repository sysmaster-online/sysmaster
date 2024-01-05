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

//! implement the management of configured path monitor
//!

use crate::error::*;
use basic::fd::close;
use basic::{errno_is_transient, IN_SET};
use constants::INVALID_FD;
use libc::{
    IN_ATTRIB, IN_CLOSE_WRITE, IN_CREATE, IN_DELETE, IN_DELETE_SELF, IN_MODIFY, IN_MOVED_FROM,
    IN_MOVED_TO, IN_MOVE_SELF,
};
use nix::errno::Errno;
use nix::sys::inotify::AddWatchFlags;
use nix::sys::inotify::WatchDescriptor;
use nix::sys::inotify::{InitFlags, Inotify};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fmt;
use std::os::unix::prelude::{AsRawFd, FromRawFd, RawFd};
use std::path::PathBuf;

/// Path type
#[allow(missing_docs)]
#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum PathType {
    Exists,
    ExistsGlob,
    DirectoryNotEmpty,
    Changed,
    Modified,
    TypeMax,
}

static FLAGS_TABLE: [u32; PathType::TypeMax as usize] = [
    IN_DELETE_SELF | IN_MOVE_SELF | IN_ATTRIB,
    IN_DELETE_SELF | IN_MOVE_SELF | IN_ATTRIB,
    IN_DELETE_SELF | IN_MOVE_SELF | IN_ATTRIB | IN_CREATE | IN_MOVED_TO,
    IN_DELETE_SELF
        | IN_MOVE_SELF
        | IN_ATTRIB
        | IN_CLOSE_WRITE
        | IN_CREATE
        | IN_DELETE
        | IN_MOVED_FROM
        | IN_MOVED_TO,
    IN_DELETE_SELF
        | IN_MOVE_SELF
        | IN_ATTRIB
        | IN_CLOSE_WRITE
        | IN_CREATE
        | IN_DELETE
        | IN_MOVED_FROM
        | IN_MOVED_TO
        | IN_MODIFY,
];

/// Path property configuration
pub struct PathSpec {
    path: PathBuf,
    p_type: PathType,
    inotify_fd: RefCell<RawFd>,
    primary_wd: RefCell<Option<WatchDescriptor>>,
    previous_exists: RefCell<bool>,
}

impl PathSpec {
    /// Create PathSpec
    pub fn new(path: PathBuf, p_type: PathType) -> Self {
        PathSpec {
            path,
            p_type,
            inotify_fd: RefCell::new(INVALID_FD),
            primary_wd: RefCell::new(None),
            previous_exists: RefCell::new(false),
        }
    }

    /// get file path
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    /// get PathType
    pub fn path_type(&self) -> PathType {
        self.p_type
    }

    /// get inotify_fd
    pub fn inotify_fd(&self) -> RawFd {
        *self.inotify_fd.borrow()
    }

    /// set inotify_fd
    pub fn set_inotify_fd(&self, inotify_fd: RawFd) {
        *self.inotify_fd.borrow_mut() = inotify_fd;
    }

    /// get primary_wd
    pub fn primary_wd(&self) -> Option<WatchDescriptor> {
        *self.primary_wd.borrow()
    }

    /// set primary_wd
    pub fn set_primary_wd(&self, primary_wd: Option<WatchDescriptor>) {
        *self.primary_wd.borrow_mut() = primary_wd;
    }

    /// get previous_exists
    pub fn previous_exists(&self) -> bool {
        *self.previous_exists.borrow()
    }

    /// set previous_exists
    pub fn set_previous_exists(&self, previous_exists: bool) {
        *self.previous_exists.borrow_mut() = previous_exists;
    }

    /// start file watch
    pub fn watch(&self) -> Result<()> {
        self.unwatch();

        let inotify =
            Inotify::init(InitFlags::IN_NONBLOCK | InitFlags::IN_CLOEXEC).map_err(|_e| {
                Error::Other {
                    msg: "create initofy fd err".to_string(),
                }
            })?;
        self.set_inotify_fd(inotify.as_raw_fd());

        let mut ansters = self.path.as_path().ancestors().collect::<Vec<_>>();
        ansters.reverse();

        let mut flags: AddWatchFlags;
        let mut wd: Option<WatchDescriptor> = None;

        let mut exists = false;
        for anster in ansters {
            let mut incomplete = false;

            flags = if anster != self.path.as_path() {
                AddWatchFlags::IN_DELETE_SELF
                    | AddWatchFlags::IN_MOVE_SELF
                    | AddWatchFlags::IN_ATTRIB
                    | AddWatchFlags::IN_CREATE
                    | AddWatchFlags::IN_MOVED_TO
            } else {
                AddWatchFlags::from_bits_truncate(FLAGS_TABLE[self.p_type as usize])
            };

            /* If this is a symlink watch both the symlink inode and where it points to. If the inode is
             * not a symlink both calls will install the same watch, which is redundant and doesn't
             * hurt. */
            for follow_symlink in 0..2 {
                let mut f = flags;
                if 0 == follow_symlink {
                    f |= AddWatchFlags::IN_DONT_FOLLOW;
                } else {
                    f &= !AddWatchFlags::IN_DONT_FOLLOW;
                }

                match inotify.add_watch(anster, flags) {
                    Ok(w) => wd = Some(w),
                    Err(err) => {
                        if IN_SET!(err, Errno::EACCES, Errno::ENOENT) {
                            /* This is an expected error, let's accept this
                             * quietly: we have an incomplete watch for now. */
                            incomplete = true;
                            break;
                        }

                        /* This second call to add_watch() should fail like the previous one
                         * and is done for logging the error in a comprehensive way. */
                        match inotify.add_watch(anster, flags) {
                            Ok(w) => wd = Some(w),
                            Err(err) => {
                                self.unwatch();
                                return Err(Error::Nix { source: err });
                            }
                        }

                        /* Succeeded in adding the watch this time. let's continue. */
                    }
                }
            }

            if incomplete {
                break;
            }

            exists = true;

            /* Path exists, we don't need to watch parent too closely. */
            if anster.parent().is_some() {
                let _ = inotify.add_watch(anster.parent().unwrap(), AddWatchFlags::IN_MOVE_SELF);
                /* Error is ignored, the worst can happen is we get spurious events. */
            }

            if anster == self.path.as_path() {
                *self.primary_wd.borrow_mut() = wd;
            }
        }

        if !exists {
            log::error!(
                "Failed to add watch on any of the components of: {:?}",
                self.path
            );
            self.unwatch();
            return Err(Error::Nix {
                source: Errno::EACCES,
            });
        }

        Ok(())
    }

    /// file unwatch
    pub fn unwatch(&self) {
        if INVALID_FD != self.inotify_fd() {
            close(self.inotify_fd());
            self.set_inotify_fd(INVALID_FD)
        }
    }

    /// read file event
    pub fn read_fd_event(&self) -> Result<bool> {
        let inotify = unsafe { Inotify::from_raw_fd(self.inotify_fd()) };
        let events = match inotify.read_events() {
            Ok(events) => events,
            Err(e) => {
                if errno_is_transient(e) {
                    return Ok(false);
                }

                return Err(Error::Other {
                    msg: "read evnets from inotify error".to_string(),
                });
            }
        };

        if IN_SET!(self.p_type, PathType::Changed, PathType::Modified) {
            for event in events {
                if let Some(ref wd) = self.primary_wd() {
                    if event.wd == *wd {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }
}

impl fmt::Display for PathSpec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "path:{:?} type:{:?} inotify_fd:{:?}",
            self.path(),
            self.path_type(),
            self.inotify_fd(),
        )
    }
}
