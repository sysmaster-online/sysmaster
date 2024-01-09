// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS,WITHOUT WARRANTIES OF ANY
// KIND,EITHER EXPRESS OR IMPLIED,INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT,MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//!  Timer is a kind of sysmaster startup type. It can accelerate startup by creating a socket socket first, and then pulling the corresponding service when the socket socket receives a request.
//!  The timer configuration file contains three sections: Unit, Socket, and Install.
//!
//! #  Example:
//! ``` toml
//!  [Unit]
//!  Description="test timer"
//!  Documentation="test.timer"
//!
//!  [Timer]
//!  OnActiveSec=3s
//!
//!  [Install]
//!  WantedBy="timers.target"
//! ```
//!  `[Timer]` section related configuration
//!  AccuracySec
//!
//!  Set the trigger precision of the timer
//!
//!  OnActiveSec,OnBootSec,OnStartupSec,OnUnitActiveSec,OnUnitInactiveSec
//!
//!  Defines a monotone timer relative to a period of time after a specific point in time
//!
//!  OnCalendar
//!
//!  Define a calendar timer based on the wallclock
//!
//!  RandomizedDelaySec
//!
//!  Randomly delay the timer of this unit for a short period of time
//!
//!  Unit
//!
//!  The matching unit of the timer unit. The default value is the service unit with the same name as the timer unit
//!
//!  Persistent
//!
//!  WakeSystem
//!
//!  Whether to wake up the hibernating system and prevent the system from entering the hibernation state when a timer reaches the timer's trigger time point
//!
//!  RemainAfterElapse
//!
//!  Whether to maintain the loaded state of the expired timer unit and still allow the query of its status

pub use {manager::__um_obj_create, unit::__subunit_create_with_params};

// dependency:
// timer_base -> timer_rentry -> {timer_comm | timer_config}
// timer_mng -> timer_unit -> timer_manager

mod base;
mod bus;
mod comm;
mod config;
mod load;
mod manager;
mod mng;
mod rentry;
mod unit;
