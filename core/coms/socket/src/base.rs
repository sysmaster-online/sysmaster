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

use nix::{libc, sys::socket::SockProtocol};

#[cfg(feature = "plugin")]
pub(super) const PLUGIN_NAME: &str = "SocketUnit";

#[repr(i32)]
#[derive(Debug, Eq, PartialEq)]
#[allow(clippy::enum_variant_names)]
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
        match protocol.as_str() {
            "route" => NetlinkProtocol::NetlinkRoute,
            "firewall" => NetlinkProtocol::NetlinkFirewall,
            "inet-diag" => NetlinkProtocol::NetlinkInetDiag,
            "nflog" => NetlinkProtocol::NetlinkNflog,
            "xfrm" => NetlinkProtocol::NetlinkXfrm,
            "selinux" => NetlinkProtocol::NetlinkSelinux,
            "iscsi" => NetlinkProtocol::NetlinkIscsi,
            "audit" => NetlinkProtocol::NetlinkAudit,
            "fib-lookup" => NetlinkProtocol::NetlinkFibLookup,
            "connector" => NetlinkProtocol::NetlinkConnector,
            "netfilter" => NetlinkProtocol::NetlinkNetfilter,
            "ip6-fw" => NetlinkProtocol::NetlinkIpv6Fw,
            "dnrtmsg" => NetlinkProtocol::NetlinkDnrtMag,
            "kobject-uevent" => NetlinkProtocol::NetlinkKobjectUevent,
            "generic" => NetlinkProtocol::NetlinkGeneric,
            "scsitransport" => NetlinkProtocol::NetlinkSCSITransport,
            "ecryptfs" => NetlinkProtocol::NetlinkEcryptfs,
            "rdma" => NetlinkProtocol::NetlinkRdma,
            _ => NetlinkProtocol::NetlinkInvalid,
        }
    }
}

impl From<NetlinkProtocol> for SockProtocol {
    fn from(protocol: NetlinkProtocol) -> Self {
        match protocol {
            NetlinkProtocol::NetlinkRoute => SockProtocol::NetlinkRoute,
            NetlinkProtocol::NetlinkFirewall => todo!(),
            NetlinkProtocol::NetlinkInetDiag => SockProtocol::NetlinkSockDiag,
            NetlinkProtocol::NetlinkNflog => todo!(),
            NetlinkProtocol::NetlinkXfrm => todo!(),
            NetlinkProtocol::NetlinkSelinux => SockProtocol::NetlinkSELinux,
            NetlinkProtocol::NetlinkIscsi => SockProtocol::NetlinkISCSI,
            NetlinkProtocol::NetlinkAudit => SockProtocol::NetlinkAudit,
            NetlinkProtocol::NetlinkFibLookup => SockProtocol::NetlinkFIBLookup,
            NetlinkProtocol::NetlinkConnector => todo!(),
            NetlinkProtocol::NetlinkNetfilter => SockProtocol::NetlinkNetFilter,
            NetlinkProtocol::NetlinkIpv6Fw => SockProtocol::NetlinkIPv6Firewall,
            NetlinkProtocol::NetlinkDnrtMag => SockProtocol::NetlinkDECNetRoutingMessage,
            NetlinkProtocol::NetlinkKobjectUevent => SockProtocol::NetlinkKObjectUEvent,
            NetlinkProtocol::NetlinkGeneric => todo!(),
            NetlinkProtocol::NetlinkSCSITransport => SockProtocol::NetlinkSCSITransport,
            NetlinkProtocol::NetlinkEcryptfs => todo!(),
            NetlinkProtocol::NetlinkRdma => SockProtocol::NetlinkRDMA,
            NetlinkProtocol::NetlinkInvalid => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::base::NetlinkProtocol;

    #[test]
    fn test_netlink_from_string() {
        assert_eq!(
            NetlinkProtocol::from("route".to_string()),
            NetlinkProtocol::NetlinkRoute
        );
        assert_eq!(
            NetlinkProtocol::from("firewall".to_string()),
            NetlinkProtocol::NetlinkFirewall
        );
        assert_eq!(
            NetlinkProtocol::from("inet-diag".to_string()),
            NetlinkProtocol::NetlinkInetDiag
        );
        assert_eq!(
            NetlinkProtocol::from("nflog".to_string()),
            NetlinkProtocol::NetlinkNflog
        );
        assert_eq!(
            NetlinkProtocol::from("xfrm".to_string()),
            NetlinkProtocol::NetlinkXfrm
        );
        assert_eq!(
            NetlinkProtocol::from("selinux".to_string()),
            NetlinkProtocol::NetlinkSelinux
        );
        assert_eq!(
            NetlinkProtocol::from("iscsi".to_string()),
            NetlinkProtocol::NetlinkIscsi
        );
        assert_eq!(
            NetlinkProtocol::from("audit".to_string()),
            NetlinkProtocol::NetlinkAudit
        );
        assert_eq!(
            NetlinkProtocol::from("fib-lookup".to_string()),
            NetlinkProtocol::NetlinkFibLookup
        );
        assert_eq!(
            NetlinkProtocol::from("connector".to_string()),
            NetlinkProtocol::NetlinkConnector
        );
        assert_eq!(
            NetlinkProtocol::from("netfilter".to_string()),
            NetlinkProtocol::NetlinkNetfilter
        );
        assert_eq!(
            NetlinkProtocol::from("ip6-fw".to_string()),
            NetlinkProtocol::NetlinkIpv6Fw
        );
        assert_eq!(
            NetlinkProtocol::from("dnrtmsg".to_string()),
            NetlinkProtocol::NetlinkDnrtMag
        );
        assert_eq!(
            NetlinkProtocol::from("kobject-uevent".to_string()),
            NetlinkProtocol::NetlinkKobjectUevent
        );
        assert_eq!(
            NetlinkProtocol::from("generic".to_string()),
            NetlinkProtocol::NetlinkGeneric
        );
        assert_eq!(
            NetlinkProtocol::from("scsitransport".to_string()),
            NetlinkProtocol::NetlinkSCSITransport
        );
        assert_eq!(
            NetlinkProtocol::from("ecryptfs".to_string()),
            NetlinkProtocol::NetlinkEcryptfs
        );
        assert_eq!(
            NetlinkProtocol::from("rdma".to_string()),
            NetlinkProtocol::NetlinkRdma
        );
        assert_eq!(
            NetlinkProtocol::from("test".to_string()),
            NetlinkProtocol::NetlinkInvalid
        );
    }
}
