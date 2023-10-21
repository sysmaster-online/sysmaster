#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test Description/Documentation/RemainAfterExit
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1

    # RemainAfterExit=false
    sed -i 's/^Description=.*/Description=this is a test/' ${SYSMST_LIB_PATH}/base.service
    sed -i '/Description/ a Documentation=this is doc' ${SYSMST_LIB_PATH}/base.service
    sed -i '/ExecStart/ a RemainAfterExit=false' ${SYSMST_LIB_PATH}/base.service
    sed -i 's/sleep 100/sleep 2/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    check_status base inactive
    expect_eq $? 0 || return 1
    # check Description/Documentation
    sctl status base 2>&1 | grep "base.service - this is a test" && sctl status base 2>&1 | grep "Docs: this is doc"
    expect_eq $? 0 || sctl status base

    # RemainAfterExit=true
    sed -i '/RemainAfterExit/ s/false/true/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base 'active (running)'
    expect_eq $? 0 || return 1
    main_pid="$(get_pids base)"
    sleep 2
    check_status base 'active (exited)'
    expect_eq $? 0 || return 1
    ps -elf | grep -v grep | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 1 || ps -elf

    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1
}

# usage: test RemainAfterExit with oneshot service
function test02() {
    log_info "===== test02 ====="
    sed -i '/ExecStart/ a Type=oneshot' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base &
    check_status base activating
    expect_eq $? 0 || return 1
    main_pid="$(get_pids base)"
    sleep 2
    check_status base active
    expect_eq $? 0 || return 1
    sctl status base 2>&1 | grep active | grep 'exited'
    expect_eq $? 0 || sctl status base
    ps -elf | grep -v grep | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 1 || ps -elf
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1
}

# usage: test DefaultDependencies
function test03() {
    log_info "===== test03 ====="

    # default true
    rm -rf ${SYSMST_LIB_PATH}/*.target
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sctl daemon-reload
    sctl restart base
    sleep 1
    cp ${BUILD_PATH}/units/*.target ${SYSMST_LIB_PATH}
    check_status base inactive
    expect_eq $? 0 || return 1

    # DefaultDependencies=false
    sed -i '/Description/ a DefaultDependencies=false' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1
}

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
test03 || exit 1
exit "${EXPECT_FAIL}"
