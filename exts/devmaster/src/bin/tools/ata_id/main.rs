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

//! ata_id - reads product/serial number from ATA drives
//!

#![allow(deprecated)]
use basic::IN_SET;
use clap::Parser;
use libdevmaster::utils::commons::{encode_devnode_name, replace_chars, replace_whitespace};
use nix::{
    errno::Errno,
    fcntl::{open, OFlag},
    ioctl_write_ptr_bad,
    sys::stat::Mode,
};
use scsi_generic_rs::{
    hd_driveid, sg_io_hdr, sg_io_v4, BSG_PROTOCOL_SCSI, BSG_SUB_PROTOCOL_SCSI_CMD,
    HDIO_GET_IDENTITY, SG_DXFER_FROM_DEV, SG_IO,
};
use std::{
    mem,
    os::raw::{c_ulong, c_void},
    path::Path,
    ptr, str,
};

const IDENTIFY_SIZE: usize = 512;
const COMMAND_TIMEOUT_MSEC: u32 = 30000;

ioctl_write_ptr_bad!(
    /// ioctl sg_io_v4 with SG_IO
    ioctl_sg_io_v4,
    SG_IO,
    sg_io_v4
);
ioctl_write_ptr_bad!(
    /// ioctl sg_io_hdr with SG_IO
    ioctl_sg_io_hdr,
    SG_IO,
    sg_io_hdr
);
ioctl_write_ptr_bad!(
    /// ioctl hd_driveid with HDIO_GET_IDENTITY
    ioctl_hdio_get_identity,
    HDIO_GET_IDENTITY,
    hd_driveid
);

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Print values as environment keys
    #[clap(short('x'), long("export"))]
    export_flag: bool,
    device: String,
}

enum DiskCmdType {
    ScsiInquiry,
    Identify,
    IdentifyPacketDevice,
}

