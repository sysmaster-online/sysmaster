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

//! Service is one of the unit types supported in sysmaster. Service is used to pull up services and manage them.
//! The service configuration file contains three sections: Unit,Socket,and Install.
//!
//! # Example:
//! ``` toml
//! [Unit]
//! Description="test service"
//! Documentation="test.service"
//!
//! [Service]
//! Type="Simple"
//! ExecCondition="/usr/bin/sleep 5"
//! ExecStartPre="/usr/bin/sleep 5"
//! ExecStart="/bin/echo 'test'"
//! ExecStop="/bin/kill $MAINPID"
//!
//!
//! [Install]
//! WantedBy="dbus.service"
//! ```
//! `[Service]` section related configuration
//!
//! Type
//!
//! The service type configuration field currently supports simple,forking,oneshot,and notify The default value is simple when not configured.
//!
//! The simple mode indicates that the service startup is completed when the fork sub process succeeds.
//! The forking mode indicates that when the fork sub process exits,the service startup is completed. The pid of the sub process needs to be obtained through PIDFile.
//! The oneshot mode exits after the service is executed.
//! Notify the status message to the sysmaster after the notify mode service is started.
//! Supported notification messages MAINPID=$val,READY=$val,STOPPING=$val,ERRNO=$val
//!
//!
//! ExecCondition,ExecStartPre,ExecStart,ExecStop,ExecStartPost
//!
//! The commands that a service needs to execute at different startup stages support configuring multiple commands division. For example,"/usr/bin/sleep 5;/bin/echo 'test'".
//!
//! PIDFile
//!
//! When the Type field is forking,you need to configure this field to obtain the PID of the child process
//!
//! RemainAfterExit
//!
//! Support the configuration of true and false. When the configuration is true,the service is still considered as active after exiting. The default configuration is false.
//!
//! NotifyAccess
//!
//! The support configuration is main,which means that the notification sent by the MAINPID process is supported.
//!
//! Environment
//!
//! The environment variable parameter passed to the child process can be configured with more than one The configuration format is "key=value".
//!
//! Sockets
//!
//! Indicates the socket service that the current service depends on. The dependency is Wants. Support multiple configurations,and use ";" when configuring multiple division.
//!

#[cfg(all(feature = "plugin", feature = "noplugin"))]
compile_error!("feature plugin and noplugin cannot be enabled at the same time");

pub use {manager::__um_obj_create, unit::__subunit_create_with_params};

// dependency:
// base -> rentry -> {comm | config}
// {pid | spawn} ->
// {mng} ->
// {monitor} -> {bus -> unit} -> manager
mod base;
mod bus;
mod comm;
mod config;
mod manager;
mod mng;
mod monitor;
mod pid;
mod rentry;
mod spawn;
mod unit;
