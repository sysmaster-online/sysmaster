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

//! Common used functions to parse user and group

use crate::error::*;
use nix::libc::uid_t;
use nix::unistd::{Gid, Group, Uid, User};
use std::time::SystemTime;

const USEC_INFINITY: u128 = u128::MAX;

///
pub fn timespec_load(systime: SystemTime) -> u128 {
    match systime.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d.as_micros(),
        Err(_) => USEC_INFINITY,
    }
}

/// Parse a string as UID
pub fn parse_uid(uid_str: &str) -> Result<User> {
    if uid_str.is_empty() {
        return Err(Error::Invalid {
            what: "UID is empty".to_string(),
        });
    }

    if uid_str.eq("0") {
        // This shouldn't fail.
        return Ok(User::from_uid(Uid::from_raw(0)).unwrap().unwrap());
    }

    let mut first = true;
    for c in uid_str.bytes() {
        // uid must only contains 0-9
        if !first && c.is_ascii_digit() {
            continue;
        }
        // uid must starts with 1-9
        if first && (b'1'..=b'9').contains(&c) {
            first = false;
            continue;
        }
        return Err(Error::Invalid {
            what: "UID must only contains 0-9 and shouldn't starts with 0".to_string(),
        });
    }

    let uid = match uid_str.parse::<u32>() {
        Err(e) => {
            return Err(e.into());
        }
        Ok(v) => v,
    };

    let user = match User::from_uid(Uid::from_raw(uid)) {
        Err(e) => {
            return Err(Error::Nix { source: e });
        }
        Ok(v) => v,
    };

    match user {
        None => Err(Error::Invalid {
            what: "No matched UID".to_string(),
        }),
        Some(v) => Ok(v),
    }
}

/// Parse a string as UID
pub fn parse_gid(gid_str: &str) -> Result<Group> {
    // Same logic as parse_uid()
    if gid_str.is_empty() {
        return Err(Error::Invalid {
            what: "GID is empty".to_string(),
        });
    }

    if gid_str.eq("0") {
        return Ok(Group::from_gid(Gid::from_raw(0)).unwrap().unwrap());
    }

    let mut first = true;
    for c in gid_str.bytes() {
        if !first && c.is_ascii_digit() {
            continue;
        }
        if first && (b'1'..=b'9').contains(&c) {
            first = false;
            continue;
        }
        return Err(Error::Invalid {
            what: "GID must only contains 0-9 and shouldn't starts with 0".to_string(),
        });
    }

    let gid = match gid_str.parse::<u32>() {
        Err(e) => {
            return Err(e.into());
        }
        Ok(v) => v,
    };

    let group = match Group::from_gid(Gid::from_raw(gid)) {
        Err(e) => {
            return Err(Error::Nix { source: e });
        }
        Ok(v) => v,
    };

    match group {
        None => Err(Error::Invalid {
            what: "No matched GID".to_string(),
        }),
        Some(v) => Ok(v),
    }
}

/// Parse a string as Username
pub fn parse_name(name_str: &str) -> Result<User> {
    if name_str.is_empty() {
        return Err(Error::Invalid {
            what: "Username is empty".to_string(),
        });
    }

    let user = match User::from_name(name_str).context(NixSnafu) {
        Err(e) => {
            return Err(e);
        }
        Ok(v) => v,
    };
    match user {
        None => Err(Error::Invalid {
            what: "No matched username".to_string(),
        }),
        Some(v) => Ok(v),
    }
}

/// check if the user id is within the system user range
pub fn uid_is_system(uid: Uid) -> bool {
    const SYSTEM_UID_MAX: uid_t = 999;
    uid.as_raw() <= SYSTEM_UID_MAX
}

/// get user creds
pub fn get_user_creds(user: &str) -> Result<User> {
    if let Ok(u) = parse_uid(user) {
        return Ok(u);
    }
    if let Ok(Some(u)) = User::from_name(user) {
        return Ok(u);
    }
    Err(Error::Invalid {
        what: "invalid user name".to_string(),
    })
}

/// get group creds
pub fn get_group_creds(group: &str) -> Result<Group> {
    if let Ok(g) = parse_gid(group) {
        return Ok(g);
    }
    if let Ok(Some(g)) = Group::from_name(group) {
        return Ok(g);
    }
    Err(Error::Invalid {
        what: "invalid group name".to_string(),
    })
}

#[cfg(test)]
mod test {
    use super::parse_uid;

    #[test]
    fn test_parse_uid() {
        let s_uid = String::from("0");
        let u = parse_uid(&s_uid);
        println!("{:?}", u);
        assert_eq!(u.unwrap().name, "root");
        let s_uid = String::from("1");
        let u = parse_uid(&s_uid).unwrap();
        assert!(u.name == "bin" || u.name == "daemon");
        let s_invalid_uid = String::from("abc_i");
        let u = parse_uid(&s_invalid_uid);
        let e = u.expect_err("invalid uid");
        assert!(e
            .to_string()
            .contains("UID must only contains 0-9 and shouldn't starts with 0"));
    }
}
