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

//! tools to control static device nodes under devtmpfs
//!

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::{error::*, log_dev_lock_option};
use basic::fs_util::{fchmod_and_chown, futimens_opath};
use device::Device;
use libc::{mode_t, S_IFBLK, S_IFCHR, S_IFMT};
use nix::fcntl::{open, OFlag};
use nix::sys::stat::{self, fstat};
use nix::unistd::{Gid, Uid};
use snafu::ResultExt;

pub(crate) fn static_node_apply_permissions(
    name: String,
    mode: Option<mode_t>,
    uid: Option<Uid>,
    gid: Option<Gid>,
    _tags: &[String],
) -> Result<()> {
    if mode.is_none() && uid.is_none() && gid.is_none() && _tags.is_empty() {
        return Ok(());
    }

    let devnode = format!("/dev/{}", name);

    let fd = match open(
        devnode.as_str(),
        OFlag::O_PATH | OFlag::O_CLOEXEC,
        stat::Mode::empty(),
    ) {
        Ok(ret) => ret,
        Err(e) => {
            if e == nix::errno::Errno::ENOENT {
                return Ok(());
            }
            return Err(crate::rules::Error::Nix { source: e });
        }
    };

    let stat = fstat(fd).context(NixSnafu)?;

    if (stat.st_mode & S_IFMT) != S_IFBLK && (stat.st_mode & S_IFMT) != S_IFCHR {
        log::warn!("'{}' is neither block nor character, ignoring", devnode);
        return Ok(());
    }

    /*
     * todo: export tags to a directory as symlinks
     */

    apply_permission_impl(None, fd, &devnode, false, mode, uid, gid, HashMap::new())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_permission_impl(
    dev: Option<Arc<Mutex<Device>>>,
    fd: i32,
    devnode: &str,
    apply_mac: bool,
    mode: Option<mode_t>,
    uid: Option<Uid>,
    gid: Option<Gid>,
    _seclabel_list: HashMap<String, String>,
) -> Result<()> {
    let stat = fstat(fd)
        .context(NixSnafu)
        .log_dev_lock_option_error(dev.clone())?;

    /* if group is set, but mode is not set, "upgrade" group mode */
    let mode = if mode.is_none() && gid.is_some() && gid.as_ref().unwrap().as_raw() > 0 {
        Some(0o660)
    } else {
        mode
    };

    let apply_mode = mode.is_some() && (stat.st_mode & 0o777) != (mode.as_ref().unwrap() & 0o777);
    let apply_uid = uid.is_some() && stat.st_uid != uid.as_ref().unwrap().as_raw();
    let apply_gid = gid.is_some() && stat.st_gid != gid.as_ref().unwrap().as_raw();

    if apply_mode || apply_uid || apply_gid || apply_mac {
        if apply_mode || apply_uid || apply_gid {
            log_dev_lock_option!(
                debug,
                dev,
                format!(
                    "setting permission for '{}', uid='{}', gid='{}', mode='{:o}'",
                    devnode,
                    uid.as_ref()
                        .map(|i| i.as_raw())
                        .unwrap_or_else(|| stat.st_uid),
                    gid.as_ref()
                        .map(|i| i.as_raw())
                        .unwrap_or_else(|| stat.st_gid),
                    mode.as_ref()
                        .map(|i| *i as u32)
                        .unwrap_or_else(|| stat.st_mode),
                )
            );

            fchmod_and_chown(fd, devnode, mode, uid, gid)
                .context(BasicSnafu)
                .log_dev_lock_option_error(dev.clone())?;
        }
    } else {
        log_dev_lock_option!(debug, dev, "preserve devnode permission");
    }

    /*
     * todo: apply SECLABEL
     */

    futimens_opath(fd, None)
        .context(BasicSnafu)
        .log_dev_lock_option_error(dev)?;

    Ok(())
}
