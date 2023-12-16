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

void dump(struct udev_hwdb *hwdb, const char *modalias)
{
    struct udev_list_entry *list = NULL;
    udev_list_entry_foreach(list, udev_hwdb_get_properties_list_entry(hwdb, modalias, 0))
    {
        printf("%s=%s\n", udev_list_entry_get_name(list), udev_list_entry_get_value(list));
    }

    udev_hwdb_unref(hwdb);
}

void main()
{
    struct udev_hwdb *hwdb = udev_hwdb_new(NULL);
    while (1)
    {
        dump(udev_hwdb_ref(hwdb), "evdev:input:b0003v0458p07081");
        dump(udev_hwdb_ref(hwdb), "evdev:input:b0003v06CBp00091");
        sleep(1);
    }
    udev_hwdb_unref(hwdb);
}
