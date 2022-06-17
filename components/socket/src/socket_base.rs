use nix::libc;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(super) enum PortType {
    Socket,
    Fifo,
    Invalid,
}

impl Default for PortType {
    fn default() -> Self {
        PortType::Socket
    }
}

#[repr(i32)]
#[derive(Debug, Eq, PartialEq)]
pub(super) enum NetlinkProtocol {
    NetlinkRoute = libc::NETLINK_ROUTE,
    NetlinkFirewall = libc::NETLINK_FIREWALL,
    NetlinkInetDiag = libc::NETLINK_INET_DIAG,
    NetlinkNflog = libc::NETLINK_NFLOG,
    NetlinkXfrm = libc::NETLINK_XFRM,
    NetlinkSelinux = libc::NETLINK_SELINUX,
    NetlinkIscsi = libc::NETLINK_ISCSI,
    NetlinkAudit = libc::NETLINK_AUDIT,
    NetlinkFibLookup = libc::NETLINK_FIB_LOOKUP,
    NetlinkConnector = libc::NETLINK_CONNECTOR,
    NetlinkNetfilter = libc::NETLINK_NETFILTER,
    NetlinkIpv6Fw = libc::NETLINK_IP6_FW,
    NetlinkDnrtMag = libc::NETLINK_DNRTMSG,
    NetlinkKobjectUevent = libc::NETLINK_KOBJECT_UEVENT,
    NetlinkGeneric = libc::NETLINK_GENERIC,
    NetlinkSCSITransport = libc::NETLINK_SCSITRANSPORT,
    NetlinkEcryptfs = libc::NETLINK_ECRYPTFS,
    NetlinkRdma = libc::NETLINK_RDMA,
    NetlinkInvalid,
}

impl From<String> for NetlinkProtocol {
    fn from(protocol: String) -> Self {
        match protocol {
            protocol if protocol == "route" => NetlinkProtocol::NetlinkRoute,
            protocol if protocol == "firewall" => NetlinkProtocol::NetlinkFirewall,
            protocol if protocol == "inet-diag" => NetlinkProtocol::NetlinkInetDiag,
            protocol if protocol == "nflog" => NetlinkProtocol::NetlinkNflog,
            protocol if protocol == "xfrm" => NetlinkProtocol::NetlinkXfrm,
            protocol if protocol == "selinux" => NetlinkProtocol::NetlinkSelinux,
            protocol if protocol == "iscsi" => NetlinkProtocol::NetlinkIscsi,
            protocol if protocol == "audit" => NetlinkProtocol::NetlinkAudit,
            protocol if protocol == "fib-lookup" => NetlinkProtocol::NetlinkFibLookup,
            protocol if protocol == "connector" => NetlinkProtocol::NetlinkConnector,
            protocol if protocol == "netfilter" => NetlinkProtocol::NetlinkNetfilter,
            protocol if protocol == "ip6-fw" => NetlinkProtocol::NetlinkIpv6Fw,
            protocol if protocol == "dnrtmsg" => NetlinkProtocol::NetlinkDnrtMag,
            protocol if protocol == "kobject_uevent" => NetlinkProtocol::NetlinkKobjectUevent,
            protocol if protocol == "generic" => NetlinkProtocol::NetlinkGeneric,
            protocol if protocol == "scsitransport" => NetlinkProtocol::NetlinkSCSITransport,
            protocol if protocol == "ecryptfs" => NetlinkProtocol::NetlinkEcryptfs,
            protocol if protocol == "rdma" => NetlinkProtocol::NetlinkRdma,
            _ => NetlinkProtocol::NetlinkInvalid,
        }
    }
}
