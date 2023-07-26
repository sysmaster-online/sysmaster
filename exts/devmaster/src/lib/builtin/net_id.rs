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

//! net_id builtin
//!

use crate::builtin::Builtin;
use crate::rules::exec_unit::ExecuteUnit;
use crate::{error::*, log_dev};
use basic::ether_addr_util::*;
use basic::naming_scheme::*;
use device::Device;
use libc::{c_char, faccessat, ARPHRD_INFINIBAND, F_OK};
use nix::errno::errno;
use snafu::ResultExt;
use std::cell::RefCell;
use std::os::unix::prelude::{AsRawFd, RawFd};
use std::rc::Rc;

/// Reference: linux/pci_regs.h
/// https://github.com/torvalds/linux/blob/master/include/uapi/linux/pci_regs.h
const PCI_HEADER_TYPE: u8 = 0x0e;

/// net_id builtin command
pub struct NetId;

const ONBOARD_14BIT_INDEX_MAX: u32 = (1 << 14) - 1;
const ONBOARD_16BIT_INDEX_MAX: u32 = (1 << 16) - 1;

#[derive(Debug, PartialEq, Eq)]
enum HwAddrAssignType {
    Permanent = 0,
    _Random = 1,
    _Stolen = 2,
    _Set = 3,
}

struct NetNames {
    r#type: NetNameType,

    pcidev: Rc<RefCell<Device>>,
    pci_slot: String,
    pci_path: String,
    pci_onboard: String,
    pci_onboard_label: String,

    usb_ports: String,
    bcma_core: String,
    ccw_busid: String,
    vio_slot: String,
    xen_slot: String,
    platform_path: String,
    netdevsim_path: String,
    devicetree_onboard: String,
}

impl NetNames {
    fn new(dev: Rc<RefCell<Device>>) -> Self {
        NetNames {
            r#type: NetNameType::default(),

            pcidev: dev,
            pci_slot: String::new(),
            pci_path: String::new(),
            pci_onboard: String::new(),
            pci_onboard_label: String::new(),

            usb_ports: String::new(),
            bcma_core: String::new(),
            ccw_busid: String::new(),
            vio_slot: String::new(),
            xen_slot: String::new(),
            platform_path: String::new(),
            netdevsim_path: String::new(),
            devicetree_onboard: String::new(),
        }
    }
}

#[allow(dead_code)]
#[derive(PartialEq, Eq)]
enum NetNameType {
    Undef,
    Pci,
    Usb,
    Bcma,
    Virtio,
    Ccw,
    Vio,
    Xenvif,
    Platform,
    Netdevsim,
    Devicetree,

    Invalid,
}

impl NetNameType {
    fn default() -> Self {
        Self::Invalid
    }
}

struct LinkInfo {
    ifindex: u32,
    iflink: u32,
    iftype: u16,
    devtype: String,
    physical_port_name: String,
    hw_addr: HwAddress,
}

/// Skip intermediate virtio device
fn skip_virtio(dev: Rc<RefCell<Device>>) -> Option<Rc<RefCell<Device>>> {
    let mut dev = dev;

    /*
     * There can only ever be one virtio bus per parent device, so we can
     * safely ignore any virtio buses. see
     * http://lists.linuxfoundation.org/pipermail/virtualization/2015-August/030331.html
     */
    #[allow(clippy::while_let_loop)]
    loop {
        let subsystem = match dev.borrow().get_subsystem() {
            Ok(s) => s,
            Err(_) => break,
        };

        if subsystem != "virtio" {
            break;
        }

        let parent = match dev.borrow().get_parent() {
            Ok(p) => p,
            Err(_) => return None,
        };

        dev = parent;
    }

    Some(dev)
}

fn get_virtfn_info(pcidev: Rc<RefCell<Device>>) -> Result<(Rc<RefCell<Device>>, String)> {
    let syspath = pcidev.borrow().get_syspath().context(DeviceSnafu)?;

    /* Get physical function's pci device. */
    let physfn_pcidev = pcidev.borrow().get_child("physfn").context(DeviceSnafu)?;
    let mut suffix = "";

    /* Find the virtual function number by finding the right virtfn link. */
    for (subdir, child) in &physfn_pcidev.borrow().child_iter() {
        /* Only accepts e.g. virtfn0, virtfn1, and so on. */
        if subdir.starts_with("virtfn") {
            suffix = subdir.trim_start_matches("virtfn");
            if suffix.is_empty() || suffix.parse::<u32>().is_err() {
                continue;
            }
        }

        let child_syspath = match child.borrow().get_syspath() {
            Ok(s) => s,
            Err(_) => continue,
        };

        if syspath == child_syspath {
            return Ok((child.clone(), format!("v{}", suffix)));
        }
    }

    Err(Error::Nix {
        source: nix::Error::ENOENT,
    })
}

fn is_valid_onboard_index(idx: u32) -> bool {
    /*
     * Some BIOSes report rubbish indexes that are excessively high (2^24-1 is an index VMware likes to
     * report for example). Let's define a cut-off where we don't consider the index reliable anymore. We
     * pick some arbitrary cut-off, which is somewhere beyond the realistic number of physical network
     * interface a system might have. Ideally the kernel would already filter this crap for us, but it
     * doesn't currently. The initial cut-off value (2^14-1) was too conservative for s390 PCI which
     * allows for index values up 2^16-1 which is now enabled with the NAMING_16BIT_INDEX naming flag.
     */
    idx <= if naming_scheme_has(NamingSchemeFlags::ONBOARD_16BIT_INDEX) {
        ONBOARD_16BIT_INDEX_MAX
    } else {
        ONBOARD_14BIT_INDEX_MAX
    }
}

