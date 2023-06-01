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

    if test -f /etc/devmaster/config.toml; then
        mv /etc/devmaster/config.toml /etc/devmaster/config.toml.back
    fi

    if [[ ${0%/*} == $0 ]]; then
        path=""
    else
        path=${0%/*}
    fi

    echo "rules_d = [\"$(pwd)/${path}/rules.d\"]" > /etc/devmaster/config.toml
    cat <<EOF > /run/devmaster/test/properties.txt
#NATION=China
PEOPLE=Xiaoming
GENDER="Male"
HEIGHT='188'
=B
A=
INVALID"
EOF
}

function cleanup()
{
    rm -f /etc/devmaster/config.toml
    if test -f /etc/devmaster/config.toml.back; then
        mv /etc/devmaster/config.toml.back /etc/devmaster/config.toml
    fi
    rm -f /run/devmaster/test/properties.txt
}

function run_devmaster()
{
    cargo run -p devmaster --bin devmaster
}

trap cleanup EXIT

prepare

run_devmaster
