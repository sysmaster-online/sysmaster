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

//! scsi_id - retrieve and generate a unique identifier
//!

use basic::IN_SET;
use libc::{c_char, c_void, getopt_long, option, EBADF, EINVAL, ENOSYS};
use libdevmaster::utils::commons::{encode_devnode_name, replace_chars, replace_whitespace};
use nix::{
    errno::{self, Errno},
    fcntl::{open, OFlag},
    sys::stat::{self, fstat, major, minor, Mode},
};
use scsi_generic_rs::{sg_io_hdr, sg_io_v4, SG_IO};
use std::{
    ffi::{CStr, CString},
    fs::File,
    io::{BufRead, BufReader},
    os::{
        raw::{c_int, c_uchar, c_uint, c_ulonglong},
        unix::prelude::FromRawFd,
    },
    process::exit,
    ptr::NonNull,
    str::from_utf8,
    thread,
    time::Duration,
};

extern "C" {
    ///option arg for getopt_long
    pub static mut optarg: *mut c_char;
    ///option index  for getopt_long
    pub static mut optind: c_int;
}

///device in whitelist or blacklist
pub static mut ALL_GOOD: bool = false;
///device specified
pub static mut DEV_SPECIFIED: bool = false;
///config file for scsi_id
pub static mut SCSI_ID_CONFIG: &str = "/etc/scsi_id.config";
///devnode name
pub static mut MAJ_MIN_DEV: String = String::new();
///default pagecode
pub static mut DEFAULT_PAGE_CODE: PageCode = PageCode::PageUnspecified;
///default SCSI Generic version
pub static mut DEFAULT_SG_VERSION: u8 = 4;
///replace whitespace
pub static mut REFORMAT_SERIAL: bool = false;
///print values
pub static mut EXPORT: bool = false;
///vendor name
pub static mut VENDOR: String = String::new();
///encoded vendor name
pub static mut VENDOR_ENC_STR: String = String::new();
///model name
pub static mut MODEL: String = String::new();
///encoded model name
pub static mut MODEL_ENC_STR: String = String::new();
///revision get from ioctl
pub static mut REVISION: String = String::new();
///device type get from ioctl
pub static mut TYPE_STR: String = String::new();
///argc get from scsi_id.conf
pub static mut ARGC: i32 = 0;
///argv get from scsi_id.conf
pub static mut ARGV: Vec<String> = Vec::new();
///default timeout for ioctl
pub const DEF_TIMEOUT: u32 = 5000;
///length of SENSE_BUFF
pub const SENSE_BUFF_LEN: u32 = 32;
///length of SCSI_INQ_BUFF
pub const SCSI_INQ_BUFF_LEN: u32 = 254;
///max length for vendor
pub const VENDOR_LENGTH: usize = 8;
///max length for model
pub const MODEL_LENGTH: usize = 16;
///inquiry cmd for ioctl
pub const INQUIRY_CMD: c_uchar = 0x12;
///inquiry cmd length for ioctl
pub const INQUIRY_CMDLEN: u32 = 6;
///id type values of id descriptors
pub const SCSI_ID_VENDOR_SPECIFIC: c_uchar = 0;
///id type values of id descriptors
pub const SCSI_ID_T10_VENDOR: c_uchar = 1;
///id type values of id descriptors
pub const SCSI_ID_EUI_64: c_uchar = 2;
///id type values of id descriptors
pub const SCSI_ID_NAA: c_uchar = 3;
///id type values of id descriptors
pub const SCSI_ID_RELPORT: c_uchar = 4;
///id type values of id descriptors
pub const SCSI_ID_TGTGROUP: c_uchar = 5;
///naa type values of id descriptors
pub const SCSI_ID_NAA_DONT_CARE: c_uchar = 0xff;
///naa type values of id descriptors
pub const SCSI_ID_NAA_IEEE_REG: c_uchar = 0x05;
///naa type values of id descriptors
pub const SCSI_ID_NAA_IEEE_REG_EXTENDED: c_uchar = 0x06;
///code set values of id descriptors
pub const SCSI_ID_BINARY: c_uchar = 1;
///code set values of id descriptors
pub const SCSI_ID_ASCII: c_uchar = 2;
///SCSI status codes
pub const SCSI_CHECK_CONDITION: c_uchar = 0x02;
///SCSI status codes
pub const SCSI_COMMAND_TERMINATED: c_uchar = 0x22;
///unable to connect before timeout
pub const DID_NO_CONNECT: u32 = 0x01;
///bus remain busy until timeout
pub const DID_BUS_BUSY: u32 = 0x02;
///timed out for some other reason
pub const DID_TIME_OUT: u32 = 0x03;
///transport disrupted and should retry
pub const DID_TRANSPORT_DISPUPTED: u32 = 0x0e;
///driver status
pub const DRIVER_TIMEOUT: u32 = 0x06;
///sense_buffer has been set
pub const DRIVER_SENSE: u32 = 0x08;
///no errors or other information
pub const SG_ERR_CAT_CLEAN: i32 = 0;
///interpreted from sense buffer
pub const SG_ERR_CAT_MEDIA_CHANGED: i32 = 1;
///interpreted from sense buffer
pub const SG_ERR_CAT_RESET: i32 = 2;
///timeout to get sense buffer
pub const SG_ERR_CAT_TIMEOUT: i32 = 3;
///successful command after recovered err
pub const SG_ERR_CAT_RECOVERED: i32 = 4;
///illegal / unsupported command
pub const SG_ERR_CAT_NOTSUPPORTED: i32 = 5;
///illegal / unsupported command
pub const SG_ERR_CAT_RETRY: i32 = 6;
///something else in the sense buffer
pub const SG_ERR_CAT_SENSE: i32 = 98;
///some other error/warning
pub const SG_ERR_CAT_OTHER: i32 = 99;
///subprotocol of io_v4
pub const BSG_PROTOCOL_SCSI: u32 = 0;
///subprotocol of io_v4
pub const BSG_SUB_PROTOCOL_SCSI_CMD: u32 = 0;
///dxfer direction of io_hdr
pub const SG_DXFER_FROM_DEV: i32 = -3;
///sense key
pub const RECOVERED_ERROR: u64 = 0x01;
///sense key
pub const ILLEGAL_REQUEST: u64 = 0x05;
///sense key
pub const UNIT_ATTENTION: u64 = 0x06;
///max length for serial
pub const MAX_SERIAL_LEN: i32 = 256;