/// Retrieve on-board index number and label from firmware
fn dev_pci_onboard(dev: Rc<RefCell<Device>>, info: &LinkInfo, names: &mut NetNames) -> Result<()> {
    let mut dev_port: u32 = 0;

    let attr = match names.pcidev.borrow().get_sysattr_value("acpi_index") {
        Ok(v) => {
            log_dev!(debug, names.pcidev.borrow(), format!("acpi_index={}", v));
            v
        }
        Err(_) => {
            let v = names
                .pcidev
                .borrow()
                .get_sysattr_value("index")
                .context(DeviceSnafu)?;
            log_dev!(debug, names.pcidev.borrow(), format!("index={}", v));
            v
        }
    };

    let idx = attr.parse::<u32>().context(ParseIntSnafu).log_dev_debug(
        &names.pcidev.borrow(),
        &format!("Failed to parse '{}'", attr),
    )?;

    if idx == 0 && !naming_scheme_has(NamingSchemeFlags::ZERO_ACPI_INDEX) {
        log_dev!(
            debug,
            names.pcidev.borrow(),
            "Naming scheme does not allow onboard index==0"
        );
        return Err(Error::Nix {
            source: nix::Error::EINVAL,
        });
    }

    if !is_valid_onboard_index(idx) {
        log_dev!(
            debug,
            names.pcidev.borrow(),
            format!("Not a valid onboard index: {}", idx)
        );
        return Err(Error::Nix {
            source: nix::Error::ENOENT,
        });
    }

    if let Ok(v) = dev.borrow().get_sysattr_value("dev_port") {
        if let Ok(n) = v
            .parse::<u32>()
            .context(ParseIntSnafu)
            .log_dev_debug(&dev.borrow(), "Failed to parse dev_port, ignoring")
        {
            dev_port = n;
        }
        log_dev!(debug, dev.borrow(), format!("dev_port={}", dev_port));
    }

    names.pci_onboard.push_str(&format!("o{}", idx));
    if !info.physical_port_name.is_empty() {
        names
            .pci_onboard
            .push_str(&format!("n{}", info.physical_port_name));
    } else if dev_port > 0 {
        names.pci_onboard.push_str(&format!("{}", dev_port));
    }

    log_dev!(
        debug,
        dev.borrow(),
        format!(
            "Onboard index identifier: index={} phys_port={} dev_port={} >>> {}",
            idx, info.physical_port_name, dev_port, names.pci_onboard
        )
    );

    if let Ok(v) = names.pcidev.borrow().get_sysattr_value("label") {
        names.pci_onboard_label = v;
        log_dev!(
            debug,
            dev.borrow(),
            format!("Onboard label from PCI device: {}", names.pci_onboard_label)
        );
    } else {
        names.pci_onboard_label.clear();
    }

    Ok(())
}

/// Read the 256 bytes PCI configuration space to check the multi-function bit
fn is_pci_multifunction(dev: Rc<RefCell<Device>>) -> Result<bool> {
    let syspath = dev.borrow().get_syspath().context(DeviceSnafu)?;
    let filename = format!("{}/config", syspath);
    let config = std::fs::read(&filename).context(IoSnafu { filename })?;

    const PCI_HEADER_TYPE_MULTIFUNC: u8 = 0x80;

    /* Bit 0-6 header type, bit 7 multi/single function device */
    Ok(config[PCI_HEADER_TYPE as usize] & PCI_HEADER_TYPE_MULTIFUNC != 0)
}

fn is_pci_ari_enabled(dev: Rc<RefCell<Device>>) -> bool {
    let attr = match dev.borrow().get_sysattr_value("ari_enabled") {
        Ok(v) => v,
        Err(_) => return false,
    };

    &attr == "1"
}

fn is_pci_bridge(dev: Rc<RefCell<Device>>) -> bool {
    let modalias = match dev.borrow().get_sysattr_value("modalias") {
        Ok(v) => v,
        Err(_) => return false,
    };

    if !modalias.starts_with("pci:") {
        return false;
    }

    let idx = match modalias.rfind('s') {
        Some(idx) => match modalias.chars().nth(idx + 1) {
            Some(v) => {
                if v != 'c' {
                    return false;
                }
                idx
            }
            None => return false,
        },
        None => {
            return false;
        }
    };

    /* PCI device subclass 04 corresponds to PCI bridge */
    // modalias[idx+2:2
    let pci_subclass = match modalias.get(idx + 2..idx + 5) {
        Some(s) => s,
        None => return false,
    };

    if pci_subclass == "04" {
        log_dev!(debug, dev.borrow(), "Device is a PCI bridge");
        return true;
    }

    false
}

fn parse_hotplug_slot_from_function_id(
    dev: Rc<RefCell<Device>>,
    slots_dirfd: RawFd,
) -> Result<u32> {
    if !naming_scheme_has(NamingSchemeFlags::SLOT_FUNCTION_ID) {
        return Ok(0);
    }

    /*
     * The <sysname>/function_id attribute is unique to the s390 PCI driver. If present, we know
     * that the slot's directory name for this device is /sys/bus/pci/XXXXXXXX/ where XXXXXXXX is
     * the fixed length 8 hexadecimal character string representation of function_id. Therefore we
     * can short cut here and just check for the existence of the slot directory. As this directory
     * has to exist, we're emitting a debug message for the unlikely case it's not found. Note that
     * the domain part doesn't belong to the slot name here because there's a 1-to-1 relationship
     * between PCI function and its hotplug slot.
     */
    let attr = match dev.borrow().get_sysattr_value("function_id") {
        Ok(v) => v,
        Err(_) => return Ok(0),
    };

    let function_id = attr.parse::<u64>().context(ParseIntSnafu).log_dev_debug(
        &dev.borrow(),
        &format!("Failed to parse function_id, ignoring '{}'", attr),
    )?;

    if function_id > u32::MAX.into() {
        log_dev!(
            debug,
            dev.borrow(),
            format!("Invalid function id '{}', ignoring", function_id)
        );

        return Err(Error::Nix {
            source: nix::errno::Errno::EINVAL,
        });
    }

    if function_id.to_string().len() > 8 {
        log_dev!(
            warn,
            dev.borrow(),
            format!("function_id '{}' is too long, ignoring", function_id)
        );
        return Err(Error::Nix {
            source: nix::errno::Errno::ENAMETOOLONG,
        });
    }

    let filename = format!("{:08}", function_id);

    if unsafe { faccessat(slots_dirfd, filename.as_ptr() as *const c_char, F_OK, 0) } < 0 {
        log_dev!(
            debug,
            dev.borrow(),
            format!(
                "Cannot access '{}' under pci slots, ignoring: {}",
                filename,
                errno()
            )
        );
        return Err(Error::Nix {
            source: nix::Error::from_i32(errno()),
        });
    }

    Ok(function_id as u32)
}

