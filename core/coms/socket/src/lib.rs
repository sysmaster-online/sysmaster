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

//!  Socket is a kind of sysmaster startup type. It can accelerate startup by creating a socket socket first, and then pulling the corresponding service when the socket socket receives a request.
//!  The socket configuration file contains three sections: Unit, Socket, and Install.
//!
//! #  Example:
//! ``` toml
//!  [Unit]
//!  Description="test service Socket"
//!  Documentation="test.service"
//!
//!  [Socket]
//!  ExecStartPre=["/usr/bin/sleep 5"]
//!  ListenStream="31972"
//!  ReceiveBuffer = "4K"
//!  SendBuffer = "4K"
//!  PassPacketInfo = "off"
//!  PassCredentials = "off"
//!  PassSecurity = "on"
//!  SocketMode="0600"
//!
//!  [Install]
//!  WantedBy="dbus.service"
//! ```
//!  `[Socket]` section related configuration
//!
//!  `[ExecStartPre]`
//!
//!  The command line that needs to be executed before the socket starts. Similar configurations include ExecStartChown, ExecStartPost, ExecStopPre, and ExecStopPost, which are executed in the corresponding startup phase.
//!
//!  ListenStream, ListenDatagram
//!
//!  Configure SOCK separately_ STREAM, SOCK_ The configuration format of sockets of DGRAM type is as follows:
//!
//!  Unix sockets that start with/are created by default.
//!
//!  Unix sockets starting with @ will be created as abstract namespaces by default.
//!
//!  If it is a number type, it will create a socket of IPv6 type by default. If it does not support IPv6 type, it will create a socket of IPv4 type.
//!
//!  If the format is `a.b.c.d: x`, create an IPv4 socket with the IP address of "a.b.c.d" and port of x.
//!
//!  If the format is `[a]: x`, create a socket of IPv6 type with IP address of "a" and port of x.
//!
//!  ListenNetlink
//!
//!  Set a Netlink socket to listen to. The format is {name}+{group ID}
//!  The currently supported protocols are named route, inet diag, selinux, iscsi, audit, fib lookup, netfilter, ip6 fw, dnrtmsg, kobject_ uevent, scsitransport, rdma
//!
//!  ReceiveBuffer, SendBuffer
//!
//!  Set the size of the socket's receive and send buffers. The configured type is numeric.
//!
//!  PassPacketInfo
//!
//!  It can be set to true or false (default) to determine whether to obtain the relevant information of the received message, or specify the relevant control information of the message when sending the message.
//!
//!  PassCredentials
//!
//!  Can be set to true or false (default), whether to allow the socket to receive the certificate sent by the peer process in the auxiliary message.
//!
//!  PassSecurity
//!
//!  Can be set to true or false (default), whether to allow the socket to receive the security context sent by the peer process in the auxiliary message.
//!
//!  SocketMode
//!
//!  Set the access mode when creating a file node, which is applicable to files created when unix sockets are used.

#[cfg(all(feature = "plugin", feature = "noplugin"))]
compile_error!("feature plugin and noplugin cannot be enabled at the same time");

pub use {manager::__um_obj_create, unit::__subunit_create_with_params};

// dependency:
// socket_base -> service_rentry -> {socket_comm | socket_config}
// {socket_pid | socket_spawn | socket_port} ->
// {socket_mng | socket_load} -> socket_unit -> socket_manager

mod base;
mod bus;
mod comm;
mod config;
mod load;
mod manager;
mod mng;
mod pid;
mod port;
mod rentry;
mod spawn;
mod unit;
