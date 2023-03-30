#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

key_log_1='unit can not be started manually'
key_log_2='unit can not be stoppeded manually'
key_log_3='unit can not be started/stopped manually'
exp_ret=1

# usage: test RefuseManualStart/RefuseManualStop=false
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Description/ a RefuseManualStart=false' ${SYSMST_LIB_PATH}/base.service
    sed -i '/Description/ a RefuseManualStop=false' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl restart base
    expect_eq $? 0
    check_status base active || return 1
    sctl stop base
    expect_eq $? 0
    check_status base inactive || return 1
    sctl start base
    expect_eq $? 0
    check_status base active || return 1

    # clean
    sctl stop base
    check_status base inactive
    kill_sysmaster
}

# usage: test RefuseManualStart/RefuseManualStop=true
function test02() {
    log_info "===== test02 ====="
    sed -i 's/false/true/g' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_1}"
    sctl restart base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_3}"
    rm -rf log
    sctl status base
    expect_eq $? 1
    # clean
    kill_sysmaster

    sed -i '/RefuseManualStart/ s/true/false/' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    expect_eq $? 0
    check_status base active || return 1
    sctl stop base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_2}"
    check_status base active || return 1
    sctl restart base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_3}"
    main_pid="$(get_pids base)"
    kill -9 "${main_pid}"
    check_status base failed || return 1
    sctl restart base
    expect_eq $? 0
    check_status base active || return 1
    # clean
    kill_sysmaster

    sed -i 's/false/true/g' ${SYSMST_LIB_PATH}/base.service
    sed -i '/RefuseManualStop/ s/true/false/' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl restart base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_3}"
    sctl start base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_1}"
    # clean
    kill_sysmaster
}

# usage: test RefuseManualStart/RefuseManualStop=true, but start by dependency
function test03() {
    log_info "===== test03 ====="
    cp -arf "${work_dir}"/tmp_units/requires.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/false/true/g' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl status base
    expect_eq $? 1
    sctl restart requires
    check_status base active || return 1
    sctl stop base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_2}"
    check_status base active || return 1

    # clean
    kill_sysmaster
}

test01 || exit 1
test02 || exit 1
test03 || exit 1
exit "${EXPECT_FAIL}"