fn dev_pci_slot(dev: Rc<RefCell<Device>>, info: &LinkInfo, names: &mut NetNames) -> Result<()> {
    let mut hotplug_slot: u32 = 0;
    let mut dev_port: u32 = 0;
    let sysname = names
        .pcidev
        .borrow()
        .get_sysname()
        .context(DeviceSnafu)
        .log_dev_error(&dev.borrow(), "Failed to get sysname")?;

    let (mut domain, bus, slot, mut func) =
        sscanf::sscanf!(&sysname, "{:x}:{:x}:{:x}.{}", u16, u8, u8, u8)
            .context(SscanfSnafu)
            .log_dev_debug(
                &dev.borrow(),
                "Failed to parse slot information from PCI device sysname",
            )?;

    log_dev!(
        debug,
        dev.borrow(),
        format!(
            "Parsing slot information from PCI device sysname '{}'",
            sysname
        )
    );

    if naming_scheme_has(NamingSchemeFlags::NPAR_ARI) && is_pci_ari_enabled(names.pcidev.clone()) {
        /*
         * ARI devices support up to 256 functions on a single device ("slot"), and interpret the
         * traditional 5-bit slot and 3-bit function number as a single 8-bit function number,
         * where the slot makes up the upper 5 bits.
         */
        func += slot * 8;
    }
    /* kernel provided port index for multiple ports on a single PCI function */
    if let Ok(attr) = dev.borrow().get_sysattr_value("dev_port") {
        log_dev!(debug, dev.borrow(), format!("dev_port={}", attr));

        dev_port = attr.parse::<u32>().context(ParseIntSnafu).log_dev_debug(
            &dev.borrow(),
            &format!("Failed to parse attribute dev_port '{}', ignoring", attr),
        )?;

        /*
         * With older kernels IP-over-InfiniBand network interfaces sometimes erroneously
         * provide the port number in the 'dev_id' sysfs attribute instead of 'dev_port',
         * which thus stays initialized as 0.
         */
        if dev_port == 0 && info.iftype == ARPHRD_INFINIBAND {
            if let Ok(attr) = dev.borrow().get_sysattr_value("dev_id") {
                log_dev!(debug, dev.borrow(), format!("dev_id={}", attr));

                dev_port = attr.parse::<u32>().context(ParseIntSnafu).log_dev_debug(
                    &dev.borrow(),
                    &format!("Failed to parse attribute dev_id '{}', ignoring", attr),
                )?;
            }
        }
    }
    if domain > 0 {
        names.pci_path.push_str(&format!("P{}", domain));
    }
    names.pci_path.push_str(&format!("p{}s{}", bus, slot));
    if func > 0 {
        names.pci_path.push_str(&format!("f{}", func));
    } else if let Ok(true) = is_pci_multifunction(names.pcidev.clone()) {
        names.pci_path.push_str(&format!("f{}", func));
    }
    if !info.physical_port_name.is_empty() {
        /* Kernel provided front panel port name for multi-port PCI device */
        names
            .pci_path
            .push_str(&format!("n{}", info.physical_port_name));
    } else if dev_port > 0 {
        names.pci_path.push_str(&format!("d{}", dev_port));
    }

    log_dev!(
        debug,
        dev.borrow(),
        format!(
            "PCI path identifier: domain={} bus={} slot={} func={} phys_port={} dev_port={} >>> {}",
            domain, bus, slot, func, info.physical_port_name, dev_port, names.pci_path
        )
    );

    /* ACPI _SUN — slot user number */
    let pci = Device::from_subsystem_sysname("subsystem", "pci")
        .context(DeviceSnafu)
        .log_debug("Device::from_subsystem_sysname subsystem pci failed")?;

    let dir = pci
        .open_dir("slots")
        .context(DeviceSnafu)
        .log_dev_debug(&pci, "Failed to open 'slots' subdirectory")?;

    let mut hotplug_slot_dev = names.pcidev.clone();

    loop {
        match parse_hotplug_slot_from_function_id(hotplug_slot_dev.clone(), dir.as_raw_fd()) {
            Ok(r) => {
                if r > 0 {
                    domain = 0;
                    break;
                }
            }
            Err(_) => return Ok(()),
        }

        let sysname = hotplug_slot_dev
            .borrow()
            .get_sysname()
            .context(DeviceSnafu)
            .log_dev_debug(&dev.borrow(), "Failed to get sysname")?;

        let read_dir = pci
            .read_dir("slots")
            .context(DeviceSnafu)
            .log_dev_debug(&pci, "Failed to read 'slots' subdirectory")?;

        for de in read_dir {
            let de = match de {
                Ok(de) => de,
                Err(_) => continue,
            };

            let dname = match de.file_name().to_str() {
                Some(s) => {
                    if [".", ".."].contains(&s) {
                        continue;
                    }

                    s.to_string()
                }
                None => continue,
            };

            let i = match dname.parse::<u32>() {
                Ok(v) => v,
                Err(_) => continue,
            };

            let path = format!("slots/{}/address", dname);
            let address = match pci.get_sysattr_value(&path) {
                Ok(v) => v,
                Err(_) => continue,
            };

            /* Match slot address with device by stripping the function */
            if !sysname.starts_with(&address) {
                continue;
            }

            hotplug_slot = i;

            /*
             * We found the match between PCI device and slot. However, we won't use the slot
             * index if the device is a PCI bridge, because it can have other child devices that
             * will try to claim the same index and that would create name collision.
             */
            if naming_scheme_has(NamingSchemeFlags::BRIDGE_NO_SLOT)
                && is_pci_bridge(hotplug_slot_dev.clone())
            {
                if naming_scheme_has(NamingSchemeFlags::BRIDGE_MULTIFUNCTION_SLOT)
                    && !is_pci_multifunction(names.pcidev.clone())
                        .or::<Result<bool>>(Ok(false))
                        .unwrap()
                {
                    log_dev!(
                        debug,
                        dev.borrow(),
                        "Not using slot information because the PCI device associated with the hotplug slot is a bridge and the PCI device has a single function."
                    );
                    return Ok(());
                }

                if !naming_scheme_has(NamingSchemeFlags::BRIDGE_MULTIFUNCTION_SLOT) {
                    log_dev!(
                        debug,
                        dev.borrow(),
                        "Not using slot information because the PCI device is a bridge."
                    );
                    return Ok(());
                }
            }

            break;
        }
        if hotplug_slot > 0 {
            break;
        }

        let parent = match hotplug_slot_dev
            .borrow()
            .get_parent_with_subsystem_devtype("pci", None)
        {
            Ok(d) => d,
            Err(_) => break,
        };

        hotplug_slot_dev = parent;
    }

    if hotplug_slot > 0 {
        if domain > 0 {
            names.pci_slot.push_str(&format!("P{}", domain));
        }

        names.pci_slot.push_str(&format!("s{}", hotplug_slot));
        if func > 0
            || is_pci_multifunction(names.pcidev.clone())
                .or::<Result<bool>>(Ok(false))
                .unwrap()
        {
            names.pci_slot.push_str(&format!("f{}", func));
        }
        if !info.physical_port_name.is_empty() {
            names
                .pci_slot
                .push_str(&format!("n{}", info.physical_port_name));
        } else if dev_port > 0 {
            names.pci_slot.push_str(&format!("d{}", dev_port));
        }

        log_dev!(
            debug,
            dev.borrow(),
            format!(
                "Slot identifier: domain={} slot={} func={} phys_port={} dev_port={} >>> {}",
                domain, hotplug_slot, func, info.physical_port_name, dev_port, names.pci_slot
            )
        );
    }

    Ok(())
}

