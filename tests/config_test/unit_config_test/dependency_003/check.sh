#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test OnFailure
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH}/failure1.service || return 1
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH}/failure2.service || return 1
    sed -i '/Description/a OnFailure="failure1.service;failure2.service"' ${SYSMST_LIB_PATH}/base.service
    sed -i 's/sleep.*"/false"/' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl status failure1
    expect_eq $? 3
    sctl status failure2
    expect_eq $? 3
    sctl start base
    expect_eq $? 0 || return 1
    check_status base failed || return 1
    sctl status failure1 failure2
    expect_eq $? 0
    check_log "${SYSMST_LOG}" 'asdasda' 'safasfa'

    # clean
    sctl stop failure1 failure2
    check_status failure1 inactive
    check_status failure2 inactive
    kill_sysmaster

    # unit not exist
    rm -rf ${SYSMST_LIB_PATH}/failure2.service
    run_sysmaster || return 1

    sctl start base
    expect_eq $? 0 || return 1
    check_status base failed || return 1
    check_status failure1 active || return 1
    sctl status failure2
    expect_eq $? 4
    check_log "${SYSMST_LOG}" 'asdasda' 'asdafafaf'

    # clean
    sctl stop failure1
    kill_sysmaster
}

# usage: test OnFailureJobMode
function test02() {
    log_info "===== test02 ====="
    run_sysmaster || return 1

}

test01 || exit 1
test02 || exit 1
exit "${EXPECT_FAIL}"