/// Obtains specified device info based on DiskCmdType.
/// fd:           File descriptor for the block device.
/// buf:          Device info buffer.
/// buf_len:      Size of the device info buffer.
/// command_type: Typr of the command for obtaining device info.
fn disk_command(
    fd: i32,
    buf: *mut c_void,
    buf_len: u32,
    command_type: DiskCmdType,
) -> Result<i32, Errno> {
    let mut cdb_inquiry = [
        /* INQUIRY, see SPC-4 section 6.4 */
        0x12u8, /* OPERATION CODE: INQUIRY */
        0,
        0,
        (buf_len >> 8) as u8, /* ALLOCATION LENGTH */
        (buf_len & 0xff) as u8,
        0,
    ];
    let mut cdb_identify = [
        /* ATA Pass-Through 12 byte command */
        0xa1u8, /* OPERATION CODE: 12 byte pass through */
        4 << 1, /* PROTOCOL: PIO Data-in */
        0x2e,   /* OFF_LINE=0, CK_COND=1, T_DIR=1, BYT_BLOK=1, T_LENGTH=2 */
        0,      /* FEATURES */
        1,      /* SECTORS */
        0,      /* LBA LOW */
        0,      /* LBA MID */
        0,      /* LBA HIGH */
        0,      /* SELECT */
        0xEC,   /* Command: ATA IDENTIFY DEVICE */
        0,
        0,
    ];
    let mut cdb_identify_packet_device = [
        /* ATA Pass-Through 16 byte command */
        0x85u8, /* OPERATION CODE: 16 byte pass through */
        4 << 1, /* PROTOCOL: PIO Data-in */
        0x2e,   /* OFF_LINE=0, CK_COND=1, T_DIR=1, BYT_BLOK=1, T_LENGTH=2 */
        0,      /* FEATURES */
        0,      /* FEATURES */
        0,      /* SECTORS */
        1,      /* SECTORS */
        0,      /* LBA LOW */
        0,      /* LBA LOW */
        0,      /* LBA MID */
        0,      /* LBA MID */
        0,      /* LBA HIGH */
        0,      /* LBA HIGH */
        0,      /* DEVICE */
        0xA1,   /* Command: ATA IDENTIFY PACKET DEVICE */
        0,      /* CONTROL */
    ];

    let (cdb_ptr, cdb_len) = match command_type {
        DiskCmdType::ScsiInquiry => (cdb_inquiry.as_mut_ptr(), mem::size_of_val(&cdb_inquiry)),
        DiskCmdType::Identify => (cdb_identify.as_mut_ptr(), mem::size_of_val(&cdb_identify)),
        DiskCmdType::IdentifyPacketDevice => (
            cdb_identify_packet_device.as_mut_ptr(),
            mem::size_of_val(&cdb_identify_packet_device),
        ),
    };

    let mut sense: [u8; 32] = [0; 32];
    let io_v4 = sg_io_v4 {
        guard: 'Q' as i32,
        protocol: BSG_PROTOCOL_SCSI,
        subprotocol: BSG_SUB_PROTOCOL_SCSI_CMD,
        request_len: cdb_len as u32,
        request: cdb_ptr as c_ulong,
        max_response_len: mem::size_of_val(&sense) as u32,
        response: sense.as_mut_ptr() as c_ulong,
        din_xfer_len: buf_len,
        din_xferp: buf as c_ulong,
        timeout: COMMAND_TIMEOUT_MSEC,
        ..Default::default()
    };

    if let Err(e) = unsafe { ioctl_sg_io_v4(fd, &io_v4) } {
        if e != Errno::EINVAL {
            log::error!("ioctl v4 failed: {}", e);
            return Err(e);
        }

        /* Could be that the driver doesn't do version 4, try version 3 */
        let io_hdr = sg_io_hdr {
            interface_id: 'S' as i32,
            cmdp: cdb_ptr,
            cmd_len: cdb_len as u8,
            dxferp: buf,
            dxfer_len: buf_len,
            sbp: sense.as_mut_ptr() as *mut u8,
            mx_sb_len: mem::size_of_val(&sense) as u8,
            dxfer_direction: SG_DXFER_FROM_DEV,
            timeout: COMMAND_TIMEOUT_MSEC,
            ..Default::default()
        };

        if let Err(e) = unsafe { ioctl_sg_io_hdr(fd, &io_hdr) } {
            log::error!("ioctl v3 failed: {}", e);
            return Err(e);
        }

        if let DiskCmdType::ScsiInquiry = command_type {
            if (io_hdr.status != 0) || (io_hdr.host_status != 0) || (io_hdr.driver_status != 0) {
                log::error!("ioctl v3 failed: {}", Errno::EIO);
                return Err(Errno::EIO);
            }
        }
    } else {
        let mut err_flag = false;

        match command_type {
            DiskCmdType::ScsiInquiry => {
                if (io_v4.device_status != 0)
                    || (io_v4.transport_status != 0)
                    || (io_v4.driver_status != 0)
                {
                    err_flag = true;
                }
            }
            DiskCmdType::Identify => {
                if !(((sense[0] & 0x7f) == 0x72 && sense[8] == 0x09 && sense[9] == 0x0c)
                    || ((sense[0] & 0x7f) == 0x70 && sense[12] == 0x00 && sense[13] == 0x1d))
                {
                    err_flag = true;
                }
            }
            DiskCmdType::IdentifyPacketDevice => {
                if (sense[0] & 0x7f) != 0x72 || sense[8] != 0x09 || sense[9] != 0x0c {
                    err_flag = true;
                }
            }
        }

        if err_flag {
            log::error!("ioctl v4 failed: {}", Errno::EIO);
            return Err(Errno::EIO);
        }
    }

    Ok(0)
}

/// Copies the ATA string from the offset_words offset of identify to the beginning of the dest_offset.
/// identify:     A block of IDENTIFY data.
/// offset_words: Offset of the string to get, in words.
/// dest_offset:  Offset of the identify dest address.
/// dest_len:     Length of the copy size, in bytes.
unsafe fn disk_identify_get_string(
    identify: &mut [u8; IDENTIFY_SIZE],
    offset_words: usize,
    dest_offset: usize,
    dest_len: usize,
) {
    let mut i = dest_offset;
    let mut offset: usize = offset_words;
    let mut c1;
    let mut c2;

    while i < (dest_offset + dest_len) {
        c1 = identify[offset * 2 + 1];
        c2 = identify[offset * 2];

        identify[i] = c1;
        i += 1;
        identify[i] = c2;
        i += 1;
        offset += 1;
    }
}