#[allow(missing_docs)]
#[derive(Debug)]
pub enum PageCode {
    Page83PreSpc3 = -0x83,
    PageUnspecified = 0x00,
    Page80 = 0x80,
    Page83 = 0x83,
}

impl std::fmt::Display for PageCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PageCode::Page83PreSpc3 => write!(f, "-{:#04x}", 0x83),
            PageCode::PageUnspecified => write!(f, "{:#04x}", 0x00),
            PageCode::Page80 => write!(f, "{:#04x}", 0x80),
            PageCode::Page83 => write!(f, "{:#04x}", 0x83),
        }
    }
}

#[allow(missing_docs)]
#[derive(Default, Debug)]
pub struct ScsiIdDevice {
    vendor: String,
    model: String,
    revision: String,
    kernel: String,
    serial: String,
    serial_short: String,
    r#type: u8,
    use_sg: u8,
    unit_serial_number: String,
    wwn: String,
    wwn_vendor_extension: String,
    tgpt_group: String,
}

#[repr(C)]
#[allow(non_camel_case_types)]
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone)]
pub struct scsi_id_search_values {
    id_type: c_uchar,
    naa_type: c_uchar,
    code_set: c_uchar,
}
#[allow(missing_docs)]
pub static ID_SEARCH_LIST: [scsi_id_search_values; 13] = [
    scsi_id_search_values {
        id_type: SCSI_ID_TGTGROUP,
        naa_type: SCSI_ID_NAA_DONT_CARE,
        code_set: SCSI_ID_BINARY,
    },
    scsi_id_search_values {
        id_type: SCSI_ID_NAA,
        naa_type: SCSI_ID_NAA_IEEE_REG_EXTENDED,
        code_set: SCSI_ID_BINARY,
    },
    scsi_id_search_values {
        id_type: SCSI_ID_NAA,
        naa_type: SCSI_ID_NAA_IEEE_REG_EXTENDED,
        code_set: SCSI_ID_ASCII,
    },
    scsi_id_search_values {
        id_type: SCSI_ID_NAA,
        naa_type: SCSI_ID_NAA_IEEE_REG,
        code_set: SCSI_ID_BINARY,
    },
    scsi_id_search_values {
        id_type: SCSI_ID_NAA,
        naa_type: SCSI_ID_NAA_IEEE_REG,
        code_set: SCSI_ID_ASCII,
    },
    scsi_id_search_values {
        id_type: SCSI_ID_NAA,
        naa_type: SCSI_ID_NAA_DONT_CARE,
        code_set: SCSI_ID_BINARY,
    },
    scsi_id_search_values {
        id_type: SCSI_ID_NAA,
        naa_type: SCSI_ID_NAA_DONT_CARE,
        code_set: SCSI_ID_ASCII,
    },
    scsi_id_search_values {
        id_type: SCSI_ID_EUI_64,
        naa_type: SCSI_ID_NAA_DONT_CARE,
        code_set: SCSI_ID_BINARY,
    },
    scsi_id_search_values {
        id_type: SCSI_ID_EUI_64,
        naa_type: SCSI_ID_NAA_DONT_CARE,
        code_set: SCSI_ID_ASCII,
    },
    scsi_id_search_values {
        id_type: SCSI_ID_T10_VENDOR,
        naa_type: SCSI_ID_NAA_DONT_CARE,
        code_set: SCSI_ID_BINARY,
    },
    scsi_id_search_values {
        id_type: SCSI_ID_T10_VENDOR,
        naa_type: SCSI_ID_NAA_DONT_CARE,
        code_set: SCSI_ID_ASCII,
    },
    scsi_id_search_values {
        id_type: SCSI_ID_VENDOR_SPECIFIC,
        naa_type: SCSI_ID_NAA_DONT_CARE,
        code_set: SCSI_ID_BINARY,
    },
    scsi_id_search_values {
        id_type: SCSI_ID_VENDOR_SPECIFIC,
        naa_type: SCSI_ID_NAA_DONT_CARE,
        code_set: SCSI_ID_ASCII,
    },
];

