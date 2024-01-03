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

//! network interface naming scheme
use crate::{cmdline, error::Error};
use bitflags::bitflags;
use std::{env, fmt::Display, str::FromStr};

bitflags! {
    /// network interface naming scheme flags
    pub struct NamingSchemeFlags: u32 {
        ///
        const SR_IOV_V                  = 1 << 0  ;
        ///
        const NPAR_ARI                  = 1 << 1  ;
        ///
        const INFINIBAND                = 1 << 2  ;
        ///
        const ZERO_ACPI_INDEX           = 1 << 3  ;
        ///
        const ALLOW_RERENAMES           = 1 << 4  ;
        ///
        const STABLE_VIRTUAL_MACS       = 1 << 5  ;
        ///
        const NETDEVSIM                 = 1 << 6  ;
        ///
        const LABEL_NOPREFIX            = 1 << 7  ;
        ///
        const NSPAWN_LONG_HASH          = 1 << 8  ;
        ///
        const BRIDGE_NO_SLOT            = 1 << 9  ;
        ///
        const SLOT_FUNCTION_ID          = 1 << 10 ;
        ///
        const ONBOARD_16BIT_INDEX       = 1 << 11 ;
        ///
        const REPLACE_STRICTLY          = 1 << 12 ;
        ///
        const XEN_VIF                   = 1 << 13 ;
        ///
        const BRIDGE_MULTIFUNCTION_SLOT = 1 << 14 ;
        ///
        const DEVICETREE_ALIASES        = 1 << 15 ;
        ///
        const USB_HOST                  = 1 << 16 ;
    }
}

bitflags! {
    /// network interface naming scheme
    pub struct NamingScheme: u32 {
        /// no naming scheme
        const V000 = 0;
        /// version 0.2.3
        const V023 = (
            NamingSchemeFlags::SR_IOV_V.bits()
            | NamingSchemeFlags::NPAR_ARI.bits()
            | NamingSchemeFlags::INFINIBAND.bits()
            | NamingSchemeFlags::ZERO_ACPI_INDEX.bits()
            | NamingSchemeFlags::ALLOW_RERENAMES.bits()
            | NamingSchemeFlags::STABLE_VIRTUAL_MACS.bits()
            | NamingSchemeFlags::NETDEVSIM.bits()
            | NamingSchemeFlags::LABEL_NOPREFIX.bits()
            | NamingSchemeFlags::NSPAWN_LONG_HASH.bits()
            | NamingSchemeFlags::BRIDGE_NO_SLOT.bits()
            | NamingSchemeFlags::SLOT_FUNCTION_ID.bits()
            | NamingSchemeFlags::ONBOARD_16BIT_INDEX.bits()
            | NamingSchemeFlags::REPLACE_STRICTLY.bits()
        );
        /// latest scheme
        const LATEST = (
            NamingScheme::V023.bits()
            | NamingSchemeFlags::XEN_VIF.bits()
            | NamingSchemeFlags::BRIDGE_MULTIFUNCTION_SLOT.bits()
            | NamingSchemeFlags::DEVICETREE_ALIASES.bits()
            | NamingSchemeFlags::USB_HOST.bits()
        );
    }
}

impl FromStr for NamingScheme {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "v000" | "0" => Ok(NamingScheme::V000),
            "v023" => Ok(NamingScheme::V023),
            "latest" => Ok(NamingScheme::LATEST),
            _ => Err(Error::ParseNamingScheme {
                what: s.to_string(),
            }),
        }
    }
}

impl Display for NamingScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match *self {
            Self::V000 => "v000",
            Self::V023 => "v023",
            Self::LATEST => "latest",
            _ => "invalid",
        };
        write!(f, "{}", s)
    }
}

