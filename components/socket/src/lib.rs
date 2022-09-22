//! socket是process1启动类型的一种，通过先创建socket套接字，socket套接字收到请求时再拉起对应的service服务，达到加速启动的目的。
//! socket配置文件包含Unit、Socket、Install三个Section。
//!
//! # Example:
//! ```toml
//! [Unit]
//! Description="test service Socket"
//! Documentation="test.service"
//!
//! [Socket]
//! ExecStartPre=["/usr/bin/sleep 5"]
//! ListenStream="31972"
//! ReceiveBuffer = "4K"
//! SendBuffer = "4K"
//! PassPacketInfo = "off"
//! PassCredentials = "off"
//! PassSecurity = "on"
//! SocketMode="0600"
//!
//! [Install]
//! WantedBy="dbus.service"
//! ```

//! [Socket] section相关的配置
//!
//! [ExecStartPre]
//!
//! socket启动前需要执行的命令行，相似的配置还有ExecStartChown、ExecStartPost、ExecStopPre、 ExecStopPost，分别在对应的启动阶段执行。
//!
//! ListenStream、ListenDatagram
//!
//! 分别配置 SOCK_STREAM、SOCK_DGRAM 类型的套接字， 配置格式如下：
//!
//! 以 / 开头的则默认创建unix套接字。
//!
//! 以 @ 开头的则默认创建抽象名字空间的unix套接字。
//!
//! 若为数字类型， 则默认创建IPv6类型的套接字， 若不支持IPv6类型， 则创建IPv4类型的套接字。
//!
//! 若格式为a.b.c.d:x格式， 则创建IPv4类型的套接字，IP地址为“a.b.c.d”, 端口为x。
//!
//! 若格式为[a]:x格式， 则创建IPv6类型的套接字，IP地址为“a", 端口为x。
//!
//! ListenNetlink
//!
//! 设置一个要监听的 Netlink 套接字。 格式为 {名称} + {组ID}
//! 当前支持的协议名为route、inet-diag、selinux、iscsi、audit、fib-lookup、 netfilter、ip6-fw、dnrtmsg、kobject_uevent、scsitransport、rdma
//!
//! ReceiveBuffer 、SendBuffer
//!
//! 设置套接字的接受和发送缓冲区的大小， 配置的类型为数值型。
//!
//! PassPacketInfo
//!
//! 可设为 true 或 false(默认), 是否获取接收报文的相关信息，也可在发送报文时指定报文的相关控制信息。
//!
//! PassCredentials
//!
//! 可设为 true 或 false(默认), 是否允许套接字接收对端进程在辅助消息中发送的证书。
//!
//! PassSecurity
//!
//! 可设为 true 或 false(默认), 是否允许套接字接收对端进程在辅助消息中发送的安全上下文。
//!
//! SocketMode
//!
//! 设置创建文件节点时的访问模式，适用于unix套接字时创建的文件。

// dependency:
// socket_base -> {socket_comm | socket_config}
// {socket_pid | socket_spawn} ->
// {socket_mng | socket_load}
// {socket_port} -> socket_unit

mod socket_base;

mod socket_comm;

mod socket_config;

mod socket_mng;

mod socket_load;

mod socket_spawn;

mod socket_pid;

mod socket_port;

mod socket_unit;
