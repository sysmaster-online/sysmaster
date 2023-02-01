//! Common used functions to parse user and group

use nix::unistd::{Gid, Group, Uid, User};
use std::error::Error;
use std::result::Result;

/// Parse a string as UID
pub fn parse_uid(uid_str: &String) -> Result<User, Box<dyn Error>> {
    if uid_str.is_empty() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "UID is empty",
        )));
    }

    if uid_str.eq("0") {
        // This shouldn't fail.
        return Ok(User::from_uid(Uid::from_raw(0)).unwrap().unwrap());
    }

    let mut first = true;
    for c in uid_str.bytes() {
        // uid must only contains 0-9
        if !first && (b'0'..=b'9').contains(&c) {
            continue;
        }
        // uid must starts with 1-9
        if first && (b'1'..=b'9').contains(&c) {
            first = false;
            continue;
        }
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "UID must only contains 0-9 and shouldn't starts with 0",
        )));
    }

    let uid = match uid_str.parse::<u32>() {
        Err(e) => {
            return Err(Box::new(e));
        }
        Ok(v) => v,
    };

    let user = match User::from_uid(Uid::from_raw(uid)) {
        Err(e) => {
            return Err(Box::new(e));
        }
        Ok(v) => v,
    };

    match user {
        None => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No matched UID",
        ))),
        Some(v) => Ok(v),
    }
}

/// Parse a string as UID
pub fn parse_gid(gid_str: &String) -> Result<Group, Box<dyn Error>> {
    // Same logic as parse_uid()
    if gid_str.is_empty() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "GID is empty",
        )));
    }

    if gid_str.eq("0") {
        return Ok(Group::from_gid(Gid::from_raw(0)).unwrap().unwrap());
    }

    let mut first = true;
    for c in gid_str.bytes() {
        if !first && (b'0'..=b'9').contains(&c) {
            continue;
        }
        if first && (b'1'..=b'9').contains(&c) {
            first = false;
            continue;
        }
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "GID must only contains 0-9 and shouldn't starts with 0",
        )));
    }

    let gid = match gid_str.parse::<u32>() {
        Err(e) => {
            return Err(Box::new(e));
        }
        Ok(v) => v,
    };

    let group = match Group::from_gid(Gid::from_raw(gid)) {
        Err(e) => {
            return Err(Box::new(e));
        }
        Ok(v) => v,
    };

    match group {
        None => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No matched GID",
        ))),
        Some(v) => Ok(v),
    }
}
