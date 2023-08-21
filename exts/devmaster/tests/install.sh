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

#!/bin/bash

mode="${1:-debug}"
pwd=$(pwd)
target_dir=${pwd}/target/${mode}
devmaster_install_target=/usr/lib/devmaster
conf_install_target=/etc/devmaster
config_dir=${pwd}/exts/devmaster/config

install -Dm0550 -t /usr/bin ${target_dir}/devctl || exit 1
install -Dm0550 -t ${devmaster_install_target} ${target_dir}/devmaster || exit 1
install -Dm0550 -t ${devmaster_install_target} ${target_dir}/ata_id || exit 1

install -Dm0640 -t ${conf_install_target} ${config_dir}/config.toml || exit 1
install -Dm0640 -t ${conf_install_target}/rules.d ${config_dir}/rules.d/* || exit 1
install -Dm0640 -t ${conf_install_target}/network.d ${config_dir}/network.d/* || exit 1
