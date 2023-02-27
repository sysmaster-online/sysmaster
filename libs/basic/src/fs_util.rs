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

//! the utils of the file operation
//!
use crate::error::*;
use nix::{fcntl::OFlag, sys::stat::Mode};
use pathdiff::diff_paths;
use std::path::Path;

/// open the parent directory of path
pub fn open_parent(path: &Path, flags: OFlag, mode: Mode) -> Result<i32> {
    let parent = path.parent().ok_or(Error::Nix {
        source: nix::errno::Errno::EINVAL,
    })?;

    nix::fcntl::open(parent, flags, mode).context(NixSnafu)
}

/// create symlink link_name -> target
pub fn symlink(target: &str, link: &str, relative: bool) -> Result<()> {
    let link_path = Path::new(&link);
    let target_path = Path::new(&target);

    let (target_path, fd) = if relative {
        let link_path_parent = link_path.parent().ok_or(Error::NotExisted {
            what: format!("{}'s parent", link_path.to_string_lossy()),
        })?;

        let rel_path = diff_paths(target_path, link_path_parent).unwrap();
        let fd = nix::fcntl::open(&rel_path, OFlag::O_DIRECT, Mode::from_bits(0).unwrap())
            .context(NixSnafu)?;
        (rel_path, Some(fd))
    } else {
        (target_path.to_path_buf(), None)
    };

    nix::unistd::symlinkat(target_path.as_path(), fd, link_path).map_err(|e| {
        log::debug!("Failed to create symlink: {} -> {}", link, target);
        Error::Nix { source: e }
    })
}

#[cfg(test)]
mod tests {
    use crate::fs_util::symlink;
    use nix::unistd;

    #[test]
    fn test_symlink() {
        // use a complicated long name to make sure we don't have this file
        // before running this testcase.
        let link_name_path = std::path::Path::new("/tmp/test_link_name_39285b");
        if link_name_path.exists() {
            return;
        }

        let ret = symlink("/dev/null", "/tmp/test_link_name_39285b", false);
        assert!(ret.is_ok());

        let ret = unistd::unlinkat(
            None,
            link_name_path.to_str().unwrap(),
            unistd::UnlinkatFlags::NoRemoveDir,
        );
        assert!(ret.is_ok());
    }
}
