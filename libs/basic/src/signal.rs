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

use nix::sys::signal::{self, SaFlags, SigAction, SigHandler, SigSet, Signal};

/// reset all signal handlers
pub fn reset_all_signal_handlers() {
    for sig in nix::sys::signal::Signal::iterator() {
        /* SIGKILL and SIGSTOP is invalid, see sigaction(2) */
        if sig == Signal::SIGKILL || sig == Signal::SIGSTOP {
            continue;
        }
        let flags = SaFlags::SA_RESTART;
        let sig_handler = SigHandler::SigDfl;
        let sig_action = SigAction::new(sig_handler, flags, SigSet::empty());
        unsafe {
            if let Err(e) = signal::sigaction(sig, &sig_action) {
                log::warn!("Failed to reset signal {}: {}", sig, e);
            }
        }
    }
}

/// reset signal mask
pub fn reset_signal_mask() {
    if let Err(e) = signal::sigprocmask(
        signal::SigmaskHow::SIG_SETMASK,
        Some(&SigSet::empty()),
        None,
    ) {
        log::warn!("reset sigprocmask failed:{}", e);
    }
}
