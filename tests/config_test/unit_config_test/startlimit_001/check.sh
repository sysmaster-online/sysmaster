#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test default StartLimitBurst=5/StartLimitInterval=10
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sctl daemon-reload
    for ((i = 0; i < 5; ++i)); do
        sctl start base.service
        expect_eq $? 0 || return 1
        check_status base.service active
        expect_eq $? 0 || return 1
        sctl stop base.service
        check_status base.service inactive
        expect_eq $? 0 || return 1
    done
    sctl start base.service
    expect_eq $? 1 || return 1
    check_status base.service failed
    expect_eq $? 0 || return 1

    sleep 9
    sctl start base.service
    expect_eq $? 1 || return 1
    check_status base.service failed
    expect_eq $? 0 || return 1

    sleep 2
    sctl start base.service
    expect_eq $? 0 || return 1
    check_status base.service active
    expect_eq $? 0 || return 1

    # clean
    sctl stop base.service
}

# usage: test StartLimitBurst
function test02() {
    log_info "===== test02 ====="
    sed -i '/Description/ a StartLimitBurst=3' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    for ((i = 0; i < 3; ++i)); do
        sctl start base.service
        expect_eq $? 0 || return 1
        check_status base.service active
        expect_eq $? 0 || return 1
        sctl stop base.service
        check_status base.service inactive
        expect_eq $? 0 || return 1
    done
    sctl start base.service
    expect_eq $? 1 || return 1
    check_status base.service failed
    expect_eq $? 0 || return 1
}

# usage: test StartLimitInterval
function test03() {
    log_info "===== test03 ====="
    sed -i '/Description/ a StartLimitInterval=3' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    for ((i = 0; i < 3; ++i)); do
        sctl start base.service
        expect_eq $? 0 || return 1
        check_status base.service active
        expect_eq $? 0 || return 1
        sctl stop base.service
        check_status base.service inactive
        expect_eq $? 0 || return 1
    done
    sctl start base.service
    expect_eq $? 1 || return 1
    check_status base.service failed
    expect_eq $? 0 || return 1

    sleep 4
    sctl start base.service
    expect_eq $? 0 || return 1
    check_status base.service active
    expect_eq $? 0 || return 1

    # clean
    sctl stop base.service
}

# usage: test StartLimitBurst=0
function test04() {
    log_info "===== test04 ====="
    sed -i '/StartLimitBurst=/ s/3/0/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    for ((i = 0; i < 50; ++i)); do
        sctl start base.service
        expect_eq $? 0 || return 1
        check_status base.service active
        expect_eq $? 0 || return 1
        sctl stop base.service
        check_status base.service inactive
        expect_eq $? 0 || return 1
    done
}

# usage: test StartLimitInterval=0
function test05() {
    log_info "===== test05 ====="
    sed -i '/StartLimitBurst=/ s/0/3/' ${SYSMST_LIB_PATH}/base.service
    sed -i '/StartLimitInterval=/ s/3/0/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    for ((i = 0; i < 50; ++i)); do
        sctl start base.service
        expect_eq $? 0 || return 1
        check_status base.service active
        expect_eq $? 0 || return 1
        sctl stop base.service
        check_status base.service inactive
        expect_eq $? 0 || return 1
    done
}

# usage: test StartLimitAction
function test06() {
    log_info "===== test06 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Description/ a StartLimitAction=' ${SYSMST_LIB_PATH}/base.service
    # service success: reboot, poweroff, exit
    local cmd_list="reboot poweroff exit"
    for cmd in ${cmd_list}; do
        cp -arf "${work_dir}"/tmp_units/"${cmd}".target ${SYSMST_LIB_PATH} || return 1
        sed -i "s/StartLimitAction=.*/StartLimitAction=${cmd}/" ${SYSMST_LIB_PATH}/base.service
        sctl daemon-reload
        for ((i=0; i<5; ++i)); do
            sctl restart base
            sleep 0.1
        done
        echo > "${SYSMST_LOG}"
        sctl restart base
        check_status base failed
        expect_eq $? 0 || return 1
        ps aux | grep -v grep | awk '{print $2}' | grep -w "${sysmaster_pid}"
        expect_eq $? 0 || ps -elf
        check_log "${SYSMST_LOG}" "by starting ${cmd}.target caused by unit base.service hit StartLimit"
        expect_eq $? 0 || return 1
        sctl reset-failed base
    done
}

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
test05 || exit 1
test06 || exit 1
exit "${EXPECT_FAIL}"
