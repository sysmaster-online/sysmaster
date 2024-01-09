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

//! Common used constants by init, sysmaster and other extensions.

/// Signal used run unrecover, SIGRTMIN+8
pub const SIG_RUN_UNRECOVER_OFFSET: i32 = 8;
/// Signal used to restart the manager, SIGRTMIN+9
pub const SIG_RESTART_MANAGER_OFFSET: i32 = 9;
/// Signal used to switch root, SIGRTMIN+10
pub const SIG_SWITCH_ROOT_OFFSET: i32 = 10;

/// Socket used to transfer message between sysmaster and init
pub const INIT_SOCKET: &str = "/run/sysmaster/init";
/// sysmaster send this to init to keep alive
pub const ALIVE: &str = "ALIVE01234567890";

/// Socket used to transfer message between sysmaster and sctl
pub const PRIVATE_SOCKET: &str = "/run/sysmaster/private";

/// Default log file path when LogTarget is configured to "file"
pub const LOG_FILE_PATH: &str = "/var/log/sysmaster/sysmaster.log";

/// A file that stores the init parameters
pub const INIT_PARA_PATH: &str = "/run/sysmaster/init_para";

/// invalid fd
pub const INVALID_FD: i32 = -1;
