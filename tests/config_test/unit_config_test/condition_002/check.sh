#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test ConditionFileNotEmpty
function test01() {
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i "/Description=/ a ConditionFileNotEmpty=\"/tmp\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    # path is directory
    sctl restart base.service
    check_status base inactive
    expect_eq $? 0 || return 1
    grep 'asdasda' "${SYSMST_LOG}"
    expect_eq $? "${condition_test}" || return 1

    # clean
    kill -9 "${sysmaster_pid}"

    rm -rf /tmp/file_not_empty
    sed -i '/ConditionFileNotEmpty=/ s#/tmp#/tmp/file_not_empty#' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    # path not exist
    sctl restart base.service
    check_status base inactive
    expect_eq $? 0 || return 1
    grep 'asdasda' "${SYSMST_LOG}"
    expect_eq $? "${condition_test}" || return 1

    # path is an empty file
    touch /tmp/file_not_empty
    sctl restart base.service
    check_status base inactive
    expect_eq $? 0 || return 1
    grep 'asdasda' "${SYSMST_LOG}"
    expect_eq $? "${condition_test}" || return 1

    # valid file path
    echo 1 > /tmp/file_not_empty
    sctl restart base.service
    check_status base active
    expect_eq $? 0 || return 1

    # clean
    sctl stop base.service
    rm -rf /tmp/file_not_empty
    kill -9 "${sysmaster_pid}"
}

test01 || exit 1
