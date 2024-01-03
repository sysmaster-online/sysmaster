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

//! id128 functions
use bitflags::bitflags;
use nix::{errno::Errno, fcntl, fcntl::OFlag, sys::stat::Mode, unistd};
use rand::Rng;
use std::{
    fs,
    io::{Error, ErrorKind},
    os::unix::io::RawFd,
    path::Path,
    str,
};

#[derive(PartialEq)]
enum Id128String {
    UninitLen = 13,     // length of "uninitialized".
    UninitLenN = 14,    // length of "uninitialized\n".
    PlainUuidMAx = 32,  // plain UUID without trailing newline
    PlainUuidMAxN = 33, // plain UUID with trailing newline
    RfcUuidMax = 36,    // RFC UUID without trailing newline
    RfcUuidMaxN = 37,   // RFC UUID with trailing newline
    ErrorLen,
}

impl From<usize> for Id128String {
    fn from(item: usize) -> Self {
        match item {
            13 => Id128String::UninitLen,
            14 => Id128String::UninitLenN,
            32 => Id128String::PlainUuidMAx,
            33 => Id128String::PlainUuidMAxN,
            36 => Id128String::RfcUuidMax,
            37 => Id128String::RfcUuidMaxN,
            _ => Id128String::ErrorLen,
        }
    }
}

bitflags! {
    /// Format of id128
    pub struct Id128FormatFlag : u8{
        /// formatted as 32 hex chars as-is. eg: 12345678901234567890abcdef123456
        const ID128_FORMAT_PLAIN = 1;
        /// formatted as 36 character uuid string. eg:12345678-9012-3456-7890-abcdef123456
        const ID128_FORMAT_UUID = 1 << 1;
        /// format any type
        const ID128_FORMAT_ANY = (1 << 1) | 1;
    }
}

fn id128_plain_is_valid(id128: &[u8]) -> bool {
    let mut id128 = id128.to_owned();

    id128.retain(|&x| x != b'\n');
    if id128.len() != 32 {
        return false;
    }
    for i in id128 {
        if (b'0'..=b'9').contains(&i) || (b'A'..=b'F').contains(&i) || (b'a'..=b'f').contains(&i) {
            continue;
        }
        return false;
    }
    true
}

fn id128_rfc2plain(id128: &mut Vec<u8>) -> bool {
    if id128[8] != b'-' || id128[13] != b'-' || id128[18] != b'-' || id128[23] != b'-' {
        return false;
    }

    id128.retain(|&x| x != b'-');
    true
}

fn id128_plain2rfc(id128: &mut Vec<u8>) -> bool {
    if !id128_plain_is_valid(id128) {
        return false;
    }

    id128.insert(8, b'-');
    id128.insert(13, b'-');
    id128.insert(18, b'-');
    id128.insert(23, b'-');

    true
}

fn id128_rfc_is_valid(id128: &[u8]) -> bool {
    let mut id128 = id128.to_owned();

    id128.retain(|&x| x != b'\n');

    if !id128_rfc2plain(&mut id128) {
        return false;
    }

    id128_plain_is_valid(&id128)
}

/// Check id128 is valid?
pub fn id128_is_valid(id128: &[u8]) -> bool {
    let mut id128 = id128.to_owned();

    id128.retain(|&x| x != b'\n');
    match Id128String::from(id128.len()) {
        Id128String::PlainUuidMAx => id128_plain_is_valid(&id128),
        Id128String::RfcUuidMax => id128_rfc_is_valid(&id128),
        _ => false,
    }
}

/// return id128 from $path
pub fn id128_read_by_path(path: &Path, f: Id128FormatFlag) -> std::io::Result<String> {
    let mut id128: Vec<u8> = fs::read(path)?;
    let id128_len = Id128String::from(id128.len());

    match id128_len {
        Id128String::UninitLen | Id128String::UninitLenN => {
            return Err(Error::from(ErrorKind::InvalidData))
        }

        Id128String::PlainUuidMAx | Id128String::PlainUuidMAxN => {
            if id128_len == Id128String::PlainUuidMAxN {
                // end with '\n'
                if !id128.ends_with(&[b'\n']) {
                    return Err(Error::from(ErrorKind::InvalidData));
                }
                id128.pop();
            }

            if !f.contains(Id128FormatFlag::ID128_FORMAT_PLAIN)
                && !f.contains(Id128FormatFlag::ID128_FORMAT_ANY)
            {
                return Err(Error::from(ErrorKind::InvalidInput));
            }
        }

        Id128String::RfcUuidMax | Id128String::RfcUuidMaxN => {
            if id128_len == Id128String::RfcUuidMaxN {
                // end with '\n'
                if !id128.ends_with(&[b'\n']) {
                    return Err(Error::from(ErrorKind::InvalidData));
                }
                id128.pop();
            }

            if !f.contains(Id128FormatFlag::ID128_FORMAT_UUID)
                && !f.contains(Id128FormatFlag::ID128_FORMAT_ANY)
            {
                return Err(Error::from(ErrorKind::InvalidInput));
            }
        }

        _ => return Err(Error::from(ErrorKind::Other)),
    }
    match id128_is_valid(&id128) {
        true => Ok(str::from_utf8(&id128).unwrap().to_string()),
        false => Err(Error::from(ErrorKind::InvalidData)),
    }
}

