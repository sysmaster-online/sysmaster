# Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
#
# sysMaster is licensed under Mulan PSL v2.
# You can use this software according to the terms and conditions of the Mulan
# PSL v2.
# You may obtain a copy of Mulan PSL v2 at:
#         http://license.coscl.org.cn/MulanPSL2
# THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
# KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
# NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
# See the Mulan PSL v2 for more details.

#!/usr/bin/env bash

# compare triggered devices by udevadm and devctl

if [[ -n $(command -v udevadm) ]];
then
    echo "export udevadm triggered devices"
    udevadm trigger --verbose --dry-run > /run/udev_tmp.txt
else
    echo "udev does not exist"
    exit 0
fi

echo "export devctl triggered devices"
cargo run -p devmaster --bin devctl -- trigger --verbose --dry-run > /run/devmaster_tmp.txt

diff /run/udev_tmp.txt /run/devmaster_tmp.txt
