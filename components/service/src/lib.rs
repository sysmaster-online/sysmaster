//! service是sysmaster中支持的unit类型的一种，通过service拉起服务， 进行服务的管理。
//! service配置文件包含Unit、Socket、Install三个Section。
//!
//! # Example:
//! ```toml
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
//! [Service] section相关的配置
//!
//! Type
//!
//! service类型的配置字段，当前支持simple、forking、oneshot、notify. 未配置时默认值为simple。
//!
//!     simple模式表示fork子进程成功之后即代表服务启动完成。
//!     forking模式表示fork子进程退出即代表服务启动完成，子进程的pid需通过PIDFile获取。
//!     oneshot模式等待服务执行完之后退出。
//!     notify模式服务启动完成后通告状态消息给sysmaster。
//!         支持的通告消息 MAINPID=$val, READY=$val, STOPPING=$val, ERRNO=$val.
//!
//!
//! ExecCondition、ExecStartPre、ExecStart、ExecStop、ExecStartPost
//!
//! service在不同的启动阶段需要执行的命令， 支持配置多条命令，多条命令以“；”分割。如 “/usr/bin/sleep 5; /bin/echo 'test'”。
//!
//! PIDFile
//!
//! 当Type字段为forking时， 需要配置此字段，用来获取子进程的PID.
//!
//! RemainAfterExit
//!
//! 支持配置为true、false, 当配置为true时，当服务退出后仍认为服务为active状态。默认配置为false。
//!
//! NotifyAccess
//!
//! 支持配置为main, 表示支持MAINPID进程发送的通告。
//!
//! Environment
//!
//! 传递给子进程的环境变量参数，可配置多条，配置多条时以“；”分割，配置格式为“key=value”的形式。
//!
//! Sockets
//!
//! 表示当前service依赖的socket服务。依赖关系为Wants。支持配置多条，配置多条时以“；”分割。
//!

// dependency:
// socket_base -> {socket_comm | socket_config}
// {socket_pid | socket_spawn} ->
// {socket_mng | socket_load}
// {socket_port} -> socket_unit

#[macro_use]
extern crate strum;

// dependency:
// service_base -> {service_comm | service_config}
// {service_pid | service_spawn} ->
// {service_mng | service_load} ->
// {service_monitor} -> service_unit
mod service_base;
mod service_comm;
mod service_config;
mod service_mng;
mod service_monitor;
mod service_pid;
mod service_spawn;
mod service_unit;