fn disk_identify_fixup_string(identify: &mut [u8; IDENTIFY_SIZE], offset_words: usize, len: usize) {
    assert!(offset_words < (IDENTIFY_SIZE / 2));

    unsafe { disk_identify_get_string(identify, offset_words, offset_words * 2, len) };
}

fn disk_identify_fixup_u16(identify: &mut [u8; IDENTIFY_SIZE], offset_words: usize) {
    assert!(offset_words < (IDENTIFY_SIZE / 2));

    let offset = offset_words * 2;
    unsafe {
        ptr::write_unaligned(
            identify.as_mut_ptr().add(offset) as *mut u16,
            u16::from_le_bytes([identify[offset], identify[offset + 1]]),
        )
    };
}

/// Obtains the identify info.
/// fd:           File descriptor for the block device.
/// out_identify: Returen location for IDENTIFY data.
fn disk_identify(fd: i32, out_identify: &mut [u8; IDENTIFY_SIZE]) -> Result<i32, Errno> {
    let mut inquiry_buf = [0u8; 36];

    /* init results */
    out_identify.iter_mut().for_each(|x| *x = 0);

    /* If we were to use ATA PASS_THROUGH (12) on an ATAPI device
     * we could accidentally blank media. This is because MMC's BLANK
     * command has the same op-code (0x61).
     *
     * To prevent this from happening we bail out if the device
     * isn't a Direct Access Block Device, e.g. SCSI type 0x00
     * (CD/DVD devices are type 0x05). So we send a SCSI INQUIRY
     * command first... libata is handling this via its SCSI
     * emulation layer.
     *
     * This also ensures that we're actually dealing with a device
     * that understands SCSI commands.
     */
    if let Err(e) = disk_command(
        fd,
        (&mut inquiry_buf).as_mut_ptr() as *mut c_void,
        mem::size_of_val(&inquiry_buf) as u32,
        DiskCmdType::ScsiInquiry,
    ) {
        return Err(e);
    }

    /* SPC-4, section 6.4.2: Standard INQUIRY data */
    let peripheral_device_type = inquiry_buf[0] & 0x1f;
    if peripheral_device_type == 0x05 {
        if let Err(e) = disk_command(
            fd,
            out_identify.as_mut_ptr() as *mut c_void,
            IDENTIFY_SIZE as u32,
            DiskCmdType::IdentifyPacketDevice,
        ) {
            return Err(e);
        }
    } else {
        if !(IN_SET!(peripheral_device_type, 0x00, 0x14)) {
            log::error!("Unsupported device type.");
            return Err(Errno::EIO);
        }

        if let Err(e) = disk_command(
            fd,
            out_identify.as_mut_ptr() as *mut c_void,
            IDENTIFY_SIZE as u32,
            DiskCmdType::Identify,
        ) {
            return Err(e);
        }
    }

    /* Check if IDENTIFY data is all NUL bytes - if so, bail */
    let mut all_nul_bytes = true;
    for item in out_identify.iter_mut() {
        if *item != 0 {
            all_nul_bytes = false;
            break;
        }
    }

    if all_nul_bytes {
        log::error!("IDENTIFY data is all zeroes.");
        return Err(Errno::EIO);
    }

    Ok(0)
}

