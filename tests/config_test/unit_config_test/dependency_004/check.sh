#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e
key_log_1='Started success1.service'
key_log_2='Started success2.service'

# usage: test OnSuccess
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH}/success1.service || return 1
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH}/success2.service || return 1
    sed -i '/Description/a OnSuccess=success1.service success2.service' ${SYSMST_LIB_PATH}/base.service
    sed -i 's/sleep.*"/sleep 1"/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl status success1
    expect_eq $? 2
    sctl status success2
    expect_eq $? 2
    sctl start base
    expect_eq $? 0 || return 1
    check_status base active
    expect_eq $? 0 || return 1
    check_status base inactive
    expect_eq $? 0 || return 1
    sctl status success1 success2
    expect_eq $? 0
    check_log "${SYSMST_LOG}" "${key_log_1}" "${key_log_2}"
    expect_eq $? 0

    # clean
    sctl stop success1 success2
    check_status success1 inactive
    expect_eq $? 0
    check_status success2 inactive
    expect_eq $? 0
    sctl status success1
    expect_eq $? 3
    sctl status success2
    expect_eq $? 3

    # failed
    sed -i 's/sleep.*"/false/' ${SYSMST_LIB_PATH}/base.service
    echo > "${SYSMST_LOG}"
    sctl daemon-reload
    sctl restart base
    expect_eq $? 0 || return 1
    check_status base failed
    expect_eq $? 0 || return 1
    sctl status success1
    expect_eq $? 3
    sctl status success2
    expect_eq $? 3
    grep -aE "${key_log_1}|${key_log_2}" "${SYSMST_LOG}"
    expect_eq $? 1
}

run_sysmaster || exit 1
test01 || exit 1
exit "${EXPECT_FAIL}"
