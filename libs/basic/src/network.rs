// Copyright (c) 2022 Huawe&i Technologies Co.,Ltd. All rights reserved.
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

//! ethernet address utilities

use crate::error::*;
use libc::{AF_INET, AF_INET6};
use std::fmt::{self, Display};

/// IPv4 address length
pub const IPV4_LEN: usize = 4;
/// IPv6 address length
pub const IPV6_LEN: usize = 16;
/// Mac address length
pub const MAC_LEN: usize = 6;
/// InfiniBand address length
pub const INFINIBAND_LEN: usize = 20;

/// Hardware address max size
pub const HW_ADDR_MAX_SIZE: usize = 32;

/// Hardware address
#[derive(PartialEq, Eq, Debug, Default)]
pub struct HwAddress {
    /// Bytes number of address data
    pub length: usize,
    /// Internal address data
    pub data: AddrData,
}

/// Internal address data
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum AddrData {
    /// IPv4
    Ipv4(Vec<u8>),
    /// IPv6
    Ipv6(Vec<u8>),
    /// MAC
    Ether(Vec<u8>),
    /// InfiniBand
    InfiniBand(Vec<u8>),

    /// Arbitrary address
    Arbitrary(Vec<u8>),
}

impl AddrData {
    fn bytes_ref(&self) -> &Vec<u8> {
        match self {
            Self::Ipv4(bytes) => bytes,
            Self::Ipv6(bytes) => bytes,
            Self::Ether(bytes) => bytes,
            Self::InfiniBand(bytes) => bytes,
            Self::Arbitrary(bytes) => bytes,
        }
    }

    fn bytes_mut(&mut self) -> &Vec<u8> {
        match self {
            Self::Ipv4(bytes) => bytes,
            Self::Ipv6(bytes) => bytes,
            Self::Ether(bytes) => bytes,
            Self::InfiniBand(bytes) => bytes,
            Self::Arbitrary(bytes) => bytes,
        }
    }
}

impl Default for AddrData {
    fn default() -> Self {
        AddrData::Arbitrary(vec![])
    }
}

impl HwAddress {
    /// borrow reference to the internal bytes of ethernet address
    pub fn bytes_ref(&self) -> &Vec<u8> {
        self.data.bytes_ref()
    }

    /// borrow mutable reference to the internal bytes of ethernet address
    pub fn bytes_mut(&mut self) -> &Vec<u8> {
        self.data.bytes_mut()
    }
}

impl Display for HwAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();

        for byte in self.bytes_ref() {
            s.push_str(&format!("{:02x}", byte));
        }

        write!(f, "{}", s)
    }
}

