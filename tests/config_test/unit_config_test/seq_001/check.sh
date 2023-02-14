#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test After
function test01() {
    cp -arf "${work_dir}"/tmp_units/{fork.service,after.service} ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    sctl start fork.service after.service &
    sleep 3
    check_log "${SYSMST_LOG}" 'asdaasd' || return 1
    grep 'asdasda' "${SYSMST_LOG}"
    expect_eq $? 1 || return 1
    sleep 3
    line1="$(grep -n "fork asdasd" "${SYSMST_LOG}" | awk -F: '{print $1}')"
    line2="$(grep -n "after asdasd" "${SYSMST_LOG}" | awk -F: '{print $1}')"
    expect_lt "${line1}" "${line2}" || return 1

    # clean
    sctl stop fork.service after.service
    kill -9 "${sysmaster_pid}"
}

# usage: test Before
function test02() {
    cp -arf "${work_dir}"/tmp_units/before.service ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    sctl start fork.service before.service &
    sleep 3
    check_log "${SYSMST_LOG}" 'asdaasd' || return 1
    grep 'asdasda' "${SYSMST_LOG}"
    expect_eq $? 1 || return 1
    sleep 3
    line1="$(grep -n "before asdasd" "${SYSMST_LOG}" | awk -F: '{print $1}')"
    line2="$(grep -n "fork asdasd" "${SYSMST_LOG}" | awk -F: '{print $1}')"
    expect_lt "${line1}" "${line2}" || return 1

    # clean
    sctl stop fork.service before.service
    kill -9 "${sysmaster_pid}"
}

# usage: test loop
function test03() {
    sed -i '/After=/ s/fork/after/' ${SYSMST_LIB_PATH}/after.service
    run_sysmaster || return 1

    sctl restart after.service
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'asdaasd' || return 1

    # clean
    sctl stop after.service
    kill -9 "${sysmaster_pid}"

    sed -i '/After=/ s/after/before/' ${SYSMST_LIB_PATH}/after.service
    sed -i '/Before=/ s/fork/after/' ${SYSMST_LIB_PATH}/before.service
    run_sysmaster || return 1

    sctl restart after.service before.service
    expect_ne $? 0 || return 1
    check_log "${SYSMST_LOG}" 'asdaasd' || return 1

    # clean
    kill -9 "${sysmaster_pid}"
}

test01 || exit 1
test02 || exit 1
test03 || exit 1
