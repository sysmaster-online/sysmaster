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

//! example for some Device:from_* methods
//! note that this example depends on the devices existing in your system
//!
use device::Device;

fn main() {
    {
        let dev_1 = Device::from_subsystem_sysname("drivers", "i2c:dummy").unwrap();
        let dev_2 = Device::from_device_id("+drivers:i2c:dummy").unwrap();
        assert_eq!(dev_1.get_sysname().unwrap(), dev_2.get_sysname().unwrap());
        assert_eq!(
            dev_1.get_subsystem().unwrap(),
            dev_2.get_subsystem().unwrap()
        );
    }

    {
        let dev = Device::from_subsystem_sysname("drivers", "usb:hub").unwrap();
        println!("{}", dev.get_sysname().unwrap());
        println!("{}", dev.get_subsystem().unwrap());
        println!("{}", dev.get_device_id().unwrap());
    }

    {
        let dev = Device::from_ifindex(2).unwrap();
        println!("{}", dev.get_device_id().unwrap());
    }
}