/// Parse string into MAC, IPv4, IPv6, InfinitBand or arbitrary address according to the expected length:
///
/// The mapping relations between specific 'expected_len' and address kind are as following:
///     0           =>      Automatically parsed into IPv4/IPv6/MAC
///     4           =>      IPv4
///     16          =>      IPv6
///     6           =>      MAC
///     20          =>      InfiniBand
///     usize:MAX   =>      Arbitrary Address
///
/// If 'expected_len' is assigned to other value, the string will be parsed into arbitrary address.
///
/// Note that 'expected_len' can not be longer than 32 bytes.
///
/// The IPv4/IPv6 is accept as following:
///
/// IPv4 is separated by dot, the fields has 1 decimal bytes:           192.168.1.1
/// IPv6 is separated by colon, the fields has 2 hexadecimal bytes:     2001:0db8:0000:0000:0000:8a2e:0370:7334
///
/// The MAC, InfiniBand and arbitrary address is accept as following:
///
/// separated by dot, the field has 2 hexadecimal bytes:                aabb.ccdd.eeff
/// separated by colon or hyphen, the field has 1 hexadecimal byte:     aa-bb-cc-dd-ee-ff
///
/// IPv4 consists of 4 bytes.
/// IPv6 consists of 16 bytes.
/// MAC consists of 6 bytes.
/// InfiniBand consists of 20 bytes.
/// Arbitrary address can have infinite bytes.
pub fn parse_hw_addr_full(s: &str, expected_len: usize) -> Result<HwAddress> {
    if (expected_len > HW_ADDR_MAX_SIZE && expected_len != usize::MAX) || s.is_empty() {
        return Err(Error::Nix {
            source: nix::Error::EINVAL,
        });
    }

    /* Try to parse IPv4/IPv6 address at first. */
    if [0, IPV4_LEN, IPV6_LEN].contains(&expected_len) {
        if expected_len == 0 {
            if let Ok(addr) = ip_addr_from_string_auto(s) {
                return Ok(addr);
            }
        } else {
            let family = match expected_len {
                IPV4_LEN => AF_INET,
                _ => AF_INET6,
            };

            if let Ok(addr) = ip_addr_from_string(family, s) {
                let addr = match expected_len {
                    IPV4_LEN => HwAddress {
                        length: IPV4_LEN,
                        data: AddrData::Ipv4(addr),
                    },
                    _ => HwAddress {
                        length: IPV6_LEN,
                        data: AddrData::Ipv6(addr),
                    },
                };
                return Ok(addr);
            }

            return Err(Error::Nix {
                source: nix::Error::EINVAL,
            });
        }
    }

    /* Try to parse other kinds of address. */
    let (fields, _) = addr_split(s)?;
    let mut bytes: Vec<u8> = vec![];

    let field_bytes = if s.find('.').is_some() {
        2
    } else if s.find(':').is_some() || s.find('-').is_some() {
        1
    } else {
        return Err(Error::Nix {
            source: nix::Error::EINVAL,
        });
    };

    /* Validate the fields and collect the address bytes. */
    for i in fields {
        if i.len() / 2 != field_bytes {
            return Err(Error::Nix {
                source: nix::Error::EINVAL,
            });
        }

        match field_bytes {
            1 => {
                bytes.push(u8::from_str_radix(i, 16).map_err(|_| Error::Nix {
                    source: nix::Error::EINVAL,
                })?);
            }
            _ => {
                bytes.push(u8::from_str_radix(&i[0..2], 16).map_err(|_| Error::Nix {
                    source: nix::Error::EINVAL,
                })?);
                bytes.push(u8::from_str_radix(&i[2..4], 16).map_err(|_| Error::Nix {
                    source: nix::Error::EINVAL,
                })?);
            }
        }
    }

    /* Validate the length of address bytes. */
    if (expected_len == 0 && ![MAC_LEN, INFINIBAND_LEN].contains(&bytes.len()))
        || (![usize::MAX, 0].contains(&expected_len) && expected_len != bytes.len())
    {
        return Err(Error::Nix {
            source: nix::Error::EINVAL,
        });
    }

    let ret = match bytes.len() {
        MAC_LEN => HwAddress {
            length: MAC_LEN,
            data: AddrData::Ether(bytes),
        },
        INFINIBAND_LEN => HwAddress {
            length: INFINIBAND_LEN,
            data: AddrData::InfiniBand(bytes),
        },
        _ => HwAddress {
            length: bytes.len(),
            data: AddrData::Arbitrary(bytes),
        },
    };

    Ok(ret)
}

fn ip_addr_from_string_auto(s: &str) -> Result<HwAddress> {
    if let Ok(bytes) = ip_addr_from_string(AF_INET, s) {
        return Ok(HwAddress {
            length: IPV4_LEN,
            data: AddrData::Ipv4(bytes),
        });
    } else if let Ok(bytes) = ip_addr_from_string(AF_INET6, s) {
        return Ok(HwAddress {
            length: IPV6_LEN,
            data: AddrData::Ipv6(bytes),
        });
    }

    Err(Error::Nix {
        source: nix::Error::EINVAL,
    })
}

fn ip_addr_from_string(family: i32, s: &str) -> Result<Vec<u8>> {
    let (fields, splitter) = addr_split(s)?;

    let mut ret: Vec<u8> = vec![];

    match family {
        AF_INET => {
            if fields.len() != IPV4_LEN || splitter != '.' {
                return Err(Error::Nix {
                    source: nix::Error::EINVAL,
                });
            }

            for i in fields.iter() {
                ret.push(i.parse::<u8>().map_err(|_| Error::Nix {
                    source: nix::Error::EINVAL,
                })?);
            }
        }
        AF_INET6 => {
            const IPV6_FIELD_BYTES: usize = 2;
            const IPV6_FIELD_WIDTH: usize = 4;

            if fields.len() != IPV6_LEN / IPV6_FIELD_BYTES || splitter != ':' {
                return Err(Error::Nix {
                    source: nix::Error::EINVAL,
                });
            }

            for i in fields.iter() {
                if i.len() != IPV6_FIELD_WIDTH {
                    return Err(Error::Nix {
                        source: nix::Error::EINVAL,
                    });
                }

                ret.push(u8::from_str_radix(&i[0..2], 16).map_err(|_| Error::Nix {
                    source: nix::Error::EINVAL,
                })?);
                ret.push(u8::from_str_radix(&i[2..4], 16).map_err(|_| Error::Nix {
                    source: nix::Error::EINVAL,
                })?);
            }
        }
        _ => {
            return Err(Error::Nix {
                source: nix::Error::EAFNOSUPPORT,
            });
        }
    }

    Ok(ret)
}

