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

/* Run this example, and it will check whether
 * devmaster is running if the libudev is
 * preloaded with that from devmaster.
 *
 * Notice the devmaster is recognized as running
 * if the socket file /run/devmaster/control exists.
 *
 * Simplely stop devmaster daemon will not clean up
 * the socket automatically. Clean the socket manually.
 */
void main()
{
    while (1)
    {
        int ret = udev_queue_get_udev_is_active(NULL);

        if (ret > 0)
            printf("devmaster running\n");
        else
            printf("heartbeat\n");

        sleep(1);
    }
}
