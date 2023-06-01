#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test AllowIsolate/IgnoreOnIsolate
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/{base.service,reboot.target} ${SYSMST_LIB_PATH} || return 1
    sctl daemon-reload
    # default AllowIsolate: false
    sctl isolate reboot.target
    expect_eq $? 1
    check_log "${SYSMST_LOG}" 'asdasd'
    expect_eq $? 0

    # AllowIsolate=true
    # default IgnoreOnIsolate: false
    echo 'AllowIsolate=true' >> ${SYSMST_LIB_PATH}/reboot.target
    sctl daemon-reload
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    sctl isolate reboot.target
    expect_eq $? 0
    check_status reboot.target active
    expect_eq $? 0 || return 1
    check_status base inactive
    expect_eq $? 0 || return 1

    # IgnoreOnIsolate=true
    sed -i 's/^Description=.*/IgnoreOnIsolate=true/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    sctl isolate reboot.target
    expect_eq $? 0
    check_status reboot.target active
    expect_eq $? 0 || return 1
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1
}

run_sysmaster || exit 1
test01 || exit 1
exit "${EXPECT_FAIL}"
