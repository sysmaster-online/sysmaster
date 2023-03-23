#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test default StartLimitBurst=5/StartLimitInterval=10
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    for ((i = 0; i < 5; ++i)); do
        sctl start base.service
        expect_eq $? 0 || return 1
    check_status base.service active || return 1
    sctl stop base.service
    check_status base.service inactive || return 1
    done
    sctl start base.service
    expect_eq $? 1 || return 1
    check_status base.service failed || return 1

    sleep 9
    sctl start base.service
    expect_eq $? 1 || return 1
    check_status base.service failed || return 1

    sleep 2
    sctl start base.service
    expect_eq $? 0 || return 1
    check_status base.service active || return 1

    # clean
    sctl stop base.service
    kill_sysmaster
}

# usage: test StartLimitBurst
function test02() {
    log_info "===== test02 ====="
    sed -i '/Description/ a StartLimitBurst=3' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    for ((i = 0; i < 3; ++i)); do
        sctl start base.service
    expect_eq $? 0 || return 1
    check_status base.service active || return 1
    sctl stop base.service
    check_status base.service inactive || return 1
    done
    sctl start base.service
    expect_eq $? 1 || return 1
    check_status base.service failed || return 1

    # clean
    kill_sysmaster
}

# usage: test StartLimitInterval
function test03() {
    log_info "===== test03 ====="
    sed -i '/Description/ a StartLimitInterval=3' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    for ((i = 0; i < 3; ++i)); do
        sctl start base.service
        expect_eq $? 0 || return 1
        check_status base.service active || return 1
        sctl stop base.service
        check_status base.service inactive || return 1
    done
    sctl start base.service
    expect_eq $? 1 || return 1
    check_status base.service failed || return 1

    sleep 4
    sctl start base.service
    expect_eq $? 0 || return 1
    check_status base.service active || return 1

    # clean
    sctl stop base.service
    kill_sysmaster
}

# usage: test StartLimitBurst=0
function test04() {
    log_info "===== test04 ====="
    sed -i '/StartLimitBurst=/ s/3/0/' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    for ((i = 0; i < 50; ++i)); do
        sctl start base.service
        expect_eq $? 0 || return 1
    check_status base.service active || return 1
    sctl stop base.service
    check_status base.service inactive || return 1
    done

    # clean
    kill_sysmaster
}

# usage: test StartLimitInterval=0
function test05() {
    log_info "===== test05 ====="
    sed -i '/StartLimitBurst=/ s/0/3/' ${SYSMST_LIB_PATH}/base.service
    sed -i '/StartLimitInterval=/ s/3/0/' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    for ((i = 0; i < 50; ++i)); do
        sctl start base.service
        expect_eq $? 0 || return 1
        check_status base.service active || return 1
        sctl stop base.service
        check_status base.service inactive || return 1
    done

    # clean
    kill_sysmaster
}

test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
test05 || exit 1
exit "${EXPECT_FAIL}"
