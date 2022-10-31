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
//!  [Socket] section related configuration
//!
//!  [ExecStartPre]
//!
//!  The command line that needs to be executed before the socket starts. Similar configurations include ExecStartChown, ExecStartPost, ExecStopPre, and ExecStopPost, which are executed in the corresponding startup phase.
//!
//!  ListenStream、ListenDatagram
//!
//!  Configure SOCK separately_ STREAM、SOCK_ The configuration format of sockets of DGRAM type is as follows:
//!
//!  Unix sockets that start with/are created by default.
//!
//!  Unix sockets starting with @ will be created as abstract namespaces by default.
//!
//!  If it is a number type, it will create a socket of IPv6 type by default. If it does not support IPv6 type, it will create a socket of IPv4 type.
//!
//!  If the format is a.b.c.d: x, create an IPv4 socket with the IP address of "a.b.c.d" and port of x.
//!
//!  If the format is [a]: x, create a socket of IPv6 type with IP address of "a" and port of x.
//!
//!  ListenNetlink
//!
//!  Set a Netlink socket to listen to. The format is {name}+{group ID}
//!  The currently supported protocols are named route, inet diag, selinux, iscsi, audit, fib lookup, netfilter, ip6 fw, dnrtmsg, kobject_ uevent、scsitransport、rdma
//!
//!  ReceiveBuffer 、SendBuffer
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

// dependency:
// socket_base -> service_rentry -> {socket_comm | socket_config}
// {socket_pid | socket_spawn | socket_port} ->
// {socket_mng | socket_load} -> socket_unit -> socket_manager

mod socket_base;
mod socket_comm;
mod socket_config;
mod socket_load;
mod socket_manager;
mod socket_mng;
mod socket_pid;
mod socket_port;
mod socket_rentry;
mod socket_spawn;
mod socket_unit;