fn names_vio(dev: Rc<RefCell<Device>>, names: &mut NetNames) -> Result<()> {
    /* Check if our direct parent is a VIO device with no other bus in-between */
    dev.borrow()
        .get_parent()
        .context(DeviceSnafu)
        .log_dev_debug(&dev.borrow(), "failed to get parent")?;

    let subsystem = dev
        .borrow()
        .get_subsystem()
        .context(DeviceSnafu)
        .log_dev_debug(&dev.borrow(), "failed to get subsystem")?;

    if "vio" != &subsystem {
        return Err(Error::Nix {
            source: nix::Error::ENOENT,
        });
    }

    log_dev!(debug, dev.borrow(), "Parent device is in the vio subsystem");

    /*
     * The devices' $DEVPATH number is tied to (virtual) hardware (slot id
     * selected in the HMC), thus this provides a reliable naming (e.g.
     * "/devices/vio/30000002/net/eth1"); we ignore the bus number, as
     * there should only ever be one bus, and then remove leading zeros.
     */
    let syspath = dev
        .borrow()
        .get_syspath()
        .context(DeviceSnafu)
        .log_dev_debug(&dev.borrow(), "failed to get syspath")?;

    let (busid, slotid, _ethid) = sscanf::sscanf!(
        &syspath,
        r"/sys/devices/vio/{str:/.{4}/}{str:/.{4}/}/net/eth{u8}"
    )
    .context(SscanfSnafu)
    .log_dev_debug(
        &dev.borrow(),
        &format!("Parsing vio slot information from syspath '{}'", syspath),
    )?;
    let _busid = u32::from_str_radix(busid, 16).context(ParseIntSnafu)?;
    let slotid = u32::from_str_radix(slotid, 16).context(ParseIntSnafu)?;
    log_dev!(
        debug,
        dev.borrow(),
        format!("Parsing vio slot information from syspath '{}'", syspath)
    );

    names.vio_slot = format!("v{}", slotid);
    names.r#type = NetNameType::Vio;
    log_dev!(
        debug,
        dev.borrow(),
        format!(
            "Vio slot identifier: slotid={} >>> {}",
            slotid, names.vio_slot
        )
    );

    Ok(())
}

const PLATFORM_TEST: &str = "/sys/devices/platform/aaaaBBBB";

