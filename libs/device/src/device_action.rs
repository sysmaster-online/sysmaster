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

//! device action
//!
use std::fmt::Display;
use std::str::FromStr;

use nix::errno::Errno;

/// device action based on kobject from kernel
#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceAction {
    Add,
    Remove,
    Change,
    Move,
    Online,
    Offline,
    Bind,
    Unbind,
    Invalid,
}

impl Default for DeviceAction {
    fn default() -> Self {
        Self::Invalid
    }
}

impl FromStr for DeviceAction {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "add" => Ok(Self::Add),
            "remove" => Ok(Self::Remove),
            "change" => Ok(Self::Change),
            "move" => Ok(Self::Move),
            "online" => Ok(Self::Online),
            "offline" => Ok(Self::Offline),
            "bind" => Ok(Self::Bind),
            "unbind" => Ok(Self::Unbind),
            _ => Err(Self::Err::Nix {
                msg: "device: invalid action string".to_string(),
                source: Errno::EINVAL,
            }),
        }
    }
}

impl Display for DeviceAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Add => "add",
            Self::Remove => "remove",
            Self::Change => "change",
            Self::Move => "move",
            Self::Online => "online",
            Self::Offline => "offline",
            Self::Bind => "bind",
            Self::Unbind => "unbind",
            _ => "invalid",
        };

        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use crate::DeviceAction;

    /// test whether device action parse and display normally
    #[test]
    fn test_device_action() {
        let action_add: DeviceAction = "add".parse().unwrap();
        let action_remove: DeviceAction = "remove".parse().unwrap();
        let action_change: DeviceAction = "change".parse().unwrap();
        let action_move: DeviceAction = "move".parse().unwrap();
        let action_online: DeviceAction = "online".parse().unwrap();
        let action_offline: DeviceAction = "offline".parse().unwrap();
        let action_bind: DeviceAction = "bind".parse().unwrap();
        let action_unbind: DeviceAction = "unbind".parse().unwrap();

        assert_eq!(action_add, DeviceAction::Add);
        assert_eq!(action_remove, DeviceAction::Remove);
        assert_eq!(action_change, DeviceAction::Change);
        assert_eq!(action_move, DeviceAction::Move);
        assert_eq!(action_online, DeviceAction::Online);
        assert_eq!(action_offline, DeviceAction::Offline);
        assert_eq!(action_bind, DeviceAction::Bind);
        assert_eq!(action_unbind, DeviceAction::Unbind);

        assert_eq!(format!("{}", action_add), "add");
        assert_eq!(format!("{}", action_remove), "remove");
        assert_eq!(format!("{}", action_change), "change");
        assert_eq!(format!("{}", action_move), "move");
        assert_eq!(format!("{}", action_online), "online");
        assert_eq!(format!("{}", action_offline), "offline");
        assert_eq!(format!("{}", action_bind), "bind");
        assert_eq!(format!("{}", action_unbind), "unbind");
    }
}
