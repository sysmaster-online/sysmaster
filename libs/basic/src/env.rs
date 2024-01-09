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

//!

use crate::error::*;
use nix::unistd;
/// struct Env
pub struct Env(String);

impl Env {
    ///create a new instance of Env
    pub fn new(input: &str) -> Result<Env> {
        Env::from_str(input)
    }

    fn from_str(input: &str) -> Result<Env> {
        if Env::is_valid(input) {
            Ok(Env(input.to_string()))
        } else {
            Err(Error::Invalid {
                what: " Invalid Env".to_string(),
            })
        }
    }

    ///Check whether environment variables are valid
    pub fn is_valid(input: &str) -> bool {
        let pos = match input.find('=') {
            Some(pos) => pos,
            None => return false,
        };
        Env::is_valid_name(&input[0..pos]) && Env::is_valid_value(&input[pos + 1..])
    }

    fn is_valid_name(str: &str) -> bool {
        if str.is_empty() {
            return false;
        }

        let str_bytes: &[u8] = str.as_bytes();

        if str_bytes[0].is_ascii_digit() {
            return false;
        }

        let arg_max =
            unistd::sysconf(unistd::SysconfVar::ARG_MAX).map_or(2, |f| f.map_or(2, |s| s)) - 2;
        if str_bytes.len() > arg_max as usize {
            return false;
        }

        for s in str_bytes {
            if !(s.is_ascii_alphanumeric() || *s == b'_') {
                return false;
            }
        }
        true
    }

    fn is_valid_value(str: &str) -> bool {
        let arg_max =
            unistd::sysconf(unistd::SysconfVar::ARG_MAX).map_or(3, |f| f.map_or(3, |s| s)) - 3;
        if str.as_bytes().len() > arg_max as usize {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_env() {
        assert!(Env::new("a=").is_ok());
        assert!(Env::new("b=głąb kapuściany").is_ok());
        assert!(Env::new("c=\\007\\009\\011").is_ok());
        assert!(Env::new("e=printf \"\\x1b]0;<mock-chroot>\\x07<mock-chroot>\"").is_ok());
        assert!(Env::new("f=tab\tcharacter").is_ok());
        assert!(Env::new("g=new\nline").is_ok());

        assert!(Env::new("=").is_err());
        assert!(Env::new("a b=").is_err());
        assert!(Env::new("a =").is_err());
        assert!(Env::new(" b=").is_err());
        assert!(Env::new("a.b=").is_err());
        assert!(Env::new("a-b=").is_err());
        assert!(Env::new("\007=głąb kapuściany").is_err());
        assert!(Env::new("c\009=\007\009\011").is_err());
        assert!(Env::new("głąb=printf \"\x1b]0;<mock-chroot>\x07<mock-chroot>\"").is_err());
    }
}