fn names_platform(dev: Rc<RefCell<Device>>, names: &mut NetNames, _test: bool) -> Result<()> {
    /* Check if our direct parent is a platform device with no other bus in-between */
    let parent = dev
        .borrow()
        .get_parent()
        .context(DeviceSnafu)
        .log_dev_debug(&dev.borrow(), "failed to get parent")?;

    let subsystem = parent
        .borrow()
        .get_subsystem()
        .context(DeviceSnafu)
        .log_dev_debug(&parent.borrow(), "failed to get subsystem")?;

    if &subsystem != "platform" {
        return Err(Error::Nix {
            source: nix::Error::ENOENT,
        });
    }

    log_dev!(
        debug,
        dev.borrow(),
        "Parent device is in the platform subsystem"
    );

    let syspath = dev
        .borrow()
        .get_syspath()
        .context(DeviceSnafu)
        .log_dev_debug(&dev.borrow(), "failed to get syspath")?;

    if syspath.len() < PLATFORM_TEST.len() + 1 {
        log_dev!(
            debug,
            dev.borrow(),
            format!(
                "The syspath '{}' is too short for a valid ACPI instance",
                syspath
            )
        );
        return Err(Error::Nix {
            source: nix::Error::EINVAL,
        });
    }

    /*
     * Vendor ID can be either PNP ID (3 chars A-Z) or ACPI ID (4 chars A-Z and numerals)
     *
     * Platform devices are named after ACPI table match, and instance id
     * eg. "/sys/devices/platform/HISI00C2:00");
     * The Vendor (3 or 4 char), followed by hexadecimal model number : instance id.
     */
    let (vendor, model, instance, ethid, validchars) =
        if syspath.chars().nth(PLATFORM_TEST.len()).unwrap() == ':' {
            let (vendor, model, instance, ethid) = sscanf::scanf!(
                &syspath,
                "/sys/devices/platform/{str:/.{4}/}{str:/.{4}/}:{str:/.{2}/}/net/eth{u8}"
            )
            .context(SscanfSnafu)
            .log_dev_debug(
                &dev.borrow(),
                &format!(
                    "Parsing platform device information from syspath '{}'",
                    syspath
                ),
            )?;

            (
                vendor,
                model,
                instance,
                ethid,
                "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
            )
        } else {
            let (vendor, model, instance, ethid) = sscanf::scanf!(
                &syspath,
                "/sys/devices/platform/{str:/.{3}/}{str:/.{4}/}:{str:/.{2}/}/net/eth{u8}"
            )
            .context(SscanfSnafu)
            .log_dev_debug(
                &dev.borrow(),
                &format!(
                    "Parsing platform device information from syspath '{}'",
                    syspath
                ),
            )?;
            (vendor, model, instance, ethid, "ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        };

    let (vendor, model, instance, _ethid) = (
        vendor,
        u32::from_str_radix(model, 16)
            .context(ParseIntSnafu)
            .log_dev_debug(&dev.borrow(), &format!("invalid model '{}'", model))?,
        u32::from_str_radix(instance, 16)
            .context(ParseIntSnafu)
            .log_dev_debug(&dev.borrow(), &format!("invalid instance '{}'", instance))?,
        ethid,
    );

    if !vendor.chars().all(|c| validchars.contains(c)) {
        log_dev!(
            debug,
            dev.borrow(),
            format!("Platform vendor contains invalid characters: {}", vendor)
        );
    }

    let vendor = vendor.to_lowercase();

    names.platform_path = format!("a{}{:x}i{}", vendor, model, instance);
    names.r#type = NetNameType::Platform;

    log_dev!(
        debug,
        dev.borrow(),
        format!(
            "Platform identifier: vendor={} model={} instance={} >>> {}",
            vendor, model, instance, names.platform_path
        )
    );

    Ok(())
}

fn dev_devicetree_onboard(dev: Rc<RefCell<Device>>, names: &mut NetNames) -> Result<()> {
    if !naming_scheme_has(NamingSchemeFlags::DEVICETREE_ALIASES) {
        return Ok(());
    }

    /* Check if our direct parent has an of_node */
    let parent = dev.borrow().get_parent().context(DeviceSnafu)?;
    let ofnode_dev = parent.borrow().get_child("of_node").context(DeviceSnafu)?;
    let ofnode_syspath = ofnode_dev.borrow().get_syspath().context(DeviceSnafu)?;

    /* /proc/device-tree should be a symlink to /sys/firmware/devicetree/base. */
    let devicetree_dev = Device::from_path("/proc/device-tree").context(DeviceSnafu)?;
    let devicetree_syspath = devicetree_dev.get_syspath().context(DeviceSnafu)?;

    /*
     * Example paths:
     * devicetree_syspath = /sys/firmware/devicetree/base
     * ofnode_syspath = /sys/firmware/devicetree/base/soc/ethernet@deadbeef
     * ofnode_path = soc/ethernet@deadbeef
     */
    let ofnode_path = match ofnode_syspath.strip_prefix(&devicetree_syspath) {
        Some(v) => v,
        None => {
            return Err(Error::Nix {
                source: nix::Error::ENOENT,
            });
        }
    };

    debug_assert!(ofnode_path.starts_with('/'));

    let aliases_dev = devicetree_dev.get_child("aliases").context(DeviceSnafu)?;

    for alias in &aliases_dev.borrow().sysattr_iter() {
        let alias_index = match alias.strip_prefix("ethernet") {
            Some(suffix) => suffix,
            None => continue,
        };

        let alias_path = match aliases_dev.borrow().get_sysattr_value(alias) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if ofnode_path != alias_path {
            continue;
        }

        /* If there's no index, we default to 0... */
        let (i, conflict) = if alias_index.is_empty() {
            (0, "ethernet0")
        } else {
            let i = alias_index
                .parse::<u32>()
                .context(ParseIntSnafu)
                .log_dev_debug(
                    &dev.borrow(),
                    &format!("Could not get index of alias '{}'", alias),
                )?;
            (i, "ethernet")
        };

        /* ...but make sure we don't have an alias conflict */
        if i == 0 && aliases_dev.borrow().get_sysattr_value(conflict).is_ok() {
            log_dev!(
                debug,
                dev.borrow(),
                "Ethernet alias conflict: ethernet and ethernet0 both exist"
            );
            return Err(Error::Nix {
                source: nix::Error::EEXIST,
            });
        }

        names.devicetree_onboard = format!("d{}", i);
        names.r#type = NetNameType::Devicetree;
        return Ok(());
    }

    Err(Error::Nix {
        source: nix::Error::ENOENT,
    })
}

#[allow(clippy::unnecessary_unwrap)]
fn names_pci(dev: Rc<RefCell<Device>>, info: &LinkInfo, names: &mut NetNames) -> Result<()> {
    let parent = dev.borrow().get_parent().context(DeviceSnafu)?;

    /* Skip virtio subsystem if present */
    let parent = skip_virtio(parent);

    if parent.is_none() {
        return Err(Error::Nix {
            source: nix::Error::ENOENT,
        });
    }

    let parent = parent.unwrap();

    /* Check if our direct parent is a PCI device with no other bus in-between */
    match parent.borrow().get_subsystem() {
        Ok(s) => {
            if &s == "pci" {
                names.r#type = NetNameType::Pci;
                names.pcidev = parent.clone();
            }
        }
        Err(_) => {
            names.pcidev = dev
                .borrow()
                .get_parent_with_subsystem_devtype("pci", None)
                .context(DeviceSnafu)?;
        }
    };

    let ret = get_virtfn_info(names.pcidev.clone());

    if naming_scheme_has(NamingSchemeFlags::SR_IOV_V) && ret.is_ok() {
        let (physfn_pcidev, virtfn_suffix) = ret.unwrap();

        let mut vf_names = NetNames::new(physfn_pcidev);

        /* If this is an SR-IOV virtual device, get base name using physical device and add virtfn suffix. */
        let _ = dev_pci_onboard(dev.clone(), info, &mut vf_names);
        let _ = dev_pci_slot(dev, info, &mut vf_names);

        if !vf_names.pci_onboard.is_empty() {
            if vf_names.pci_onboard.len() + virtfn_suffix.len() < names.pci_onboard.len() {
                names.pci_onboard = format!("{}{}", vf_names.pci_onboard, virtfn_suffix);
            }

            if vf_names.pci_slot.len() + virtfn_suffix.len() < names.pci_slot.len() {
                names.pci_slot = format!("{}{}", vf_names.pci_slot, virtfn_suffix);
            }

            if vf_names.pci_path.len() + virtfn_suffix.len() < names.pci_path.len() {
                names.pci_path = format!("{}{}", vf_names.pci_path, virtfn_suffix);
            }
        }
    } else {
        let _ = dev_pci_onboard(dev.clone(), info, names);
        let _ = dev_pci_slot(dev, info, names);
    }

    Ok(())
}

fn names_usb(dev: Rc<RefCell<Device>>, names: &mut NetNames) -> Result<()> {
    let usbdev = dev
        .borrow()
        .get_parent_with_subsystem_devtype("usb", Some("usb_interface"))
        .context(DeviceSnafu)
        .log_dev_debug(
            &dev.borrow(),
            "Failed to get parent with subsystem 'usb' and devtype 'usb_interface'",
        )?;

    let sysname = usbdev
        .borrow()
        .get_sysname()
        .context(DeviceSnafu)
        .log_dev_debug(&usbdev.borrow(), "Failed to get parent sysname")?;

    /* Get USB port number chain, configuration, interface */
    let name = sysname.clone();
    let idx_1 = name.find('-').ok_or_else(|| {
        log_dev!(
            debug,
            usbdev.borrow(),
            format!("sysname '{}' does not have '-' as expected", sysname)
        );
        Error::Nix {
            source: nix::Error::EINVAL,
        }
    })?;

    let ports = name.get(idx_1 + 1..).ok_or_else(|| {
        log_dev!(
            debug,
            usbdev.borrow(),
            format!("sysname '{}' does not contain ports", sysname)
        );
        Error::Nix {
            source: nix::Error::EINVAL,
        }
    })?;

    let idx_2 = ports.find(':').ok_or_else(|| {
        log_dev!(
            debug,
            usbdev.borrow(),
            format!("sysname '{}' does not have ':' as expected", sysname)
        );
        Error::Nix {
            source: nix::Error::EINVAL,
        }
    })?;

    let ports = name.get(idx_1 + 1..idx_2).unwrap().to_string();

    let config = name.get(idx_2 + 1..).ok_or_else(|| {
        log_dev!(
            debug,
            usbdev.borrow(),
            format!("sysname '{}' does not contain config", sysname)
        );
        Error::Nix {
            source: nix::Error::EINVAL,
        }
    })?;

    let idx_3 = config.find('.').ok_or_else(|| {
        log_dev!(
            debug,
            usbdev.borrow(),
            format!("sysname '{}' does not have '.' as expected", sysname)
        );
        Error::Nix {
            source: nix::Error::EINVAL,
        }
    })?;

    let config = name.get(idx_2 + 1..idx_3).unwrap().to_string();
    let interf = name
        .get(idx_3 + 1..)
        .ok_or_else(|| {
            log_dev!(
                debug,
                usbdev.borrow(),
                format!("sysname '{}' does not contain interface", sysname)
            );
            Error::Nix {
                source: nix::Error::EINVAL,
            }
        })?
        .to_string();

    /* Prefix every port number in the chain with "u" */
    let ports = ports.replace('.', "u");
    names.usb_ports = format!("u{}", ports);

    /* Append USB config number, suppress the common config == 1 */
    if config != "1" {
        names.usb_ports.push_str(&format!("c{}", config));
    }

    if interf != "0" {
        names.usb_ports.push_str(&format!("i{}", interf));
    }

    log_dev!(
        debug,
        dev.borrow(),
        format!(
            "USB name identifier: ports={} config={} interface={} >>> {}",
            ports, config, interf, names.usb_ports
        )
    );

    names.r#type = NetNameType::Usb;

    Ok(())
}

fn names_bcma(dev: Rc<RefCell<Device>>, names: &mut NetNames) -> Result<()> {
    let bcmadev = dev
        .borrow()
        .get_parent_with_subsystem_devtype("bcma", None)
        .context(DeviceSnafu)
        .log_dev_debug(&dev.borrow(), "Failed to get parent with subsystem 'bcma'")?;

    let sysname = bcmadev
        .borrow()
        .get_sysname()
        .context(DeviceSnafu)
        .log_dev_debug(&dev.borrow(), "Failed to get bcma device sysname")?;

    /* Bus num:core num */
    let (_bus, core) = sscanf::sscanf!(&sysname, "bcma{u8}:{u8}")
        .context(SscanfSnafu)
        .log_dev_debug(
            &dev.borrow(),
            &format!(
                "Parsing bcmadevice information from sysname '{}' failed",
                sysname
            ),
        )?;
    log_dev!(
        debug,
        dev.borrow(),
        format!("Parsing bcma device information from sysname '{}'", sysname)
    );

    /* Suppress the common core == 0 */
    if core > 0 {
        names.bcma_core = format!("b{}", core);
    }

    names.r#type = NetNameType::Bcma;

    log_dev!(
        debug,
        dev.borrow(),
        format!(
            "BCMA core identifier: core={} >>> {}",
            core, names.bcma_core
        )
    );

    Ok(())
}

fn names_ccw(dev: Rc<RefCell<Device>>, names: &mut NetNames) -> Result<()> {
    /* Retrieve the associated CCW device */
    let _ = dev
        .borrow()
        .get_parent()
        .context(DeviceSnafu)
        .log_dev_debug(&dev.borrow(), "Failed to get parent")?;

    /* Skip virtio subsystem if present */
    let cdev = match skip_virtio(dev.clone()) {
        Some(d) => d,
        None => {
            return Err(Error::Nix {
                source: nix::Error::ENOENT,
            });
        }
    };

    let subsys = cdev
        .borrow()
        .get_subsystem()
        .context(DeviceSnafu)
        .log_dev_debug(&cdev.borrow(), "Failed to get subsystem")?;

    /* Network devices are either single or grouped CCW devices */
    if !["ccwgroup", "ccw"].contains(&subsys.as_str()) {
        return Err(Error::Nix {
            source: nix::Error::ENOENT,
        });
    }

    log_dev!(debug, dev.borrow(), "Device is CCW");

    /*
     * Retrieve bus-ID of the CCW device.  The bus-ID uniquely
     * identifies the network device on the Linux on System z channel
     * subsystem.  Note that the bus-ID contains lowercase characters.
     */
    let bus_id = cdev
        .borrow()
        .get_sysname()
        .context(DeviceSnafu)
        .log_dev_debug(&cdev.borrow(), "Failed to get sysname")?;

    /*
     * Check the length of the bus-ID. Rely on the fact that the kernel provides a correct bus-ID;
     * alternatively, improve this check and parse and verify each bus-ID part...
     */
    let bus_id_len = bus_id.len();
    if ![8, 9].contains(&bus_id_len) {
        log_dev!(debug, cdev.borrow(), format!("Invalid bus_id '{}'", bus_id));
        return Err(Error::Nix {
            source: nix::Error::EINVAL,
        });
    }

    /*
     * Strip leading zeros from the bus id for aesthetic purposes. This
     * keeps the ccw names stable, yet much shorter in general case of
     * bus_id 0.0.0600 -> 600. This is similar to e.g. how PCI domain is
     * not prepended when it is zero. Preserve the last 0 for 0.0.0000.
     */
    let bus_id_strip = match bus_id.strip_prefix(['0', '.']) {
        Some(s) => s.to_string(),
        None => "".to_string(),
    };

    /* Store the CCW bus-ID for use as network device name */
    names.ccw_busid = format!("c{}", bus_id_strip);

    names.r#type = NetNameType::Ccw;

    log_dev!(
        debug,
        dev.borrow(),
        format!(
            "CCW identifier: ccw_busid={} >>> {}",
            bus_id, names.ccw_busid
        )
    );

    Ok(())
}

fn names_mac(dev: Rc<RefCell<Device>>, info: &LinkInfo) -> Result<()> {
    /*
     * The persistent part of a hardware address of an InfiniBand NIC is 8 bytes long. We cannot
     * fit this much in an iface name.
     * TODO: but it can be used as alternative names??
     */

    if info.iftype == ARPHRD_INFINIBAND {
        log_dev!(
            debug,
            dev.borrow(),
            "Not generating MAC name for infiniband device"
        );
        return Err(Error::Nix {
            source: nix::Error::EOPNOTSUPP,
        });
    }

    if info.hw_addr.length as u8 != 6 {
        log_dev!(
            debug,
            dev.borrow(),
            format!(
                "Not generating MAC name for device with MAC address of length {}",
                info.hw_addr.length as u8
            )
        );
        return Err(Error::Nix {
            source: nix::Error::EOPNOTSUPP,
        });
    }

    /* Check for NET_ADDR_PERM, skip random MAC addresses */
    let s = dev
        .borrow()
        .get_sysattr_value("addr_assign_type")
        .context(DeviceSnafu)
        .log_dev_debug(&dev.borrow(), "Failed to read addr_assign_type")?;

    let i = s
        .parse::<u32>()
        .context(ParseIntSnafu)
        .log_dev_debug(&dev.borrow(), "Failed to parse addr_assign_type number")?;

    if i != HwAddrAssignType::Permanent as u32 {
        log_dev!(
            debug,
            dev.borrow(),
            format!("addr_assign_type={}, MAC address is not permant", i)
        );
        return Err(Error::Nix {
            source: nix::Error::EINVAL,
        });
    }

    Ok(())
}

fn names_netdevsim(dev: Rc<RefCell<Device>>, info: &LinkInfo, names: &mut NetNames) -> Result<()> {
    if !naming_scheme_has(NamingSchemeFlags::NETDEVSIM) {
        return Ok(());
    }

    if info.physical_port_name.is_empty() {
        return Err(Error::Nix {
            source: nix::Error::EINVAL,
        });
    }

    let netdevsimdev = dev
        .borrow()
        .get_parent_with_subsystem_devtype("netdevsim", None)
        .context(DeviceSnafu)?;

    let sysname = netdevsimdev.borrow().get_sysname().context(DeviceSnafu)?;

    let addr = sscanf::sscanf!(&sysname, "netdevsim{}", u8).context(SscanfSnafu)?;

    names.netdevsim_path = format!("i{}n{}", addr, info.physical_port_name);
    names.r#type = NetNameType::Netdevsim;

    Ok(())
}

fn names_xen(dev: Rc<RefCell<Device>>, names: &mut NetNames) -> Result<()> {
    if !naming_scheme_has(NamingSchemeFlags::XEN_VIF) {
        return Ok(());
    }

    /* Check if our direct parent is a Xen VIF device with no other bus in-between */
    let parent = dev.borrow().get_parent().context(DeviceSnafu)?;

    /*
     * Do an exact-match on subsystem "xen". This will miss on "xen-backend" on
     * purpose as the VIFs on the backend (dom0) have their own naming scheme
     * which we don't want to affect
     */
    let subsystem = parent.borrow().get_subsystem().context(DeviceSnafu)?;

    if "xen" != &subsystem {
        return Err(Error::Nix {
            source: nix::Error::ENOENT,
        });
    }

    /* Use the vif-n name to extract "n" */
    let syspath = dev.borrow().get_syspath().context(DeviceSnafu)?;

    let p = match syspath.strip_prefix("/sys/devices/") {
        Some(p) => p,
        None => {
            return Err(Error::Nix {
                source: nix::Error::ENOENT,
            });
        }
    };

    let p = match p.strip_prefix("vif-") {
        Some(p) => p,
        None => {
            return Err(Error::Nix {
                source: nix::Error::ENOENT,
            });
        }
    };

    let idx = match p.find('/') {
        Some(i) => i,
        None => {
            return Err(Error::Nix {
                source: nix::Error::ENOENT,
            });
        }
    };

    let id = match p.get(..idx) {
        Some(s) => s.parse::<u32>().context(ParseIntSnafu)?,
        None => {
            return Err(Error::Nix {
                source: nix::Error::ENOENT,
            });
        }
    };

    names.xen_slot = format!("X{}", id);
    names.r#type = NetNameType::Xenvif;

    Ok(())
}

/// IEEE Organizationally Unique Identifier vendor string
fn ieee_oui(_dev: Rc<RefCell<Device>>, info: &LinkInfo, _test: bool) -> Result<()> {
    if info.hw_addr.length as u8 != 0 {
        return Err(Error::Nix {
            source: nix::Error::EOPNOTSUPP,
        });
    }

    /* Skip commonly misused 00:00:00 (Xerox) prefix */
    if info.hw_addr.bytes_ref()[0] == 0
        && info.hw_addr.bytes_ref()[1] == 0
        && info.hw_addr.bytes_ref()[2] == 0
    {
        return Err(Error::Nix {
            source: nix::Error::EINVAL,
        });
    }

    let _s = format!(
        "OUI:{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        info.hw_addr.bytes_ref()[0],
        info.hw_addr.bytes_ref()[1],
        info.hw_addr.bytes_ref()[2],
        info.hw_addr.bytes_ref()[3],
        info.hw_addr.bytes_ref()[4],
        info.hw_addr.bytes_ref()[5]
    );

    todo!("hwdb lookup s")
}

fn get_link_info(dev: Rc<RefCell<Device>>) -> Result<LinkInfo> {
    let ifindex = dev.borrow().get_ifindex().context(DeviceSnafu)?;

    let iflink = dev
        .borrow()
        .get_sysattr_value("iflink")
        .context(DeviceSnafu)?
        .parse::<u32>()
        .context(ParseIntSnafu)?;

    let iftype = dev
        .borrow()
        .get_sysattr_value("type")
        .context(DeviceSnafu)?
        .parse::<u16>()
        .context(ParseIntSnafu)?;

    let devtype = match dev.borrow().get_devtype() {
        Ok(t) => t,
        Err(e) => {
            if !e.is_errno(nix::Error::ENOENT) {
                return Err(Error::Device { source: e });
            }

            "".to_string()
        }
    };

    let physical_port_name = dev
        .borrow()
        .get_sysattr_value("phys_port_name")
        .unwrap_or_default();

    let hw_addr = match dev.borrow().get_sysattr_value("address") {
        Ok(s) => parse_hw_addr_full(&s, 0)
            .context(BasicSnafu)
            .log_dev_debug(&dev.borrow(), "Failed to parse 'address' sysattr, ignoring")?,
        Err(e) => {
            if !e.is_errno(nix::Error::ENOENT) {
                return Err(Error::Device { source: e });
            }

            HwAddress::default()
        }
    };

    let info = LinkInfo {
        ifindex,
        iflink,
        iftype,
        devtype,
        physical_port_name,
        hw_addr,
    };

    Ok(info)
}

impl Builtin for NetId {
    /// builtin command
    fn cmd(
        &self,
        exec_unit: &ExecuteUnit,
        _argc: i32,
        _argv: Vec<String>,
        test: bool,
    ) -> Result<bool> {
        let dev = exec_unit.get_device();
        let mut names = NetNames::new(Rc::new(RefCell::new(Device::default())));
        let link_info = get_link_info(dev.clone())?;

        /* Skip stacked devices, like VLANs, ... */
        if link_info.ifindex != link_info.iflink {
            return Ok(true);
        }

        /* Handle only ARPHRD_ETHER, ARPHRD_SLIP and ARPHRD_INFINIBAND devices. */
        let mut prefix = match link_info.iftype {
            libc::ARPHRD_ETHER => "en",
            libc::ARPHRD_INFINIBAND => {
                if naming_scheme_has(NamingSchemeFlags::INFINIBAND) {
                    "ib"
                } else {
                    return Ok(true);
                }
            }
            libc::ARPHRD_SLIP => "sl",
            _ => {
                return Ok(true);
            }
        };

        if &link_info.devtype == "wlan" {
            prefix = "wl";
        } else if &link_info.devtype == "wwan" {
            prefix = "ww";
        }

        let _ = self.add_property(
            dev.clone(),
            test,
            "ID_NET_NAMING_SCHEME",
            &naming_scheme().to_string(),
        );

        if names_mac(dev.clone(), &link_info).is_ok() {
            let s = format!("{}x{}", prefix, &link_info.hw_addr.to_string());

            let _ = self.add_property(dev.clone(), test, "ID_NET_NAME_MAC", &s);
            log_dev!(
                debug,
                dev.borrow(),
                format!(
                    "MAC address identifier: hw_addr={} >>> {}",
                    link_info.hw_addr,
                    s.get(prefix.len()..).unwrap()
                )
            );

            let _ = ieee_oui(dev.clone(), &link_info, test);
        }

        /* Get devicetree aliases; only ethernet supported for now.  */
        if prefix == "en"
            && dev_devicetree_onboard(dev.clone(), &mut names).is_ok()
            && names.r#type == NetNameType::Devicetree
        {
            let s = format!("{}{}", prefix, names.devicetree_onboard);
            let _ = self.add_property(dev.clone(), test, "ID_NET_NAME_ONBOARD", &s);
        }

        /* Get path names for Linux on System z network devices. */
        if names_ccw(dev.clone(), &mut names).is_ok() && names.r#type == NetNameType::Ccw {
            let s = format!("{}{}", prefix, names.ccw_busid);

            let _ = self.add_property(dev, test, "ID_NET_NAME_PATH", &s);
            return Ok(true);
        }

        /* Get ibmveth/ibmvnic slot-based names. */
        if names_vio(dev.clone(), &mut names).is_ok() && names.r#type == NetNameType::Vio {
            let s = format!("{}{}", prefix, names.vio_slot);
            let _ = self.add_property(dev, test, "ID_NET_NAME_SLOT", &s);
            return Ok(true);
        }

        /* Get ACPI path names for ARM64 platform devices. */
        if names_platform(dev.clone(), &mut names, test).is_ok()
            && names.r#type == NetNameType::Platform
        {
            let s = format!("{}{}", prefix, names.platform_path);
            let _ = self.add_property(dev, test, "ID_NET_NAME_PATH", &s);
            return Ok(true);
        }

        /* Get netdevsim path names. */
        if names_netdevsim(dev.clone(), &link_info, &mut names).is_ok()
            && names.r#type == NetNameType::Netdevsim
        {
            let s = format!("{}{}", prefix, names.netdevsim_path);
            let _ = self.add_property(dev, test, "ID_NET_NAME_PATH", &s);
            return Ok(true);
        }

        /* Get xen vif "slot" based names. */
        if names_xen(dev.clone(), &mut names).is_ok() && names.r#type == NetNameType::Xenvif {
            let s = format!("{}{}", prefix, names.xen_slot);
            let _ = self.add_property(dev, test, "ID_NET_NAME_SLOT", &s);
            return Ok(true);
        }

        /* Get PCI based path names. */
        if let Err(e) = names_pci(dev.clone(), &link_info, &mut names) {
            /*
             * Check for usb devices that are not off pci interfaces to
             * support various on-chip asics that have usb ports
             */
            if e.get_errno() == nix::Error::ENOENT
                && naming_scheme_has(NamingSchemeFlags::USB_HOST)
                && names_usb(dev.clone(), &mut names).is_ok()
                && names.r#type == NetNameType::Usb
            {
                let s = format!("{}{}", prefix, names.usb_ports);
                let _ = self.add_property(dev, test, "ID_NET_NAME_PATH", &s);
            }

            return Ok(true);
        }

        /* Plain PCI device */
        if names.r#type == NetNameType::Pci {
            if !names.pci_onboard.is_empty() {
                let s = format!("{}{}", prefix, names.pci_onboard);
                let _ = self.add_property(dev.clone(), test, "ID_NET_NAME_ONBOARD", &s);
            }

            if !names.pci_onboard_label.is_empty() {
                let s = format!(
                    "{}{}",
                    if naming_scheme_has(NamingSchemeFlags::LABEL_NOPREFIX) {
                        ""
                    } else {
                        prefix
                    },
                    names.pci_onboard_label
                );
                let _ = self.add_property(dev.clone(), test, "ID_NET_LABEL_ONBOARD", &s);
            }

            if !names.pci_path.is_empty() {
                let s = format!("{}{}", prefix, names.pci_path);
                let _ = self.add_property(dev.clone(), test, "ID_NET_NAME_PATH", &s);
            }

            if !names.pci_slot.is_empty() {
                let s = format!("{}{}", prefix, names.pci_slot);
                let _ = self.add_property(dev.clone(), test, "ID_NET_NAME_SLOT", &s);
            }
        }

        /* USB device */
        if names_usb(dev.clone(), &mut names).is_ok() && names.r#type == NetNameType::Usb {
            if !names.pci_path.is_empty() {
                let s = format!("{}{}{}", prefix, names.pci_path, names.usb_ports);
                let _ = self.add_property(dev.clone(), test, "ID_NET_NAME_PATH", &s);
            }

            if !names.pci_slot.is_empty() {
                let s = format!("{}{}{}", prefix, names.pci_slot, names.usb_ports);
                let _ = self.add_property(dev, test, "ID_NET_NAME_SLOT", &s);
            }

            return Ok(true);
        }

        /* Broadcom bus */
        if names_bcma(dev.clone(), &mut names).is_ok() && names.r#type == NetNameType::Bcma {
            if !names.pci_path.is_empty() {
                let s = format!("{}{}{}", prefix, names.pci_path, names.bcma_core);
                let _ = self.add_property(dev.clone(), test, "ID_NET_NAME_PATH", &s);
            }

            if !names.pci_slot.is_empty() {
                let s = format!("{}{}{}", prefix, names.pci_slot, names.bcma_core);
                let _ = self.add_property(dev, test, "ID_NET_NAME_SLOT", &s);
            }

            return Ok(true);
        }

        Ok(true)
    }

    /// builtin init function
    fn init(&self) {}

    /// builtin exit function
    fn exit(&self) {}

    /// check whether builtin command should reload
    fn should_reload(&self) -> bool {
        false
    }

    /// the help of builtin command
    fn help(&self) -> String {
        "Network device properties".to_string()
    }

    /// whether the builtin command can only run once
    fn run_once(&self) -> bool {
        false
    }
}
