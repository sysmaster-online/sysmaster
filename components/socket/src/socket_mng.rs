use std::{cell::RefCell, rc::Rc};

use process1::manager::{ExecCommand, UnitActionError, UnitActiveState};
use utils::IN_SET;

use crate::{socket_comm::SocketComm, socket_config::SocketConfig, socket_spawn::SocketSpawn};

#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum SocketState {
    Dead,
    StartPre,
    StartChown,
    StartPost,
    Listening,
    Running,
    StopPre,
    StopPreSigterm,
    StopPreSigkill,
    StopPost,
    FinalSigterm,
    FinalSigkill,
    Failed,
    Cleaning,
    StateMax,
}

impl SocketState {
    pub(super) fn to_unit_active_state(&self) -> UnitActiveState {
        match self {
            SocketState::Dead => UnitActiveState::UnitInActive,
            SocketState::StartPre => UnitActiveState::UnitActivating,
            SocketState::StartChown => UnitActiveState::UnitActivating,
            SocketState::StartPost => UnitActiveState::UnitActivating,
            SocketState::Listening => UnitActiveState::UnitActive,
            SocketState::Running => UnitActiveState::UnitActive,
            SocketState::StopPre => UnitActiveState::UnitDeActivating,
            SocketState::StopPreSigterm => UnitActiveState::UnitDeActivating,
            SocketState::StopPreSigkill => UnitActiveState::UnitDeActivating,
            SocketState::StopPost => UnitActiveState::UnitDeActivating,
            SocketState::FinalSigterm => UnitActiveState::UnitDeActivating,
            SocketState::FinalSigkill => UnitActiveState::UnitDeActivating,
            SocketState::Failed => UnitActiveState::UnitDeActivating,
            SocketState::Cleaning => UnitActiveState::UnitFailed,
            SocketState::StateMax => UnitActiveState::UnitMaintenance,
        }
    }
}

#[allow(dead_code)]
enum SocketResult {
    Success,
    FailureResources,
    FailureTimeout,
    FailureExitCode,
    FailureSignal,
    FailureCoreDump,
    FailureStartLimitHit,
    FailureTriggerLimitHit,
    FailureServiceStartLimitHit,
    ResultInvalid,
}

#[allow(dead_code)]
pub(super) struct SocketMng {
    comm: Rc<SocketComm>,
    config: Rc<SocketConfig>,

    spawn: SocketSpawn,
    state: RefCell<SocketState>,
    result: RefCell<SocketResult>,
    control_command: RefCell<Vec<Rc<ExecCommand>>>,
}

impl SocketMng {
    pub(super) fn new(commr: &Rc<SocketComm>, configr: &Rc<SocketConfig>) -> SocketMng {
        SocketMng {
            comm: Rc::clone(commr),
            config: Rc::clone(configr),
            spawn: SocketSpawn::new(commr),
            state: RefCell::new(SocketState::StateMax),
            result: RefCell::new(SocketResult::Success),
            control_command: RefCell::new(Vec::new()),
        }
    }

    pub(super) fn start_check(&self) -> Result<(), UnitActionError> {
        if IN_SET!(
            self.state(),
            SocketState::StopPre,
            SocketState::StopPreSigkill,
            SocketState::StopPreSigterm,
            SocketState::StopPost,
            SocketState::FinalSigterm,
            SocketState::FinalSigkill,
            SocketState::Cleaning
        ) {
            return Err(UnitActionError::UnitActionEAgain);
        }

        if IN_SET!(
            self.state(),
            SocketState::StartPre,
            SocketState::StartChown,
            SocketState::StartPost
        ) {
            return Ok(());
        }

        Ok(())
    }

    pub(super) fn start_action(&self) {}

    pub(super) fn current_active_state(&self) -> UnitActiveState {
        self.state().to_unit_active_state()
    }

    fn state(&self) -> SocketState {
        *self.state.borrow()
    }
}