fn set_options(argc: i32, argv: Vec<String>) -> i32 {
    let mut args_c: Vec<*mut c_char> = Vec::new();
    for option in &argv {
        args_c.push(CString::new(option.clone()).unwrap().into_raw());
    }
    let device = CString::new("device").unwrap();
    let config = CString::new("config").unwrap();
    let page = CString::new("page").unwrap();
    let denylisted = CString::new("denylisted").unwrap();
    let allowlisted = CString::new("allowlisted").unwrap();
    let blacklisted = CString::new("blacklisted").unwrap();
    let whitelisted = CString::new("whitelisted").unwrap();
    let replace_whitespace = CString::new("replace-whitespace").unwrap();
    let sg_version = CString::new("sg-version").unwrap();
    let verbose = CString::new("verbose").unwrap();
    let version = CString::new("version").unwrap();
    let export = CString::new("export").unwrap();
    let help = CString::new("help").unwrap();

    let longopts = [
        option {
            name: device.as_ptr(),
            has_arg: 1,
            flag: std::ptr::null_mut(),
            val: b'd' as c_int,
        },
        option {
            name: config.as_ptr(),
            has_arg: 1,
            flag: std::ptr::null_mut(),
            val: b'f' as c_int,
        },
        option {
            name: page.as_ptr(),
            has_arg: 1,
            flag: std::ptr::null_mut(),
            val: b'p' as c_int,
        },
        option {
            name: denylisted.as_ptr(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: b'b' as c_int,
        },
        option {
            name: allowlisted.as_ptr(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: b'g' as c_int,
        },
        option {
            name: blacklisted.as_ptr(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: b'b' as c_int,
        },
        option {
            name: whitelisted.as_ptr(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: b'g' as c_int,
        },
        option {
            name: replace_whitespace.as_ptr(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: b'u' as c_int,
        },
        option {
            name: sg_version.as_ptr(),
            has_arg: 1,
            flag: std::ptr::null_mut(),
            val: b's' as c_int,
        },
        option {
            name: verbose.as_ptr(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: b'v' as c_int,
        },
        option {
            name: version.as_ptr(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: b'V' as c_int,
        },
        option {
            name: export.as_ptr(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: b'x' as c_int,
        },
        option {
            name: help.as_ptr(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: b'h' as c_int,
        },
        option {
            name: std::ptr::null_mut(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: 0,
        },
    ];
    unsafe {
        optind = 1;
        let optstring = CString::new("d:f:gp:uvVxhbs:").unwrap();

        loop {
            let c = getopt_long(
                argc as c_int,
                args_c.as_ptr() as *const *mut c_char,
                optstring.as_ptr(),
                longopts.as_ptr(),
                &mut 0,
            );

            if c == -1 {
                break;
            }

            match c as u8 as char {
                'b' => ALL_GOOD = false,
                'd' => {
                    DEV_SPECIFIED = true;
                    let optarg_tmp = CStr::from_ptr(optarg as *const c_char);
                    MAJ_MIN_DEV = from_utf8(optarg_tmp.to_bytes()).unwrap().to_string();
                }
                'f' => {
                    let optarg_tmp = CStr::from_ptr(optarg as *const c_char);
                    SCSI_ID_CONFIG = optarg_tmp.to_str().unwrap();
                }
                'g' => ALL_GOOD = true,
                'h' => {
                    println!("Usage: scsi_id [OPTION...] DEVICE");
                    println!("SCSI device identification.");
                    println!();
                    println!("  -h --help                        Print this message");
                    println!("  -V --version                     Print version of the program");
                    println!("  -d --device=                     Device node for SG_IO commands");
                    println!("  -f --config=                     Location of config file");
                    println!(
                        "  -p --page=0x80|0x83|pre-spc3-83  SCSI page (0x80, 0x83, pre-spc3-83)"
                    );
                    println!("  -s --sg-version=3|4              Use SGv3 or SGv4");
                    println!("  -b --blacklisted                 Treat device as blacklist");
                    println!("  -g --whitelisted                 Treat device as whitelisted");
                    println!(
                        "  -u --replace-whitespace          Replace all whitespace by underscores"
                    );
                    println!("  -x --export                      Print values as environment keys");
                    exit(0);
                }
                'p' => {
                    let optarg_tmp = CStr::from_ptr(optarg as *const c_char);
                    let pagecode = optarg_tmp.to_str().unwrap();
                    match pagecode {
                        "0x80" => DEFAULT_PAGE_CODE = PageCode::Page80,
                        "0x83" => DEFAULT_PAGE_CODE = PageCode::Page83,
                        "pre-spc3-83" => DEFAULT_PAGE_CODE = PageCode::Page83PreSpc3,
                        _ => {
                            log::error!("unknown page code.");
                            return -1;
                        }
                    }
                }
                's' => {
                    let optarg_tmp = CStr::from_ptr(optarg as *const c_char);
                    let sg_version_str = optarg_tmp.to_str().unwrap();
                    DEFAULT_SG_VERSION = sg_version_str.parse().unwrap();
                    if !(3..=4).contains(&DEFAULT_SG_VERSION) {
                        return -1;
                    }
                }
                'u' => REFORMAT_SERIAL = true,
                'V' => {
                    println!("scsi_id {}", env!("CARGO_PKG_VERSION"));
                    return 0;
                }
                'x' => EXPORT = true,
                _ => {
                    println!("invalid arguments");
                    return -1;
                }
            }
        }

        if optind < argc && !DEV_SPECIFIED {
            DEV_SPECIFIED = true;
            let tmp = CStr::from_ptr(args_c[optind as usize] as *const c_char);
            MAJ_MIN_DEV = from_utf8(tmp.to_bytes()).unwrap().to_string();
        }
    }

    0
}

fn sg_err_category_new(
    mut scsi_status: u32,
    host_status: u32,
    driver_status: u32,
    sense: u64,
    sb_len: u32,
) -> i32 {
    scsi_status &= 0x7e;
    if scsi_status == 0 && host_status == 0 && driver_status == 0 {
        return SG_ERR_CAT_CLEAN;
    }

    if IN_SET!(
        scsi_status,
        SCSI_CHECK_CONDITION as u32,
        SCSI_COMMAND_TERMINATED as u32
    ) || (driver_status & 0xf) == DRIVER_SENSE
    {
        let sense_buffer;
        let non_null_ptr = NonNull::new(sense as *mut u64);
        match non_null_ptr {
            Some(sense) => {
                sense_buffer = unsafe {
                    std::slice::from_raw_parts(sense.as_ptr() as *mut u64, SENSE_BUFF_LEN as usize)
                };
            }
            None => {
                return SG_ERR_CAT_SENSE;
            }
        }
        if sb_len > 2 {
            let sense_key;
            let asc;

            if (sense_buffer[0] & 0x2) != 0 {
                sense_key = sense_buffer[1] & 0xf;
                asc = sense_buffer[2];
            } else {
                sense_key = sense_buffer[2] & 0xf;
                if sb_len > 12 {
                    asc = sense_buffer[12];
                } else {
                    asc = 0;
                }
            }

            if sense_key == RECOVERED_ERROR {
                return SG_ERR_CAT_RECOVERED;
            } else if sense_key == UNIT_ATTENTION {
                if asc == 0x28 {
                    return SG_ERR_CAT_MEDIA_CHANGED;
                }

                if asc == 0x29 {
                    return SG_ERR_CAT_RESET;
                }
            } else if sense_key == ILLEGAL_REQUEST {
                return SG_ERR_CAT_NOTSUPPORTED;
            }
            return SG_ERR_CAT_SENSE;
        }
        if host_status != 0 {
            if IN_SET!(host_status, DID_NO_CONNECT, DID_BUS_BUSY, DID_TIME_OUT) {
                return SG_ERR_CAT_TIMEOUT;
            }

            if host_status == DID_TRANSPORT_DISPUPTED {
                return SG_ERR_CAT_RETRY;
            }
        }

        if driver_status != 0 && driver_status == DRIVER_TIMEOUT {
            return SG_ERR_CAT_TIMEOUT;
        }
    }
    SG_ERR_CAT_OTHER
}

fn sg_err_catagory4(io_buf_v4: &mut sg_io_v4) -> i32 {
    sg_err_category_new(
        io_buf_v4.device_status,
        io_buf_v4.transport_status,
        io_buf_v4.device_status,
        io_buf_v4.response,
        io_buf_v4.request_len,
    )
}

fn sg_err_catagory3(io_buf_hdr: &mut sg_io_hdr) -> i32 {
    sg_err_category_new(
        io_buf_hdr.status.into(),
        io_buf_hdr.host_status.into(),
        io_buf_hdr.driver_status.into(),
        io_buf_hdr.sbp as u64,
        io_buf_hdr.sb_len_wr.into(),
    )
}

fn scsi_dump_sense(dev_scsi: &mut ScsiIdDevice, sense: u64, sb_len: u32) -> i32 {
    if sb_len < 1 {
        log::debug!("{}: sense buffer empty.", dev_scsi.kernel);
        return -EINVAL;
    }
    let sense_buffer;
    let non_null_ptr = NonNull::new(sense as *mut u64);
    match non_null_ptr {
        Some(sense) => {
            sense_buffer =
                unsafe { std::slice::from_raw_parts(sense.as_ptr() as *mut u64, sb_len as usize) };
        }
        None => {
            return -EINVAL;
        }
    }
    let s;
    let sense_key;
    let asc;
    let ascq;

    let sense_class = sense_buffer[0] >> 4 & 0x07;
    let code = sense_buffer[0] & 0xf;

    if sense_class == 7 {
        s = sense_buffer[7] as u32 + 8;
        if sb_len < s {
            log::debug!(
                "{}: sense buffer too small {} bytes, {} bytes too short.",
                dev_scsi.kernel,
                sb_len,
                s - sb_len
            );
            return -EINVAL;
        }

        if IN_SET!(code, 0x0, 0x1) {
            sense_key = sense_buffer[2] & 0xf;
            if s < 14 {
                log::debug!("{}: sense result too small {} bytes", dev_scsi.kernel, s);
                return -EINVAL;
            }
            asc = sense_buffer[12];
            ascq = sense_buffer[13];
        } else if IN_SET!(code, 0x2, 0x3) {
            sense_key = sense_buffer[1] & 0xf;
            asc = sense_buffer[2];
            ascq = sense_buffer[3];
        } else {
            log::debug!("{}: invalid sense code {:x}.", dev_scsi.kernel, code);
            return -EINVAL;
        }
        log::debug!(
            "{}: sense key {:x} ASC {:x} ASCQ {:x}.",
            dev_scsi.kernel,
            sense_key,
            asc,
            ascq
        );
    } else {
        if sb_len < 4 {
            log::debug!(
                "{}: sense buffer too small {} bytes, {} bytes too short.",
                dev_scsi.kernel,
                sb_len,
                4 - sb_len
            );
            return -EINVAL;
        }

        if sense_buffer[0] < 15 {
            log::debug!(
                "{}: old sense key: {:x}",
                dev_scsi.kernel,
                sense_buffer[0] & 0x0f
            );
        } else {
            log::debug!(
                "{}: sense = {} {}.",
                dev_scsi.kernel,
                sense_buffer[0],
                sense_buffer[2]
            );
        }
        log::debug!(
            "{}: non-extended sense class {} code {:x}.",
            dev_scsi.kernel,
            sense_class,
            code
        );
    }

    -1
}

fn scsi_dump_v4(dev_scsi: &mut ScsiIdDevice, io_buf_v4: sg_io_v4) -> i32 {
    if io_buf_v4.device_status != 0
        && io_buf_v4.transport_status != 0
        && io_buf_v4.driver_status != 0
    {
        log::debug!("scsi_dump_v4: {}: sense buffer empty.", dev_scsi.kernel);
        return -EINVAL;
    }

    log::debug!(
        "scsi_dump_v4: {}: sg_io failed status {:x} {:x} {:x}",
        dev_scsi.kernel,
        io_buf_v4.driver_status,
        io_buf_v4.transport_status,
        io_buf_v4.device_status
    );
    if io_buf_v4.device_status == (SCSI_CHECK_CONDITION as u32) {
        return scsi_dump_sense(dev_scsi, io_buf_v4.response, io_buf_v4.request_len);
    }

    -1
}

fn scsi_dump(dev_scsi: &mut ScsiIdDevice, io_buf_hdr: sg_io_hdr) -> i32 {
    if io_buf_hdr.status != 0
        && io_buf_hdr.host_status != 0
        && io_buf_hdr.msg_status != 0
        && io_buf_hdr.driver_status != 0
    {
        log::debug!("scsi_dump: {}: sense buffer empty.", dev_scsi.kernel);
        return -EINVAL;
    }

    log::debug!(
        "scsi_dump: {}: sg_io failed status {:x} {:x} {:x} {:x}",
        dev_scsi.kernel,
        io_buf_hdr.driver_status,
        io_buf_hdr.host_status,
        io_buf_hdr.msg_status,
        io_buf_hdr.status
    );
    if io_buf_hdr.status == (SCSI_CHECK_CONDITION as u8) {
        return scsi_dump_sense(dev_scsi, io_buf_hdr.sbp as u64, io_buf_hdr.sb_len_wr as u32);
    }

    -1
}

fn scsi_inquiry(
    dev_scsi: &mut ScsiIdDevice,
    fd: i32,
    evpd: c_uchar,
    page: c_uchar,
    buffer: &mut [c_uchar],
    buflen: u32,
) -> i32 {
    let inq_cmd: [c_uchar; INQUIRY_CMDLEN as usize] =
        [INQUIRY_CMD, evpd, page, 0, buflen as c_uchar, 0];
    let sense = [0_u8; SENSE_BUFF_LEN as usize];
    let mut io_buf_v4 = sg_io_v4::default();
    let mut io_buf_hdr = sg_io_hdr::default();
    let mut r = 0;
    let mut retry = 3;

    while retry > 0 {
        if dev_scsi.use_sg == 4 {
            io_buf_v4.guard = 'Q' as c_int;
            io_buf_v4.protocol = BSG_PROTOCOL_SCSI;
            io_buf_v4.subprotocol = BSG_SUB_PROTOCOL_SCSI_CMD;
            io_buf_v4.request_len = INQUIRY_CMDLEN;
            io_buf_v4.request = inq_cmd.as_ptr() as c_ulonglong;
            io_buf_v4.max_response_len = SENSE_BUFF_LEN;
            io_buf_v4.response = sense.as_ptr() as c_ulonglong;
            io_buf_v4.din_xfer_len = buflen;
            io_buf_v4.din_xferp = buffer.as_mut_ptr() as c_ulonglong;

            r = unsafe { libc::ioctl(fd, (SG_IO as i32).try_into().unwrap(), &io_buf_v4) };
            if r < 0 {
                if IN_SET!(errno::errno(), EINVAL, ENOSYS) {
                    dev_scsi.use_sg = 3;
                } else {
                    log::debug!("{}: ioctl failed for io_buf_v4!", dev_scsi.kernel);
                    return r;
                }
            }
        }

        if dev_scsi.use_sg != 4 {
            io_buf_hdr.interface_id = 'S' as c_int;
            io_buf_hdr.cmd_len = INQUIRY_CMDLEN as c_uchar;
            io_buf_hdr.mx_sb_len = SENSE_BUFF_LEN as c_uchar;
            io_buf_hdr.dxfer_direction = SG_DXFER_FROM_DEV;
            io_buf_hdr.dxfer_len = buflen;
            io_buf_hdr.dxferp = buffer.as_mut_ptr() as *mut c_void;
            io_buf_hdr.cmdp = inq_cmd.as_ptr() as *mut c_uchar;
            io_buf_hdr.sbp = sense.as_ptr() as *mut c_uchar;
            io_buf_hdr.timeout = DEF_TIMEOUT;

            r = unsafe { libc::ioctl(fd, (SG_IO as i32).try_into().unwrap(), &io_buf_hdr) };
            if r < 0 {
                log::debug!("{}: ioctl failed for io_buf_hdr!", dev_scsi.kernel);
                return r;
            }
        }

        if dev_scsi.use_sg == 4 {
            r = sg_err_catagory4(&mut io_buf_v4);
        } else {
            r = sg_err_catagory3(&mut io_buf_hdr);
        }

        match r {
            SG_ERR_CAT_NOTSUPPORTED => {
                buffer[1] = 0_u8;
                r = 0;
            }
            SG_ERR_CAT_CLEAN | SG_ERR_CAT_RECOVERED => {
                r = 0;
            }
            SG_ERR_CAT_RETRY => {}
            _ => {
                if dev_scsi.use_sg == 4 {
                    r = scsi_dump_v4(dev_scsi, io_buf_v4);
                } else {
                    r = scsi_dump(dev_scsi, io_buf_hdr);
                }
            }
        }

        if r == 0 {
            r = buflen as i32;
            break;
        }

        if r > 0 && retry == 1 {
            r = -1
        }

        retry -= 1;
    }
    r
}

fn scsi_std_inquiry(dev_scsi: &mut ScsiIdDevice, devname: &str) -> i32 {
    let mut buffer = [0_u8; SCSI_INQ_BUFF_LEN as usize];
    let fd = match open(
        devname,
        OFlag::O_RDONLY | OFlag::O_NONBLOCK | OFlag::O_CLOEXEC | OFlag::O_NOCTTY,
        Mode::from_bits(0o666).unwrap(),
    ) {
        Ok(fd) => fd,
        Err(e) => {
            log::debug!("scsi_std_inquiry: Cannot open {}: {}", devname, e);
            return e as i32;
        }
    };

    let statbuf = match fstat(fd) {
        Err(_) => {
            log::debug!("scsi_std_inquiry: can't fstat {}", devname);
            return 2;
        }
        Ok(stat) => stat,
    };

    dev_scsi.kernel = format!("{}:{}", major(statbuf.st_rdev), minor(statbuf.st_rdev));

    let r = scsi_inquiry(dev_scsi, fd, 0, 0, &mut buffer, SCSI_INQ_BUFF_LEN);
    if r < 0 {
        return r;
    }

    let tmp =
        unsafe { std::slice::from_raw_parts(buffer.as_mut_ptr() as *mut c_uchar, buffer.len()) };
    dev_scsi.vendor = String::from_utf8(tmp[8..16].to_vec()).unwrap();
    dev_scsi.model = String::from_utf8(tmp[16..32].to_vec()).unwrap();
    dev_scsi.revision = String::from_utf8(tmp[32..36].to_vec()).unwrap();
    dev_scsi.r#type = tmp[0] as u8 & 0x1f;

    0
}

fn set_inq_values(dev_scsi: &mut ScsiIdDevice, maj_min_dev: &str) -> i32 {
    dev_scsi.use_sg = unsafe { DEFAULT_SG_VERSION };

    let r = scsi_std_inquiry(dev_scsi, maj_min_dev);
    if r != 0 {
        return r;
    }

    encode_devnode_name(&dev_scsi.vendor, unsafe { &mut VENDOR_ENC_STR });
    encode_devnode_name(&dev_scsi.model, unsafe { &mut MODEL_ENC_STR });

    unsafe {
        VENDOR = replace_whitespace(&dev_scsi.vendor);
        VENDOR = replace_chars(&VENDOR, "");
        MODEL = replace_whitespace(&dev_scsi.model);
        MODEL = replace_chars(&MODEL, "");
        REVISION = replace_whitespace(&dev_scsi.revision);
        REVISION = replace_chars(&REVISION, "");
    };

    unsafe {
        TYPE_STR = match dev_scsi.r#type {
            0 | 0xe => "disk".to_string(),
            1 => "tape".to_string(),
            4 | 7 | 0xf => "optical".to_string(),
            5 => "cd".to_string(),
            _ => "generic".to_string(),
        }
    }

    0
}

fn get_file_options(vendor: String, model: String) -> i32 {
    let r = 0;
    let mut vendor_in = String::new();
    let mut model_in = String::new();
    let mut options_in = String::new();
    let file = match open(
        unsafe { SCSI_ID_CONFIG },
        OFlag::O_RDONLY | OFlag::O_NONBLOCK | OFlag::O_CLOEXEC | OFlag::O_NOCTTY,
        stat::Mode::empty(),
    ) {
        Ok(fd) => unsafe { File::from_raw_fd(fd) },
        Err(e) => {
            return e as i32;
        }
    };

    let reader: BufReader<File> = BufReader::new(file);
    for (lineno, lines) in reader.lines().enumerate() {
        let line = match lines {
            Ok(line) => line,
            Err(e) => {
                log::debug!("read line err: {}", e.to_string());
                return -1;
            }
        };

        if line.clone().starts_with('#') {
            continue;
        }

        if line.clone().trim().is_empty() {
            continue;
        }

        let vec: Vec<String> = line.split(',').map(|s| s.to_string()).collect();
        for v in vec {
            let pair: Vec<String> = v.split('=').map(|s| s.to_string()).collect();
            if pair.len() > 2 {
                log::error!(
                    "failed to parse config file {} line {}",
                    unsafe { SCSI_ID_CONFIG },
                    lineno
                );
                return -1;
            }

            if pair[0].to_lowercase().starts_with("vendor") {
                vendor_in = pair[1].clone();
            }

            if pair[0].to_lowercase().starts_with("model") {
                model_in = pair[1].clone();
            }

            if pair[0].to_lowercase().starts_with("options") {
                options_in = pair[1].clone();
            }
        }

        if options_in.is_empty() || (vendor_in.is_empty() && !model_in.is_empty()) {
            log::error!(
                "failed to parse config file {} line {}",
                unsafe { SCSI_ID_CONFIG },
                lineno
            );
            return -1;
        }

        if vendor.is_empty() {
            if vendor_in.is_empty() {
                break;
            }
        } else if !vendor_in.is_empty()
            && vendor.starts_with(&vendor_in)
            && (model_in.is_empty() || model.starts_with(&model_in))
        {
            break;
        }
    }

    if vendor_in.is_empty() && model_in.is_empty() && options_in.is_empty() {
        log::error!("can't get any values from {}", unsafe { SCSI_ID_CONFIG });
        return -1;
    }
    unsafe {
        ARGV = options_in.split(' ').map(|s| s.to_string()).collect();
        ARGV.insert(0, "".to_owned());
        ARGC = ARGV.len() as i32
    };
    r
}

fn per_dev_options(dev_scsi: &mut ScsiIdDevice) -> i32 {
    let newargc: i32 = 0;
    let newargv: Vec<String> = Vec::new();

    let page = CString::new("page").unwrap();
    let denylisted = CString::new("denylisted").unwrap();
    let allowlisted = CString::new("allowlisted").unwrap();
    let blacklisted = CString::new("blacklisted").unwrap();
    let whitelisted = CString::new("whitelisted").unwrap();

    let longopts = [
        option {
            name: page.as_ptr(),
            has_arg: 1,
            flag: std::ptr::null_mut(),
            val: b'p' as c_int,
        },
        option {
            name: denylisted.as_ptr(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: b'b' as c_int,
        },
        option {
            name: allowlisted.as_ptr(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: b'g' as c_int,
        },
        option {
            name: blacklisted.as_ptr(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: b'b' as c_int,
        },
        option {
            name: whitelisted.as_ptr(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: b'g' as c_int,
        },
        option {
            name: std::ptr::null_mut(),
            has_arg: 0,
            flag: std::ptr::null_mut(),
            val: 0,
        },
    ];

    let mut r = get_file_options(dev_scsi.vendor.clone(), dev_scsi.model.clone());
    unsafe { optind = 1 };
    let optstring = CString::new("bgp:").unwrap();
    while r == 0 {
        unsafe {
            let o = getopt_long(
                newargc,
                newargv.as_ptr() as *const *mut c_char,
                optstring.as_ptr(),
                longopts.as_ptr(),
                &mut 0,
            );
            if o == -1 {
                break;
            }
            match o as u8 as char {
                'b' => ALL_GOOD = false,
                'g' => ALL_GOOD = true,
                'p' => {
                    let optarg_tmp = CStr::from_ptr(optarg as *const c_char);
                    let pagecode = optarg_tmp.to_str().unwrap();
                    match pagecode {
                        "0x80" => DEFAULT_PAGE_CODE = PageCode::Page80,
                        "0x83" => DEFAULT_PAGE_CODE = PageCode::Page83,
                        "pre-spc3-83" => DEFAULT_PAGE_CODE = PageCode::Page83PreSpc3,
                        _ => {
                            log::error!("unknown page code.");
                            return -1;
                        }
                    }
                }
                _ => {
                    println!("invalid arguments");
                    r = -1;
                    break;
                }
            }
        }
    }
    r
}

fn do_scsi_page0_inquiry(
    dev_scsi: &mut ScsiIdDevice,
    fd: i32,
    buffer: &mut [c_uchar],
    len: u32,
) -> bool {
    let r = scsi_inquiry(dev_scsi, fd, 1, 0x0, buffer, len);
    if r < 0 {
        return false;
    }

    if buffer[1] != 0_u8 {
        log::debug!("{}: page 0 not available.", dev_scsi.kernel);
        return false;
    }

    if (buffer[3] as u32) > len {
        log::debug!("{}: page 0 buffer too long {}", dev_scsi.kernel, buffer[3]);
        return false;
    }

    if (buffer[3] as i32) > (MODEL_LENGTH as i32)
        && String::from_utf8(buffer[VENDOR_LENGTH..2 * VENDOR_LENGTH].to_vec()).unwrap()
            != dev_scsi.vendor[VENDOR_LENGTH..]
    {
        log::debug!("{}: invalid page0 data", dev_scsi.kernel);
        return false;
    }

    true
}

fn append_vendor_model(dev_scsi: &mut ScsiIdDevice) -> i32 {
    if dev_scsi.vendor.len() >= VENDOR_LENGTH {
        dev_scsi.serial += &dev_scsi.vendor[..VENDOR_LENGTH];
    } else {
        log::debug!(
            "append_vendor_model: {} bad vendor string {}",
            dev_scsi.kernel,
            dev_scsi.vendor
        );
        return -EINVAL;
    }
    if dev_scsi.model.len() >= MODEL_LENGTH {
        dev_scsi.serial += &dev_scsi.model[..MODEL_LENGTH];
    } else {
        log::debug!(
            "append_vendor_model: {} bad model string {}",
            dev_scsi.kernel,
            dev_scsi.model
        );
        return -EINVAL;
    }
    (VENDOR_LENGTH + MODEL_LENGTH).try_into().unwrap()
}

fn do_scsi_page80_inquiry(
    dev_scsi: &mut ScsiIdDevice,
    fd: i32,
    get_serial: bool,
    get_unit_serial_number: bool,
    max_len: i32,
) -> i32 {
    let mut buffer = [0_u8; SCSI_INQ_BUFF_LEN as usize];
    let r = scsi_inquiry(
        dev_scsi,
        fd,
        1,
        PageCode::Page80 as c_uchar,
        &mut buffer,
        SCSI_INQ_BUFF_LEN,
    );
    if r < 0 {
        return r;
    }

    if buffer[1] != (PageCode::Page80 as c_uchar) {
        log::debug!("{}: Invalid page 0x80.", dev_scsi.kernel);
        return 1;
    }

    let len = 1 + (VENDOR_LENGTH + MODEL_LENGTH) as i32 + buffer[3] as i32;

    if max_len < len {
        log::debug!(
            "{}: length {} too short - need {}.",
            dev_scsi.kernel,
            max_len,
            len
        );
        return 1;
    }

    let len: usize = buffer[3] as usize + 4;

    if get_serial {
        dev_scsi.serial = "S".to_string();
        if append_vendor_model(dev_scsi) < 0 {
            return 1;
        }
        dev_scsi.serial += &String::from_utf8(buffer[4..len].to_vec()).unwrap();
    }

    if get_unit_serial_number {
        dev_scsi.unit_serial_number = String::from_utf8(buffer[4..len].to_vec()).unwrap();
    } else {
        dev_scsi.serial_short = String::from_utf8(buffer[4..len].to_vec()).unwrap();
    }

    0
}

fn check_fill_0x83_prespc3(dev_scsi: &mut ScsiIdDevice, page_83: Vec<c_uchar>, len: i32) -> i32 {
    if dev_scsi.serial.is_empty() {
        dev_scsi.serial = format!("{:x}", SCSI_ID_NAA);
    } else {
        dev_scsi
            .serial
            .replace_range(0..=0, &format!("{:x}", SCSI_ID_NAA));
    }

    for i in 0..page_83[3] as i32 {
        dev_scsi.serial += &format!("{:x}", (page_83[4 + i as usize] as i32 & 0xf0) >> 4);
        dev_scsi.serial += &format!("{:x}", page_83[4 + i as usize] as i32 & 0x0f);
        if dev_scsi.serial.len() >= ((len - 3) as usize) {
            break;
        }
    }
    dev_scsi.serial_short = dev_scsi.serial.clone();
    0
}

fn check_fill_0x83_id(
    dev_scsi: &mut ScsiIdDevice,
    page_83: Vec<c_uchar>,
    id_search: scsi_id_search_values,
    max_len: i32,
) -> i32 {
    if (page_83[1] & 0x30) == 0x10 {
        if id_search.id_type != SCSI_ID_TGTGROUP {
            return 1;
        }
    } else if (page_83[1] & 0x30) != 0 {
        return 1;
    }

    if (page_83[1] & 0x0f) != id_search.id_type {
        return 1;
    }

    if id_search.naa_type != SCSI_ID_NAA_DONT_CARE && id_search.naa_type != (page_83[4] & 0xf0 >> 4)
    {
        return 1;
    }

    if (page_83[0] & 0x0f) != id_search.code_set {
        return 1;
    }

    let mut len: i32 = page_83[3] as i32;
    if (page_83[0] & 0x0f) != SCSI_ID_ASCII {
        len *= 2;
    }

    len += 2;
    if id_search.id_type == SCSI_ID_VENDOR_SPECIFIC {
        len += (VENDOR_LENGTH + MODEL_LENGTH) as i32;
    }

    if max_len < len {
        log::debug!(
            "{}: length {} too short - need {}",
            dev_scsi.kernel,
            max_len,
            len
        );
        return 1;
    }

    if id_search.id_type == SCSI_ID_TGTGROUP && !dev_scsi.tgpt_group.is_empty() {
        let group = ((page_83[6] as c_uint) << 8) | page_83[7] as c_uint;
        dev_scsi.tgpt_group = format!("{:x}", group);
        return 1;
    }

    if !dev_scsi.serial.is_empty() {
        dev_scsi
            .serial
            .replace_range(0..=0, &format!("{:x}", id_search.id_type));
    } else {
        dev_scsi.serial = format!("{:x}", id_search.id_type);
    }

    if id_search.id_type == SCSI_ID_VENDOR_SPECIFIC && append_vendor_model(dev_scsi) < 0 {
        return -1;
    }

    let mut i = 4; /* offset to the start of the identifier */
    let s = dev_scsi.serial.len();
    if (page_83[0] as c_uchar & 0x0f) == SCSI_ID_ASCII {
        let range = 4..(4 + page_83[3]);
        dev_scsi.serial +=
            &String::from_utf8(page_83[range.start as usize..range.end as usize].to_vec()).unwrap();
    } else {
        while i < (4 + page_83[3] as i32) {
            dev_scsi.serial += &format!("{:x}", (page_83[i as usize] as usize & 0xf0) >> 4);
            dev_scsi.serial += &format!("{:x}", page_83[i as usize] as usize & 0x0f);
            i += 1;
        }
    }

    dev_scsi.serial_short = dev_scsi.serial[s..].to_string();

    if id_search.id_type == SCSI_ID_NAA && dev_scsi.wwn.is_empty() {
        let len = dev_scsi.serial.len();
        if len >= (s + 16) {
            dev_scsi.wwn = dev_scsi.serial[s..s + 16].to_string();
        } else {
            dev_scsi.wwn = dev_scsi.serial[s..].to_string();
        }

        if dev_scsi.wwn_vendor_extension.is_empty() {
            if len >= (s + 32) {
                dev_scsi.wwn_vendor_extension = dev_scsi.serial[s + 16..s + 32].to_string();
            } else if len >= (s + 16) {
                dev_scsi.wwn_vendor_extension = dev_scsi.serial[s + 16..].to_string();
            }
        }
    }

    0
}

fn do_scsi_page83_inquiry(dev_scsi: &mut ScsiIdDevice, fd: i32, len: i32) -> i32 {
    let mut page_83 = vec![0_u8; SCSI_INQ_BUFF_LEN as usize];
    do_scsi_page80_inquiry(dev_scsi, fd, false, true, MAX_SERIAL_LEN);
    let mut r = scsi_inquiry(
        dev_scsi,
        fd,
        1,
        PageCode::Page83 as u8,
        &mut page_83,
        SCSI_INQ_BUFF_LEN,
    );
    if r < 0 {
        return 1;
    }

    if page_83[1] != (PageCode::Page83 as c_uchar) {
        log::debug!("{}: Invalid page 0x83.", dev_scsi.kernel);
        return 1;
    }

    if page_83[6] != 0_u8 {
        return check_fill_0x83_prespc3(dev_scsi, page_83, len);
    }

    for id in &ID_SEARCH_LIST {
        let mut i = 4;
        while i <= (((page_83[2] as c_uint) << 8) + page_83[3] as c_uint + 3) {
            r = check_fill_0x83_id(
                dev_scsi,
                page_83.clone().split_off(i.try_into().unwrap()),
                *id,
                len,
            );
            if r <= 0 {
                return r;
            }
            i += page_83[i as usize + 3] as c_uint + 4;
        }
    }

    1
}

fn do_scsi_page83_prespc3_inquiry(dev_scsi: &mut ScsiIdDevice, fd: i32) -> i32 {
    let mut page_83 = vec![0_u8; SCSI_INQ_BUFF_LEN as usize];
    let r = scsi_inquiry(
        dev_scsi,
        fd,
        1,
        PageCode::Page83 as u8,
        &mut page_83,
        SCSI_INQ_BUFF_LEN,
    );
    if r < 0 {
        return 1;
    }

    if page_83[1] != (PageCode::Page83 as c_uchar) {
        log::debug!("{}: Invalid page 0x83.", dev_scsi.kernel);
        return 1;
    }

    if page_83[6] == 0_u8 {
        return 2;
    }

    if dev_scsi.serial.is_empty() {
        dev_scsi.serial += &format!("{:x}", SCSI_ID_NAA);
    } else {
        dev_scsi
            .serial
            .replace_range(0..1, &format!("{:x}", SCSI_ID_NAA));
    }

    let mut i = 4;

    while i < (page_83[3] as i32 + 4) {
        dev_scsi.serial += &format!("{:x}", (page_83[i as usize] as usize & 0xf0) >> 4);
        dev_scsi.serial += &format!("{:x}", page_83[i as usize] as usize & 0x0f);
        i += 1;
    }

    0
}

fn scsi_get_serial(
    dev_scsi: &mut ScsiIdDevice,
    devname: &str,
    page_code: &PageCode,
    len: i32,
) -> i32 {
    dev_scsi.serial = String::new();
    let mut fd = -EBADF;
    let mut page0 = vec![0_u8; SCSI_INQ_BUFF_LEN as usize];
    for _i in 0..=20 {
        fd = match open(
            devname,
            OFlag::O_RDONLY | OFlag::O_NONBLOCK | OFlag::O_CLOEXEC | OFlag::O_NOCTTY,
            Mode::from_bits(0o666).unwrap(),
        ) {
            Ok(fd) => fd,
            Err(e) => {
                log::error!("Cannot open {}: {}", devname, e);
                if e != Errno::EBUSY {
                    break;
                }
                -1
            }
        };

        if fd >= 0 {
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }

    if fd < 0 {
        return 1;
    }

    match page_code {
        PageCode::Page80 => {
            if do_scsi_page80_inquiry(dev_scsi, fd, true, false, len) != 0 {
                return 1;
            } else {
                return 0;
            }
        }
        PageCode::Page83 => {
            if do_scsi_page83_inquiry(dev_scsi, fd, len) != 0 {
                return 1;
            } else {
                return 0;
            }
        }
        PageCode::Page83PreSpc3 => {
            let r = do_scsi_page83_prespc3_inquiry(dev_scsi, fd);
            if r != 0 {
                if r == 2 {
                    if do_scsi_page83_inquiry(dev_scsi, fd, len) != 0 {
                        return 1;
                    } else {
                        return 0;
                    }
                } else {
                    return 1;
                }
            } else {
                return r;
            }
        }
        PageCode::PageUnspecified => {}
    }

    if !do_scsi_page0_inquiry(dev_scsi, fd, &mut page0, SCSI_INQ_BUFF_LEN) {
        return 1;
    }

    for ind in 4..=(page0[3] as i32 + 3) {
        if page0[ind as usize] == (PageCode::Page83 as c_uchar)
            && do_scsi_page83_inquiry(dev_scsi, fd, len) == 0
        {
            return 0;
        }
    }

    for ind in 4..=(page0[3] as i32 + 3) {
        if page0[ind as usize] == (PageCode::Page80 as c_uchar)
            && do_scsi_page80_inquiry(dev_scsi, fd, true, false, len) == 0
        {
            return 0;
        }
    }
    1
}

fn scsi_id(maj_min_dev: &str) -> i32 {
    let mut dev_scsi = ScsiIdDevice::default();
    let page_code = unsafe { &DEFAULT_PAGE_CODE };

    if set_inq_values(&mut dev_scsi, maj_min_dev) < 0 {
        return 1;
    }

    per_dev_options(&mut dev_scsi);
    if !unsafe { ALL_GOOD } {
        return 1;
    }

    scsi_get_serial(&mut dev_scsi, maj_min_dev, page_code, MAX_SERIAL_LEN);
    unsafe {
        if EXPORT {
            println!("ID_SCSI=1");
            println!("ID_VENDOR={}", &VENDOR);
            println!("ID_VENDOR_ENC={}", &VENDOR_ENC_STR);
            println!("ID_MODEL={}", &MODEL);
            println!("ID_MODEL_ENC={}", &MODEL_ENC_STR);
            println!("ID_REVISION={}", &REVISION);
            println!("ID_TYPE={}", &TYPE_STR);

            if !dev_scsi.serial.is_empty() {
                dev_scsi.serial = replace_whitespace(&dev_scsi.serial);
                dev_scsi.serial = replace_chars(&dev_scsi.serial, "");
                println!("ID_SERIAL={}", dev_scsi.serial);
                dev_scsi.serial_short = replace_whitespace(&dev_scsi.serial_short);
                dev_scsi.serial_short = replace_chars(&dev_scsi.serial_short, "");
                println!("ID_SERIAL_SHORT={}", dev_scsi.serial_short);
            }

            if !dev_scsi.wwn.is_empty() {
                println!("ID_WWN=0x{}", dev_scsi.wwn);
                if !dev_scsi.wwn_vendor_extension.is_empty() {
                    println!(
                        "ID_WWN_VENDOR_EXTENSION=0x{}",
                        dev_scsi.wwn_vendor_extension
                    );
                    println!(
                        "ID_WWN_WITH_EXTENSION=0x{}{}",
                        dev_scsi.wwn, dev_scsi.wwn_vendor_extension
                    );
                } else {
                    println!("ID_WWN_WITH_EXTENSION=0x{}", dev_scsi.wwn);
                }
            }

            if !dev_scsi.tgpt_group.is_empty() {
                println!("ID_TARGET_PORT={}", dev_scsi.tgpt_group);
            }

            if !dev_scsi.unit_serial_number.is_empty() {
                println!("ID_SCSI_SERIAL={}", dev_scsi.unit_serial_number);
            }

            return 0;
        }
    }

    if dev_scsi.serial.is_empty() {
        return 1;
    }

    if unsafe { REFORMAT_SERIAL } {
        dev_scsi.serial = replace_whitespace(&dev_scsi.serial);
        dev_scsi.serial = replace_chars(&dev_scsi.serial, "");
        println!("{}", dev_scsi.serial);
        return 0;
    }

    println!("{}", dev_scsi.serial);
    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let argc = args.len() as i32;

    let r = get_file_options(String::from(""), String::from(""));
    if r < 0 {
        exit(1);
    }

    if r == 0 && set_options(unsafe { ARGC }, unsafe { ARGV.clone() }) < 0 {
        exit(2);
    }

    if set_options(argc, args) < 0 {
        exit(1);
    }

    if !unsafe { DEV_SPECIFIED } {
        log::error!("No device specified!");
        exit(1);
    }

    let r = scsi_id(unsafe { &MAJ_MIN_DEV });
    exit(r);
}
