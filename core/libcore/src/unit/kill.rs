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

use crate::serialize::DeserializeWith;
use nix::sys::signal::Signal;
use serde::{Deserialize, Deserializer, Serialize};
use std::{cell::RefCell, rc::Rc};
use unit_parser::prelude::UnitEntry;

/// kill operation send to process
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug)]
pub enum KillOperation {
    KillTerminate,
    KillTerminateAndLog,
    KillRestart,
    KillKill,
    KillWatchdog,
    KillInvalid,
}

impl KillOperation {
    ///
    pub fn to_signal(&self, kill_context: Rc<KillContext>) -> Signal {
        match *self {
            KillOperation::KillTerminate
            | KillOperation::KillTerminateAndLog
            | KillOperation::KillRestart => kill_context.kill_signal(),
            KillOperation::KillKill => Signal::SIGKILL,
            KillOperation::KillWatchdog => Signal::SIGABRT,
            _ => Signal::SIGTERM,
        }
    }
}

/// the method to kill the process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KillMode {
    /// kill all the process in the cgroup of the unit
    ControlGroup,
    /// only kill the main process
    Process,
    /// send SIGKILL to the process of the cgroup
    Mixed,
}

impl Default for KillMode {
    fn default() -> Self {
        Self::ControlGroup
    }
}

impl UnitEntry for KillMode {
    type Error = crate::error::Error;

    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        let s = String::from(input.as_ref());
        match s {
            s if s == "control-group" => Ok(KillMode::ControlGroup),
            s if s == "process" => Ok(KillMode::Process),
            s if s == "mixed" => Ok(KillMode::Mixed),
            _ => Ok(KillMode::ControlGroup),
        }
    }
}

impl DeserializeWith for KillMode {
    type Item = Self;
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;
        match s {
            s if s == "control-group" => Ok(KillMode::ControlGroup),
            s if s == "process" => Ok(KillMode::Process),
            s if s == "mixed" => Ok(KillMode::Mixed),
            _ => Ok(KillMode::ControlGroup),
        }
    }
}

/// kill method context of the unit
pub struct KillContext {
    kill_mode: RefCell<KillMode>,
    kill_signal: RefCell<Signal>,
}

impl Default for KillContext {
    fn default() -> Self {
        Self {
            kill_mode: RefCell::new(KillMode::default()),
            kill_signal: RefCell::new(Signal::SIGTERM),
        }
    }
}

impl KillContext {
    /// set the kill mode
    pub fn set_kill_mode(&self, mode: KillMode) {
        *self.kill_mode.borrow_mut() = mode;
    }

    /// get the kill mode
    pub fn kill_mode(&self) -> KillMode {
        *self.kill_mode.borrow()
    }

    /// set the configured kill signal
    pub fn set_kill_signal(&self, signal: Signal) {
        *self.kill_signal.borrow_mut() = signal;
    }

    /// get the kill signal
    fn kill_signal(&self) -> Signal {
        *self.kill_signal.borrow()
    }
}
