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
#

# Prepare test configuration and run devmaster

#!/usr/bin/env bash

function prepare()
{
    mkdir -p /etc/devmaster
    mkdir -p /run/devmaster/test
    mkdir -p /run/devmaster/tmp

    if test -f /etc/devmaster/config.toml; then
        mv /etc/devmaster/config.toml /run/devmaster/tmp/config.toml
    fi

    if [[ ${0%/*} == $0 ]]; then
        path=""
    else
        path=${0%/*}
    fi

    echo "rules_d = [\"$(pwd)/${path}/rules.d\"]
log_level = \"debug\"" > /etc/devmaster/config.toml

    cat <<EOF > /run/devmaster/test/properties.txt
#NATION=China
PEOPLE=Xiaoming
GENDER="Male"
HEIGHT='188'
=B
A=
INVALID"
EOF

    # backup rules in /etc/udev and create rules files with the same name as that in /lib/udev/rules.d/
    # to avoid running udevd to execute rules and disturb devmaster
    mkdir -p /run/devmaster/tmp/rules.d
    cp /etc/udev/rules.d/* /run/devmaster/tmp/rules.d/
    ls /etc/udev/rules.d/* | xargs rm -f
    ls /lib/udev/rules.d/* | sed 's/\/lib/\/etc/g' | xargs touch

    udevadm control -R
    udevadm info --cleanup-db
}

function cleanup()
{
    rm -f /etc/devmaster/config.toml
    if test -f /run/devmaster/tmp/config.toml; then
        mv /run/devmaster/tmp/config.toml /etc/devmaster/config.toml
    fi
    rm -f /run/devmaster/test/properties.txt

    ls /etc/udev/rules.d/* | xargs rm -f
    cp /run/devmaster/tmp/rules.d/* /etc/udev/rules.d/
    rm -rf /run/devmaster/tmp
}

function run_devmaster()
{
    cargo run -p devmaster --bin devmaster
}

trap cleanup EXIT

prepare

run_devmaster
