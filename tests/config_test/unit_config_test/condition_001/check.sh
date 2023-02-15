#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test ConditionPathExists/AssertPathExists
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    if [ "${condition_test}" -eq 1 ]; then
        sed -i "/Description=/ a ConditionPathExists=\"/tmp/path_exist\"" ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        sed -i "/Description=/ a AssertPathExists=\"/tmp/path_exist\"" ${SYSMST_LIB_PATH}/base.service
    fi
    rm -rf /tmp/path_exist
    run_sysmaster || return 1

    # path not exist
    sctl start base.service
    check_status base.service inactive || return 1
    grep 'asdasda' "${SYSMST_LOG}"
    expect_eq $? "${condition_test}" || return 1

    # file path
    touch /tmp/path_exist
    sctl stop base.service
    sctl start base.service
    check_status base.service active || return 1

    # dir path
    sctl stop base.service
    rm -rf /tmp/path_exist
    mkdir /tmp/path_exist
    sctl start base.service
    check_status base.service active || return 1

    # clean
    sctl stop base.service
    rm -rf /tmp/path_exist
    kill -9 "${sysmaster_pid}"
}

test01 || exit 1
exit "${EXPECT_FAIL}"