fn main() {
    #[repr(C)]
    union IdentifyUnion {
        byte: [u8; IDENTIFY_SIZE],
        wyde: [u16; IDENTIFY_SIZE / 2],
    }

    let args = Args::parse();

    log::init_log_to_console_syslog("ata_id", log::Level::Info);

    let mut id: hd_driveid = unsafe { mem::zeroed() };
    let mut identify = IdentifyUnion {
        byte: [0; IDENTIFY_SIZE],
    };
    let identify_byte = unsafe { &mut identify.byte };

    let filename = &args.device;
    let node = Path::new(filename);
    let fd = match open(
        node,
        OFlag::O_RDONLY | OFlag::O_NONBLOCK | OFlag::O_CLOEXEC | OFlag::O_NOCTTY,
        Mode::from_bits(0o666).unwrap(),
    ) {
        Ok(fd) => fd,
        Err(e) => {
            log::error!("Cannot open {}: {}", filename, e);
            return;
        }
    };
    if disk_identify(fd, identify_byte).is_err() {
        if let Err(e) = unsafe { ioctl_hdio_get_identity(fd, &id) } {
            log::error!("{}: HDIO_GET_IDENTITY failed: {}", filename, e);
            let _ = nix::unistd::close(fd);
            return;
        }
    } else {
        /*
         * fix up only the fields from the IDENTIFY data that we are going to
         * use and copy it into the hd_driveid struct for convenience
         */
        disk_identify_fixup_string(identify_byte, 10, 20); /* serial */
        disk_identify_fixup_string(identify_byte, 23, 8); /* fwrev */
        disk_identify_fixup_string(identify_byte, 27, 40); /* model */
        disk_identify_fixup_u16(identify_byte, 0); /* configuration */
        disk_identify_fixup_u16(identify_byte, 75); /* queue depth */
        disk_identify_fixup_u16(identify_byte, 76); /* SATA capabilities */
        disk_identify_fixup_u16(identify_byte, 82); /* command set supported */
        disk_identify_fixup_u16(identify_byte, 83); /* command set supported */
        disk_identify_fixup_u16(identify_byte, 84); /* command set supported */
        disk_identify_fixup_u16(identify_byte, 85); /* command set supported */
        disk_identify_fixup_u16(identify_byte, 86); /* command set supported */
        disk_identify_fixup_u16(identify_byte, 87); /* command set supported */
        disk_identify_fixup_u16(identify_byte, 89); /* time required for SECURITY ERASE UNIT */
        disk_identify_fixup_u16(identify_byte, 90); /* time required for enhanced SECURITY ERASE UNIT */
        disk_identify_fixup_u16(identify_byte, 91); /* current APM values */
        disk_identify_fixup_u16(identify_byte, 94); /* current AAM value */
        disk_identify_fixup_u16(identify_byte, 108); /* WWN */
        disk_identify_fixup_u16(identify_byte, 109); /* WWN */
        disk_identify_fixup_u16(identify_byte, 110); /* WWN */
        disk_identify_fixup_u16(identify_byte, 111); /* WWN */
        disk_identify_fixup_u16(identify_byte, 128); /* device lock function */
        disk_identify_fixup_u16(identify_byte, 217); /* nominal media rotation rate */
        unsafe {
            debug_assert!(mem::size_of_val(&id) == IDENTIFY_SIZE);
            ptr::copy_nonoverlapping(
                identify_byte.as_ptr(),
                &mut id as *mut _ as *mut u8,
                IDENTIFY_SIZE,
            );
        }
    }

    let _ = nix::unistd::close(fd);

    let mut model = str::from_utf8(&id.model).unwrap().to_string();
    let mut model_enc = String::new();
    encode_devnode_name(&model, &mut model_enc);
    model = replace_whitespace(str::from_utf8(&id.model).unwrap());
    model = replace_chars(&model, "");
    let mut serial = replace_whitespace(str::from_utf8(&id.serial_no).unwrap());
    serial = replace_chars(&serial, "");
    let mut revision = replace_whitespace(str::from_utf8(&id.fw_rev).unwrap());
    revision = replace_chars(&revision, "");

    if args.export_flag {
        println!("ID_ATA=1");
        if ((id.config >> 8) & 0x80) != 0 {
            /* This is an ATAPI device */
            match (id.config >> 8) & 0x1f {
                0 => println!("ID_TYPE=cd"),
                1 => println!("ID_TYPE=tape"),
                5 => println!("ID_TYPE=cd"),
                7 => println!("ID_TYPE=optical"),
                _ => println!("ID_TYPE=generic"),
            }
        } else {
            println!("ID_TYPE=disk");
        }
        println!("ID_BUS=ata");
        println!("ID_MODEL={}", model);
        println!("ID_MODEL_ENC={}", model_enc);
        println!("ID_REVISION={}", revision);
        if serial.is_empty() {
            println!("ID_SERIAL={}", model);
        } else {
            println!("ID_SERIAL={}_{}", model, serial);
            println!("ID_SERIAL_SHORT={}", serial);
        }

        if (id.command_set_1 & (1 << 5)) != 0 {
            println!("ID_ATA_WRITE_CACHE=1");
            println!(
                "ID_ATA_WRITE_CACHE_ENABLED={}",
                if (id.cfs_enable_1 & (1 << 5)) != 0 {
                    1
                } else {
                    0
                }
            );
        }
        if (id.command_set_1 & (1 << 10)) != 0 {
            println!("ID_ATA_FEATURE_SET_HPA=1");
            println!(
                "ID_ATA_FEATURE_SET_HPA_ENABLED={}",
                if (id.cfs_enable_1 & (1 << 10)) != 0 {
                    1
                } else {
                    0
                }
            );
        }
        if (id.command_set_1 & (1 << 3)) != 0 {
            println!("ID_ATA_FEATURE_SET_PM=1");
            println!(
                "ID_ATA_FEATURE_SET_PM_ENABLED={}",
                if (id.cfs_enable_1 & (1 << 3)) != 0 {
                    1
                } else {
                    0
                }
            );
        }
        if (id.command_set_1 & (1 << 1)) != 0 {
            println!("ID_ATA_FEATURE_SET_SECURITY=1");
            println!(
                "ID_ATA_FEATURE_SET_SECURITY_ENABLED={}",
                if (id.cfs_enable_1 & (1 << 1)) != 0 {
                    1
                } else {
                    0
                }
            );
            println!(
                "ID_ATA_FEATURE_SET_SECURITY_ERASE_UNIT_MIN={}",
                id.trseuc * 2
            );
            if (id.cfs_enable_1 & (1 << 1)) != 0
            /* enabled */
            {
                if (id.dlf & (1 << 8)) != 0 {
                    println!("ID_ATA_FEATURE_SET_SECURITY_LEVEL=maximum");
                } else {
                    println!("ID_ATA_FEATURE_SET_SECURITY_LEVEL=high");
                }
            }
            if (id.dlf & (1 << 5)) != 0 {
                println!(
                    "ID_ATA_FEATURE_SET_SECURITY_ENHANCED_ERASE_UNIT_MIN={}",
                    id.trseuc * 2
                );
            }
            if (id.dlf & (1 << 4)) != 0 {
                println!("ID_ATA_FEATURE_SET_SECURITY_EXPIRE=1");
            }
            if (id.dlf & (1 << 3)) != 0 {
                println!("ID_ATA_FEATURE_SET_SECURITY_FROZEN=1");
            }
            if (id.dlf & (1 << 2)) != 0 {
                println!("ID_ATA_FEATURE_SET_SECURITY_LOCKED=1");
            }
        }
        if (id.command_set_1 & (1 << 0)) != 0 {
            println!("ID_ATA_FEATURE_SET_SMART=1");
            println!(
                "ID_ATA_FEATURE_SET_SMART_ENABLED={}",
                if (id.cfs_enable_1 & (1 << 0)) != 0 {
                    1
                } else {
                    0
                }
            );
        }
        if (id.command_set_2 & (1 << 9)) != 0 {
            println!("ID_ATA_FEATURE_SET_AAM=1");
            println!(
                "ID_ATA_FEATURE_SET_AAM_ENABLED={}",
                if (id.cfs_enable_2 & (1 << 9)) != 0 {
                    1
                } else {
                    0
                }
            );
            println!(
                "ID_ATA_FEATURE_SET_AAM_VENDOR_RECOMMENDED_VALUE={}",
                id.acoustic >> 8
            );
            println!(
                "ID_ATA_FEATURE_SET_AAM_CURRENT_VALUE={}",
                id.acoustic & 0xff
            );
        }
        if (id.command_set_2 & (1 << 5)) != 0 {
            println!("ID_ATA_FEATURE_SET_PUIS=1");
            println!(
                "ID_ATA_FEATURE_SET_PUIS_ENABLED={}",
                if (id.cfs_enable_2 & (1 << 5)) != 0 {
                    1
                } else {
                    0
                }
            );
        }
        if (id.command_set_2 & (1 << 3)) != 0 {
            println!("ID_ATA_FEATURE_SET_APM=1");
            println!(
                "ID_ATA_FEATURE_SET_APM_ENABLED={}",
                if (id.cfs_enable_2 & (1 << 3)) != 0 {
                    1
                } else {
                    0
                }
            );
            if (id.cfs_enable_2 & (1 << 3)) != 0 {
                println!(
                    "ID_ATA_FEATURE_SET_APM_CURRENT_VALUE={}",
                    id.CurAPMvalues & 0xff
                );
            }
        }
        if (id.command_set_2 & (1 << 0)) != 0 {
            println!("ID_ATA_DOWNLOAD_MICROCODE=1");
        }

        let identify_wyde = unsafe { &identify.wyde };
        /*
         * Word 76 indicates the capabilities of a SATA device. A PATA device shall set
         * word 76 to 0000h or FFFFh. If word 76 is set to 0000h or FFFFh, then
         * the device does not claim compliance with the Serial ATA specification and words
         * 76 through 79 are not valid and shall be ignored.
         */
        let mut word = identify_wyde[76];
        if !IN_SET!(word, 0x0000, 0xffff) {
            println!("ID_ATA_SATA=1");
            /*
             * If bit 2 of word 76 is set to one, then the device supports the Gen2
             * signaling rate of 3.0 Gb/s (see SATA 2.6).
             *
             * If bit 1 of word 76 is set to one, then the device supports the Gen1
             * signaling rate of 1.5 Gb/s (see SATA 2.6).
             */
            if (word & (1 << 2)) != 0 {
                println!("ID_ATA_SATA_SIGNAL_RATE_GEN2=1");
            }
            if (word & (1 << 1)) != 0 {
                println!("ID_ATA_SATA_SIGNAL_RATE_GEN1=1");
            }
        }

        /* Word 217 indicates the nominal media rotation rate of the device */
        word = identify_wyde[217];
        if word == 0x0001 {
            println!("ID_ATA_ROTATION_RATE_RPM=0"); /* non-rotating e.g. SSD */
        } else if (0x0401..=0xfffe).contains(&word) {
            println!("ID_ATA_ROTATION_RATE_RPM={}", word);
        }

        /*
         * Words 108-111 contain a mandatory World Wide Name (WWN) in the NAA IEEE Registered identifier
         * format. Word 108 bits (15:12) shall contain 5h, indicating that the naming authority is IEEE.
         * All other values are reserved.
         */
        word = identify_wyde[108];
        if (word & 0xf000) == 0x5000 {
            let mut wwwn: u64;

            wwwn = identify_wyde[108] as u64;
            wwwn <<= 16;
            wwwn |= identify_wyde[109] as u64;
            wwwn <<= 16;
            wwwn |= identify_wyde[110] as u64;
            wwwn <<= 16;
            wwwn |= identify_wyde[111] as u64;
            println!("ID_WWN=0x{:x}", wwwn);
            println!("ID_WWN_WITH_EXTENSION=0x{:x}", wwwn);
        }

        /* from Linux's include/linux/ata.h */
        if (IN_SET!(identify_wyde[0], 0x848a, 0x844a) || (identify_wyde[83] & 0xc004) == 0x4004) {
            println!("ID_ATA_CFA=1");
        }
    } else if serial.is_empty() {
        println!("{}", model);
    } else {
        println!("{}_{}", model, serial);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_disk_identify_fixup_string() {
        let mut identify = [0u8; IDENTIFY_SIZE];
        for (i, item) in identify.iter_mut().enumerate().take(IDENTIFY_SIZE) {
            *item = (i % 256) as u8;
        }
        disk_identify_fixup_string(&mut identify, 10, 10);

        assert!(identify[20..30] == [21, 20, 23, 22, 25, 24, 27, 26, 29, 28]);
    }

    #[test]
    fn test_disk_identify_fixup_u16() {
        let mut identify = [0u8; IDENTIFY_SIZE];
        for (i, item) in identify.iter_mut().enumerate().take(IDENTIFY_SIZE) {
            *item = (i % 256) as u8;
        }
        disk_identify_fixup_u16(&mut identify, 0);

        let ptr = identify[0..2].as_ptr() as *const u16;
        let val = unsafe { *ptr.add(0) };
        assert!(val == 256);
    }
}