/// write id128 to $p
pub fn id128_write(p: &Path, f_sync: &bool, id128: &str, f: Id128FormatFlag) -> nix::Result<()> {
    let mut id128 = id128.to_string().into_bytes();
    let fd: RawFd;

    if !id128_is_valid(&id128) {
        return Err(Errno::EINVAL);
    }

    if f == Id128FormatFlag::ID128_FORMAT_PLAIN {
        id128_rfc2plain(&mut id128);
    } else if f == Id128FormatFlag::ID128_FORMAT_UUID {
        id128_plain2rfc(&mut id128);
    }

    // add trail newline
    if !id128.ends_with(&[b'\n']) {
        id128.push(b'\n');
    }

    fd = fcntl::open(
        p,
        OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_CLOEXEC | OFlag::O_NOCTTY | OFlag::O_TRUNC,
        Mode::S_IRUSR | Mode::S_IRGRP | Mode::S_IROTH,
    )?;
    unistd::write(fd, &id128)?;
    if *f_sync {
        unistd::fsync(fd)?;
    }
    unistd::close(fd)?;

    Ok(())
}

/// The function generates a random string of hexadecimal characters and converts it to a UUID format if
/// specified.
///
/// Arguments:
///
/// * `f`: The parameter `f` is of type `Id128FormatFlag`, which is an enum representing the formatting
/// options for the generated ID. It is used to determine whether the generated ID should be in UUID
/// format or not.
///
/// Returns:
///
/// a `nix::Result<String>`.
pub fn id128_randomize(f: Id128FormatFlag) -> nix::Result<String> {
    let mut rng = rand::thread_rng();
    let mut ret: String = String::new();

    let mut i = 0;
    while i < 32 {
        let s: u32 = rng.gen_range(0..16);
        let hex = format!("{:x}", s);
        ret.push_str(&hex);
        i += 1;
    }

    if f.contains(Id128FormatFlag::ID128_FORMAT_UUID) {
        let mut ret_u8 = ret.into_bytes();
        if !id128_plain2rfc(&mut ret_u8) {
            return Err(Errno::UnknownErrno);
        }
        ret = String::from_utf8(ret_u8).unwrap();
    }
    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};

    #[test]
    fn test_id128_read_by_path() {
        let p = Path::new("test_id128_read_by_path");

        fs::write(p, b"12345678901234567890abcdef123456").unwrap();
        assert_eq!(
            id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_PLAIN).unwrap(),
            String::from("12345678901234567890abcdef123456")
        );
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_UUID).is_err());
        assert_eq!(
            id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_ANY).unwrap(),
            String::from("12345678901234567890abcdef123456")
        );

        fs::write(p, b"12345678901234567890abcdef123456\n").unwrap();
        assert_eq!(
            id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_PLAIN).unwrap(),
            String::from("12345678901234567890abcdef123456")
        );
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_UUID).is_err());
        assert_eq!(
            id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_ANY).unwrap(),
            String::from("12345678901234567890abcdef123456")
        );

        fs::write(p, b"123e4567-e89b-12b3-a456-426614174000").unwrap();
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_PLAIN).is_err());
        assert_eq!(
            id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_UUID).unwrap(),
            String::from("123e4567-e89b-12b3-a456-426614174000")
        );
        assert_eq!(
            id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_ANY).unwrap(),
            String::from("123e4567-e89b-12b3-a456-426614174000")
        );

        fs::write(p, b"123e4567-e89b-12b3-a456-426614174000\n").unwrap();
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_PLAIN).is_err());
        assert_eq!(
            id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_UUID).unwrap(),
            String::from("123e4567-e89b-12b3-a456-426614174000")
        );
        assert_eq!(
            id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_ANY).unwrap(),
            String::from("123e4567-e89b-12b3-a456-426614174000")
        );

        fs::write(p, b"123").unwrap();
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_PLAIN).is_err());
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_UUID).is_err());
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_ANY).is_err());

        fs::write(p, b"123e-4567e89b-12b3-a456-426614174000").unwrap();
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_PLAIN).is_err());
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_UUID).is_err());
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_ANY).is_err());

        fs::write(p, b"123e456--e89b-12b3-a456-426614174000").unwrap();
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_PLAIN).is_err());
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_UUID).is_err());
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_ANY).is_err());

        fs::write(p, b"z2345678901234567890abcdef123456").unwrap();
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_PLAIN).is_err());
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_UUID).is_err());
        assert!(id128_read_by_path(p, Id128FormatFlag::ID128_FORMAT_ANY).is_err());

        fs::remove_file("test_id128_read_by_path").unwrap();
    }

    #[test]
    fn test_id128_is_valid() {
        assert!(!id128_is_valid(&String::from("123").into_bytes()));
        assert!(!id128_is_valid(
            &String::from("123123sfa1234sdfasdfqwer").into_bytes()
        ));
        assert!(!id128_is_valid(
            &String::from("12345678901234567890abcdef1234561").into_bytes()
        ));
        assert!(!id128_is_valid(
            &String::from("123e456--e89b-12b3-a456-426614174000").into_bytes()
        ));
        assert!(!id128_is_valid(
            &String::from("123e-4567e89b-12b3-a456-426614174000").into_bytes()
        ));

        assert!(id128_is_valid(
            &String::from("12345678901234567890abcdef123456").into_bytes()
        ));
        assert!(id128_is_valid(
            &String::from("12345678901234567890abcdef123456\n").into_bytes()
        ));
        assert!(id128_is_valid(
            &String::from("12345678-9012-3456-7890-abcdef123456").into_bytes()
        ));
        assert!(id128_is_valid(
            &String::from("12345678-9012-3456-7890-abcdef123456\n").into_bytes()
        ));
    }
    #[test]
    fn test_id128_randomize() {
        let mut i = 0;
        while i < 10 {
            assert!(id128_is_valid(
                &id128_randomize(Id128FormatFlag::ID128_FORMAT_PLAIN)
                    .unwrap()
                    .into_bytes()
            ));
            assert!(id128_is_valid(
                &id128_randomize(Id128FormatFlag::ID128_FORMAT_UUID)
                    .unwrap()
                    .into_bytes()
            ));
            i += 1;
        }
    }
}