fn addr_split(s: &str) -> Result<(Vec<&str>, char)> {
    let splitter = if s.find('.').is_some() {
        '.'
    } else if s.find(':').is_some() {
        ':'
    } else if s.find('-').is_some() {
        '-'
    } else {
        return Err(Error::Nix {
            source: nix::Error::EINVAL,
        });
    };

    Ok((s.split(splitter).collect(), splitter))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_addr_from_string() {
        /* Test valid usage */
        assert_eq!(
            ip_addr_from_string(AF_INET, "172.0.0.1").unwrap(),
            vec![172, 0, 0, 1]
        );
        assert_eq!(
            ip_addr_from_string(AF_INET6, "2001:0db8:0000:0000:0000:8a2e:0370:7334").unwrap(),
            vec![
                0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8a, 0x2e, 0x03, 0x70,
                0x73, 0x34
            ]
        );
        assert_eq!(
            ip_addr_from_string_auto("172.0.0.1").unwrap(),
            HwAddress {
                length: IPV4_LEN,
                data: AddrData::Ipv4(vec![172, 0, 0, 1]),
            }
        );
        assert_eq!(
            ip_addr_from_string_auto("2001:0db8:0000:0000:0000:8a2e:0370:7334").unwrap(),
            HwAddress {
                length: IPV6_LEN,
                data: AddrData::Ipv6(vec![
                    0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8a, 0x2e, 0x03,
                    0x70, 0x73, 0x34
                ]),
            }
        );

        /* Test invalid ip format. */

        /* IPv4 is not allowed to be separated by colon or hyphen. */
        assert!(ip_addr_from_string(AF_INET, "172:0:0:1").is_err());
        assert!(ip_addr_from_string(AF_INET, "172-0-0-1").is_err());

        /* IPv6 is not allowed to be separated by dot or hyphen. */
        assert!(ip_addr_from_string_auto("2001.0db8.0000.0000.0000.8a2e.0370.7334").is_err());
        assert!(ip_addr_from_string_auto("2001-0db8-0000-0000-0000-8a2e-0370-7334").is_err());

        /* IPv4 fields can not exceed u8::MAX and can only have digits. */
        assert!(ip_addr_from_string(AF_INET, "256.0.0.1").is_err());
        assert!(ip_addr_from_string(AF_INET, "ff.0.0.1").is_err());

        /* IPv6 fields should be 4 width and be hexadecimal bytes. */
        assert!(ip_addr_from_string_auto("2001a-0db8-0000-0000-0000-8a2e-0370-7334").is_err());
        assert!(ip_addr_from_string_auto("200g-0db8-0000-0000-0000-8a2e-0370-7334").is_err());

        /* IPv4 should have 4 fields and IPv6 should have 16 fields. */
        assert!(ip_addr_from_string(AF_INET, "").is_err());
        assert!(ip_addr_from_string(AF_INET, "127.0.0.0.1").is_err());
        assert!(ip_addr_from_string(AF_INET, "127.0.1").is_err());
        assert!(ip_addr_from_string_auto("").is_err());
        assert!(ip_addr_from_string_auto("2001:0db8:0000:0000:0000:8a2e:0370:7334:0000").is_err());
        assert!(ip_addr_from_string_auto("2001:0db8:0000:0000:0000:8a2e:0370").is_err());
    }

    #[test]
    fn test_parse_hw_addr_full() {
        /* Test parsing MAC */
        assert_eq!(
            parse_hw_addr_full("00:0c:29:8d:21:a3", 0).unwrap(),
            HwAddress {
                length: MAC_LEN,
                data: AddrData::Ether(vec![0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3]),
            }
        );
        assert_eq!(
            parse_hw_addr_full("000c.298d.21a3", 0).unwrap(),
            HwAddress {
                length: MAC_LEN,
                data: AddrData::Ether(vec![0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3]),
            }
        );
        assert_eq!(
            parse_hw_addr_full("00-0c-29-8d-21-a3", 0).unwrap(),
            HwAddress {
                length: MAC_LEN,
                data: AddrData::Ether(vec![0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3]),
            }
        );
        assert_eq!(
            parse_hw_addr_full("00:0c:29:8d:21:a3", MAC_LEN).unwrap(),
            HwAddress {
                length: MAC_LEN,
                data: AddrData::Ether(vec![0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3]),
            }
        );

        /* Test parsing InfiniBand */
        assert_eq!(
            parse_hw_addr_full(
                "00:0c:29:8d:21:a3:00:0c:29:8d:21:a3:00:0c:29:8d:21:a3:00:0c",
                0
            )
            .unwrap(),
            HwAddress {
                length: INFINIBAND_LEN,
                data: AddrData::InfiniBand(vec![
                    0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00,
                    0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00, 0x0c
                ]),
            }
        );
        assert_eq!(
            parse_hw_addr_full(
                "00-0c-29-8d-21-a3-00-0c-29-8d-21-a3-00-0c-29-8d-21-a3-00-0c",
                0
            )
            .unwrap(),
            HwAddress {
                length: INFINIBAND_LEN,
                data: AddrData::InfiniBand(vec![
                    0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00,
                    0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00, 0x0c
                ]),
            }
        );
        assert_eq!(
            parse_hw_addr_full("000c.298d.21a3.000c.298d.21a3.000c.298d.21a3.000c", 0).unwrap(),
            HwAddress {
                length: INFINIBAND_LEN,
                data: AddrData::InfiniBand(vec![
                    0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00,
                    0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00, 0x0c
                ]),
            }
        );
        assert_eq!(
            parse_hw_addr_full(
                "00:0c:29:8d:21:a3:00:0c:29:8d:21:a3:00:0c:29:8d:21:a3:00:0c",
                INFINIBAND_LEN
            )
            .unwrap(),
            HwAddress {
                length: INFINIBAND_LEN,
                data: AddrData::InfiniBand(vec![
                    0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00,
                    0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00, 0x0c
                ]),
            }
        );

        /* Test parsing Arbitrary address */
        assert_eq!(
            parse_hw_addr_full("00:0c:29:8d:21:a3:00:0c", usize::MAX,).unwrap(),
            HwAddress {
                length: 8,
                data: AddrData::Arbitrary(vec![0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00, 0x0c]),
            }
        );
        assert_eq!(
            parse_hw_addr_full("00-0c-29-8d-21-a3-00-0c", usize::MAX,).unwrap(),
            HwAddress {
                length: 8,
                data: AddrData::Arbitrary(vec![0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00, 0x0c]),
            }
        );
        assert_eq!(
            parse_hw_addr_full("000c.298d.21a3.000c", usize::MAX,).unwrap(),
            HwAddress {
                length: 8,
                data: AddrData::Arbitrary(vec![0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00, 0x0c]),
            }
        );
        assert_eq!(
            parse_hw_addr_full("000c.298d.21a3.000c", 8,).unwrap(),
            HwAddress {
                length: 8,
                data: AddrData::Arbitrary(vec![0x00, 0x0c, 0x29, 0x8d, 0x21, 0xa3, 0x00, 0x0c]),
            }
        );

        /* Test parsing IPv4 */
        assert_eq!(
            parse_hw_addr_full("172.0.0.1", 0).unwrap(),
            HwAddress {
                length: IPV4_LEN,
                data: AddrData::Ipv4(vec![172, 0, 0, 1]),
            }
        );
        assert_eq!(
            parse_hw_addr_full("172.0.0.1", IPV4_LEN).unwrap(),
            HwAddress {
                length: IPV4_LEN,
                data: AddrData::Ipv4(vec![172, 0, 0, 1]),
            }
        );

        /* Test parsing IPv6 */
        assert_eq!(
            parse_hw_addr_full("2001:0db8:0000:0000:0000:8a2e:0370:7334", 0).unwrap(),
            HwAddress {
                length: IPV6_LEN,
                data: AddrData::Ipv6(vec![
                    0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8a, 0x2e, 0x03,
                    0x70, 0x73, 0x34
                ]),
            }
        );
        assert_eq!(
            parse_hw_addr_full("2001:0db8:0000:0000:0000:8a2e:0370:7334", IPV6_LEN).unwrap(),
            HwAddress {
                length: IPV6_LEN,
                data: AddrData::Ipv6(vec![
                    0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8a, 0x2e, 0x03,
                    0x70, 0x73, 0x34
                ]),
            }
        );

        /* Test invalid usage. */
        assert!(parse_hw_addr_full("172:0:0:1", 0).is_err());
        assert!(parse_hw_addr_full("", 0).is_err());
        assert!(parse_hw_addr_full("", usize::MAX).is_err());
    }
}
