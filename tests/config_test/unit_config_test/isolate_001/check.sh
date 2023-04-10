#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test AllowIsolate/IgnoreOnIsolate
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/{base.service,reboot.target} ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    # default AllowIsolate: false
    sctl isolate reboot.target
    expect_eq $? 1
    check_log "${SYSMST_LOG}" 'asdasd'
    # clean
    kill_sysmaster

    # AllowIsolate=true
    # default IgnoreOnIsolate: false
    echo 'AllowIsolate=true' >> ${SYSMST_LIB_PATH}/reboot.target
    run_sysmaster || return 1

    sctl restart base
    check_status base active || return 1
    sctl isolate reboot.target
    expect_eq $? 0
    check_status reboot.target active || return 1
    check_status base inactive || return 1
    # clean
    kill_sysmaster

    # IgnoreOnIsolate=true
    sed -i 's/^Description=.*/IgnoreOnIsolate=true/' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl restart base
    check_status base active || return 1
    sctl isolate reboot.target
    expect_eq $? 0
    check_status reboot.target active || return 1
    check_status base active || return 1
    sctl stop base
    check_status base inactive || return 1
    # clean
    kill_sysmaster
}

test01 || exit 1
exit "${EXPECT_FAIL}"
