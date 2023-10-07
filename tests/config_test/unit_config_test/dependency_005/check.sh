#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test Before/After
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/{fork.service,before.service,after.service} ${SYSMST_LIB_PATH} || return 1
    sctl daemon-reload
    sctl start fork.service after.service before.service &
    sleep 1
    check_status before.service activating
    expect_eq $? 0 || return 1
    check_status fork.service inactive
    expect_eq $? 0 || return 1
    check_status after.service inactive
    expect_eq $? 0 || return 1
    sleep 5
    check_status before.service inactive
    expect_eq $? 0 || return 1
    check_status fork.service activating
    expect_eq $? 0 || return 1
    check_status after.service inactive
    expect_eq $? 0 || return 1
    sleep 5
    check_status before.service inactive
    expect_eq $? 0 || return 1
    check_status fork.service inactive
    expect_eq $? 0 || return 1
    check_status after.service activating
    expect_eq $? 0 || return 1

    # clean
    sctl stop before.service fork.service after.service
}

# usage: test loop
function test02() {
    log_info "===== test02 ====="
    sed -i '/After=/ s/fork/after/' ${SYSMST_LIB_PATH}/after.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    # self-loop
    sctl start after.service
    expect_eq $? 0 || return 1
    check_status after.service activating || return 1
    expect_eq $? 0
    # clean
    sctl stop after.service

    # loop
    sed -i '/After=/ s/fork/before/' ${SYSMST_LIB_PATH}/after.service
    sed -i 's/Before=/After=/; /After=/ s/fork/after/' ${SYSMST_LIB_PATH}/before.service
    sctl daemon-reload
    sctl start after.service before.service
    expect_ne $? 0 || return 1
    check_log "${SYSMST_LOG}" 'asdaasd'
    expect_eq $? 0 || return 1
}

run_sysmaster || exit 1
test01 || exit 1
#test02 || exit 1
exit "${EXPECT_FAIL}"
