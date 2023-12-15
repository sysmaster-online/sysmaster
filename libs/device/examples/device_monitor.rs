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

//! Example for monitoring uevent sended by devmaster.
//!
//! This example shows the different impunity whether using bpf filter or not.
//!
//! Run this example with devmaster running, and use devctl to trigger any
//! disks with partitions.
//!
//! The partition disks will be monitored and be printed, while the whole block
//! disk will be filtered.
//!
//! The monitor m1 that used bpf filter won't raise messages like 'm1 filtered',
//! while the monitor m2 that filters packets by itself will raise messages like
//! 'm2 filtered'.
//!

use device::device_monitor::*;

fn main() {
    let mut m1 = DeviceMonitor::new(MonitorNetlinkGroup::Userspace, None);
    m1.filter_add_match_subsystem_devtype("block", "partition")
        .unwrap();

    let mut m2 = DeviceMonitor::new(MonitorNetlinkGroup::Userspace, None);
    m2.filter_add_match_subsystem_devtype("block", "partition")
        .unwrap();

    /* If bpf_filter_update is called, the illegal uevent packages
     * are filtered inside OS and thus this example will not print
     * "monitor filtered" messages.
     */
    m1.bpf_filter_update().unwrap();

    /* Run devmaster and trigger uevents for partitions */
    loop {
        if let Ok(d) = m1.receive_device() {
            match d {
                Some(dev) => {
                    println!("m1 receive device: {}", dev.get_syspath().unwrap());
                }
                None => {
                    /* m1 has called bpf_filter_update method,
                     * thus the following message will not be printed.
                     */
                    println!("m1 filtered");
                }
            }
        }

        if let Ok(d) = m2.receive_device() {
            match d {
                Some(dev) => {
                    println!("m2 receive device: {}", dev.get_syspath().unwrap());
                }
                None => {
                    /* m2 did not called bpf_filter_update method,
                     * it will filter the uevent packages by itself and
                     * print the following message.
                     */
                    println!("m2 filtered");
                }
            }
        }
    }
}
