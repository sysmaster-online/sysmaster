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

//! scsi generic info
//!

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(deref_nullptr)]
#![allow(unused)]
#![allow(missing_docs)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::derivable_impls)]

include!(concat!(env!("OUT_DIR"), "/scsi_generic.rs"));

impl Default for sg_io_v4 {
    fn default() -> Self {
        sg_io_v4 {
            guard: 0,
            protocol: 0,
            subprotocol: 0,
            request_len: 0,
            request: 0,
            request_tag: 0,
            request_attr: 0,
            request_priority: 0,
            request_extra: 0,
            max_response_len: 0,
            response: 0,
            dout_iovec_count: 0,
            dout_xfer_len: 0,
            din_iovec_count: 0,
            din_xfer_len: 0,
            dout_xferp: 0,
            din_xferp: 0,
            timeout: 0,
            flags: 0,
            usr_ptr: 0,
            spare_in: 0,
            driver_status: 0,
            transport_status: 0,
            device_status: 0,
            retry_delay: 0,
            info: 0,
            duration: 0,
            response_len: 0,
            din_resid: 0,
            dout_resid: 0,
            generated_tag: 0,
            spare_out: 0,
            padding: 0,
        }
    }
}

impl Default for sg_io_hdr {
    fn default() -> Self {
        sg_io_hdr {
            interface_id: 0,
            dxfer_direction: 0,
            cmd_len: 0,
            mx_sb_len: 0,
            iovec_count: 0,
            dxfer_len: 0,
            dxferp: std::ptr::null_mut(),
            cmdp: std::ptr::null_mut(),
            sbp: std::ptr::null_mut(),
            timeout: 0,
            flags: 0,
            pack_id: 0,
            usr_ptr: std::ptr::null_mut(),
            status: 0,
            masked_status: 0,
            msg_status: 0,
            sb_len_wr: 0,
            host_status: 0,
            driver_status: 0,
            resid: 0,
            duration: 0,
            info: 0,
        }
    }
}
