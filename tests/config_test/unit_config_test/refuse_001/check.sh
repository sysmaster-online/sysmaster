#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

key_log_1='unit can not be started manually'
key_log_2='unit can not be stopped manually'
exp_ret=1

# usage: test RefuseManualStart/RefuseManualStop=false
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Description/ a RefuseManualStart=false' ${SYSMST_LIB_PATH}/base.service
    sed -i '/Description/ a RefuseManualStop=false' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    expect_eq $? 0
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    expect_eq $? 0
    check_status base inactive
    expect_eq $? 0 || return 1
    sctl start base
    expect_eq $? 0
    check_status base active
    expect_eq $? 0 || return 1

    # clean
    sctl stop base
    check_status base inactive
    expect_eq $? 0
}

# usage: test RefuseManualStart/RefuseManualStop=true
function test02() {
    log_info "===== test02 ====="
    sed -i 's/false/true/g' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl start base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_1}"
    expect_eq $? 0
    sctl restart base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_2}"
    expect_eq $? 0
    rm -rf log
    sctl status base
    expect_eq $? 3

    sed -i '/RefuseManualStart/ s/true/false/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl start base
    expect_eq $? 0
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_2}"
    expect_eq $? 0
    check_status base active
    expect_eq $? 0 || return 1
    sctl restart base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_2}"
    expect_eq $? 0
    main_pid="$(get_pids base)"
    kill -9 "${main_pid}"
    check_status base failed
    expect_eq $? 0 || return 1
    sctl restart base
    expect_eq $? "${exp_ret}"
    check_status base failed
    expect_eq $? 0 || return 1

    sed -i 's/false/true/g' ${SYSMST_LIB_PATH}/base.service
    sed -i '/RefuseManualStop/ s/true/false/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_1}"
    expect_eq $? 0
    sctl start base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_1}"
    expect_eq $? 0
}

# usage: test RefuseManualStart/RefuseManualStop=true, but start by dependency
function test03() {
    log_info "===== test03 ====="
    cp -arf "${work_dir}"/tmp_units/requires.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/false/true/g' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl status base
    expect_eq $? 3
    sctl restart requires
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base &> log
    expect_eq $? "${exp_ret}"
    check_log log "${key_log_2}"
    expect_eq $? 0
    check_status base active
    expect_eq $? 0 || return 1
}

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
test03 || exit 1
exit "${EXPECT_FAIL}"
