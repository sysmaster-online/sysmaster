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

#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

#include <libudev.h>

void main()
{
    struct udev_monitor *m = udev_monitor_new_from_netlink(NULL, "kernel");
    udev_monitor_filter_add_match_subsystem_devtype(m, "block", NULL);
    udev_monitor_filter_add_match_subsystem_devtype(m, "block", "partition");
    udev_monitor_enable_receiving(m);
    while (1)
    {
        struct udev_device *d = udev_monitor_receive_device(m);
        if (d != NULL)
        {
            printf("%s\n", udev_device_get_syspath(d));
            udev_device_unref(d);
        }
    }
}