/// get the naming scheme according to cmdline and environment variables
pub fn naming_scheme() -> NamingScheme {
    let cmdline_value = cmdline::Cmdline::default()
        .get_param("net.naming-scheme")
        .unwrap_or_else(|| "".to_string());

    /*
     * Environment variable 'NET_NAMING_SCHEME' is prior to cmdline parameter 'net.naming-scheme',
     * except when 'NET_NAMING_SCHEME' starts with ':'.
     */
    let name = if let Ok(v) = env::var("NET_NAMING_SCHEME") {
        if let Some(s) = v.strip_prefix(':') {
            if !cmdline_value.is_empty() {
                cmdline_value
            } else {
                s.to_string()
            }
        } else {
            v
        }
    } else {
        cmdline_value
    };

    if let Ok(scheme) = name.parse::<NamingScheme>() {
        log::info!("Using net name scheme '{}'", name);
        return scheme;
    }

    log::info!("Using net name scheme 'latest'");

    NamingScheme::LATEST
}

/// check whether the naming scheme is enabled
pub fn naming_scheme_enabled() -> bool {
    if let Some(v) = cmdline::Cmdline::default().get_param("net.ifnames") {
        if ["0", "false"].contains(&v.as_str()) {
            return false;
        }
    }

    true
}

/// check whether a naming scheme contains the flag
pub fn naming_scheme_has(flag: NamingSchemeFlags) -> bool {
    let scheme = naming_scheme();

    (scheme.bits() & flag.bits()) > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmdline;
    use std::env;

    #[test]
    fn test_naming_scheme() {
        assert_eq!(naming_scheme(), NamingScheme::LATEST);
        env::set_var("NET_NAMING_SCHEME", "v023");
        assert_eq!(naming_scheme(), NamingScheme::V023);
        env::set_var("NET_NAMING_SCHEME", "v000");
        assert_eq!(naming_scheme(), NamingScheme::V000);
        env::set_var("NET_NAMING_SCHEME", "0");
        assert_eq!(naming_scheme(), NamingScheme::V000);

        let cmdline = cmdline::Cmdline::default();
        let cmdline_value = cmdline.get_param("net.naming-scheme").unwrap_or_default();

        let scheme = cmdline_value
            .parse::<NamingScheme>()
            .unwrap_or(NamingScheme::V023);
        env::set_var("NET_NAMING_SCHEME", ":v023");
        assert_eq!(naming_scheme(), scheme);

        let scheme = cmdline_value
            .parse::<NamingScheme>()
            .unwrap_or(NamingScheme::V000);
        env::set_var("NET_NAMING_SCHEME", ":v000");
        assert_eq!(naming_scheme(), scheme);
    }

    #[test]
    fn test_naming_scheme_enabled() {
        let ret = naming_scheme_enabled();
        if let Some(v) = cmdline::Cmdline::default().get_param("net.ifnames") {
            if ["0", "false"].contains(&v.as_str()) {
                assert!(!ret);
            } else {
                assert!(ret);
            }
        } else {
            assert!(ret);
        }
    }

    #[test]
    fn test_naming_scheme_has() {
        env::set_var("NET_NAMING_SCHEME", "latest");
        assert!(naming_scheme_has(NamingSchemeFlags::SR_IOV_V));
        assert!(naming_scheme_has(NamingSchemeFlags::NPAR_ARI));
        assert!(naming_scheme_has(NamingSchemeFlags::INFINIBAND));
        assert!(naming_scheme_has(NamingSchemeFlags::ZERO_ACPI_INDEX));
        assert!(naming_scheme_has(NamingSchemeFlags::ALLOW_RERENAMES));
        assert!(naming_scheme_has(NamingSchemeFlags::STABLE_VIRTUAL_MACS));
        assert!(naming_scheme_has(NamingSchemeFlags::NETDEVSIM));
        assert!(naming_scheme_has(NamingSchemeFlags::LABEL_NOPREFIX));
        assert!(naming_scheme_has(NamingSchemeFlags::NSPAWN_LONG_HASH));
        assert!(naming_scheme_has(NamingSchemeFlags::BRIDGE_NO_SLOT));
        assert!(naming_scheme_has(NamingSchemeFlags::SLOT_FUNCTION_ID));
        assert!(naming_scheme_has(NamingSchemeFlags::ONBOARD_16BIT_INDEX));
        assert!(naming_scheme_has(NamingSchemeFlags::REPLACE_STRICTLY));
        assert!(naming_scheme_has(NamingSchemeFlags::XEN_VIF));
        assert!(naming_scheme_has(
            NamingSchemeFlags::BRIDGE_MULTIFUNCTION_SLOT
        ));
        assert!(naming_scheme_has(NamingSchemeFlags::DEVICETREE_ALIASES));
        assert!(naming_scheme_has(NamingSchemeFlags::USB_HOST));

        env::set_var("NET_NAMING_SCHEME", "v023");
        assert!(naming_scheme_has(NamingSchemeFlags::SR_IOV_V));
        assert!(naming_scheme_has(NamingSchemeFlags::NPAR_ARI));
        assert!(naming_scheme_has(NamingSchemeFlags::INFINIBAND));
        assert!(naming_scheme_has(NamingSchemeFlags::ZERO_ACPI_INDEX));
        assert!(naming_scheme_has(NamingSchemeFlags::ALLOW_RERENAMES));
        assert!(naming_scheme_has(NamingSchemeFlags::STABLE_VIRTUAL_MACS));
        assert!(naming_scheme_has(NamingSchemeFlags::NETDEVSIM));
        assert!(naming_scheme_has(NamingSchemeFlags::LABEL_NOPREFIX));
        assert!(naming_scheme_has(NamingSchemeFlags::NSPAWN_LONG_HASH));
        assert!(naming_scheme_has(NamingSchemeFlags::BRIDGE_NO_SLOT));
        assert!(naming_scheme_has(NamingSchemeFlags::SLOT_FUNCTION_ID));
        assert!(naming_scheme_has(NamingSchemeFlags::ONBOARD_16BIT_INDEX));
        assert!(naming_scheme_has(NamingSchemeFlags::REPLACE_STRICTLY));

        assert!(!naming_scheme_has(NamingSchemeFlags::XEN_VIF));
        assert!(!naming_scheme_has(
            NamingSchemeFlags::BRIDGE_MULTIFUNCTION_SLOT
        ));
        assert!(!naming_scheme_has(NamingSchemeFlags::DEVICETREE_ALIASES));
        assert!(!naming_scheme_has(NamingSchemeFlags::USB_HOST));

        env::set_var("NET_NAMING_SCHEME", "v000");
        assert!(!naming_scheme_has(NamingSchemeFlags::SR_IOV_V));
        assert!(!naming_scheme_has(NamingSchemeFlags::NPAR_ARI));
        assert!(!naming_scheme_has(NamingSchemeFlags::INFINIBAND));
        assert!(!naming_scheme_has(NamingSchemeFlags::ZERO_ACPI_INDEX));
        assert!(!naming_scheme_has(NamingSchemeFlags::ALLOW_RERENAMES));
        assert!(!naming_scheme_has(NamingSchemeFlags::STABLE_VIRTUAL_MACS));
        assert!(!naming_scheme_has(NamingSchemeFlags::NETDEVSIM));
        assert!(!naming_scheme_has(NamingSchemeFlags::LABEL_NOPREFIX));
        assert!(!naming_scheme_has(NamingSchemeFlags::NSPAWN_LONG_HASH));
        assert!(!naming_scheme_has(NamingSchemeFlags::BRIDGE_NO_SLOT));
        assert!(!naming_scheme_has(NamingSchemeFlags::SLOT_FUNCTION_ID));
        assert!(!naming_scheme_has(NamingSchemeFlags::ONBOARD_16BIT_INDEX));
        assert!(!naming_scheme_has(NamingSchemeFlags::REPLACE_STRICTLY));
        assert!(!naming_scheme_has(NamingSchemeFlags::XEN_VIF));
        assert!(!naming_scheme_has(
            NamingSchemeFlags::BRIDGE_MULTIFUNCTION_SLOT
        ));
        assert!(!naming_scheme_has(NamingSchemeFlags::DEVICETREE_ALIASES));
        assert!(!naming_scheme_has(NamingSchemeFlags::USB_HOST));
    }
}
