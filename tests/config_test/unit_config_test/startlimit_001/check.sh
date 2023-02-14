#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test StartLimitBurst
function test01() {
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i "/Description/ a StartLimitBurst=\"3\"" ${SYSMST_LIB_PATH}/base.service
    sed -i "/Description/ a StartLimitInterval=\"10s\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    for ((i = 0; i < 3; ++i)); do
        sctl restart base.service
	expect_eq $? 0 || return 1
    done
    sctl restart base.service
    expect_eq $? 1 || return 1
    check_status base failed
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'asdaasd' || return 1

    sleep 4
    sctl restart base.service
    expect_eq $? 1 || return 1

    sleep 7
    sctl restart base.service
    expect_eq $? 0 || return 1
    check_status base active
    expect_eq $? 0 || return 1

    # clean
    sctl stop base.service
    kill -9 "${sysmaster_pid}"
}

# usage: test StartLimitInterval
function test02() {
    sed -i "/StartLimitInterval=/ s/10s/3/" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    for ((i = 0; i < 3; ++i)); do
        sctl restart base.service
        expect_eq $? 0 || return 1
    done
    sctl restart base.service
    expect_eq $? 1 || return 1
    check_status base failed
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'asdaasd' || return 1

    sleep 4
    sctl restart base.service
    expect_eq $? 0 || return 1
    check_status base active
    expect_eq $? 0 || return 1

    # clean
    sctl stop base.service
    kill -9 "${sysmaster_pid}"
}

# usage: test StartLimitBurst=0
function test03() {
    sed -i "/StartLimitBurst=/ s/3/0/" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    for ((i = 0; i < 50; ++i)); do
        sctl restart base.service
        expect_eq $? 0 || return 1
    done
    check_status base active

    # clean
    sctl stop base.service
    kill -9 "${sysmaster_pid}"
}

# usage: test StartLimitInterval=0
function test04() {
    sed -i "/StartLimitBurst=/ s/0/3/" ${SYSMST_LIB_PATH}/base.service
    sed -i "/StartLimitInterval=/ s/3/0/" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    for ((i = 0; i < 4; ++i)); do
        sctl restart base.service
        expect_eq $? 0 || return 1
    done
    check_status base active

    # clean
    sctl stop base.service
    kill -9 "${sysmaster_pid}"
}

test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
