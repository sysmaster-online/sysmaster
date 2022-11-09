use std::cell::RefCell;

use serde::{Deserialize, Deserializer, Serialize};

use crate::manager::DeserializeWith;

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
        KillMode::ControlGroup
    }
}

impl DeserializeWith for KillMode {
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
}

impl Default for KillContext {
    fn default() -> Self {
        Self {
            kill_mode: RefCell::new(KillMode::default()),
        }
    }
}

impl KillContext {
    /// set the kill mode
    pub fn set_kill_mode(&self, mode: KillMode) {
        *self.kill_mode.borrow_mut() = mode;
    }

    pub(crate) fn kill_mode(&self) -> KillMode {
        *self.kill_mode.borrow()
    }
}
