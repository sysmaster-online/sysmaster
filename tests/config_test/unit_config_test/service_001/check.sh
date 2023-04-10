#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test Description/Documentation/RemainAfterExit
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1

    # RemainAfterExit=false
    sed -i 's/^Description=.*/Description="this is a test"/' ${SYSMST_LIB_PATH}/base.service
    sed -i '/Description/ a Documentation="this is doc"' ${SYSMST_LIB_PATH}/base.service
    sed -i '/ExecStart/ a RemainAfterExit=false' ${SYSMST_LIB_PATH}/base.service
    sed -i 's/sleep 100/sleep 2/' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl restart base
    check_status base active || return 1
    check_status base inactive || return 1
    # check Description/Documentation
    sctl status base | grep "base.service - this is a test" && sctl status base | grep "Docs: this is doc"
    expect_eq $? 0 || sctl status base
    # clean
    kill_sysmaster

    # RemainAfterExit=true
    sed -i '/RemainAfterExit/ s/false/true/' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl restart base
    check_status base active || return 1
    sctl status base | grep active | grep 'running'
    expect_eq $? 0 || sctl status base
    main_pid="$(get_pids base)"
    sleep 2
    check_status base active || return 1
    sctl status base | grep active | grep 'exited'
    expect_eq $? 0 || sctl status base
    ps -elf | grep -v grep | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 1 || ps -elf

    sctl stop base
    check_status base inactive || return 1
    # clean
    kill_sysmaster
}

# usage: test RemainAfterExit with oneshot service
function test02() {
    log_info "===== test02 ====="
    sed -i '/ExecStart/ a Type="oneshot"' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl restart base &
    check_status base activating || return 1
    main_pid="$(get_pids base)"
    sleep 2
    check_status base active || return 1
    sctl status base | grep active | grep 'exited'
    expect_eq $? 0 || sctl status base
    ps -elf | grep -v grep | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 1 || ps -elf

    sctl stop base
    check_status base inactive || return 1
    # clean
    kill_sysmaster
}

# usage: test DefaultDependencies
function test03() {
    local key_log='add default dependencies for target.*'

    log_info "===== test03 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    # default true
    sctl restart base
    check_status base active || return 1
    check_log "${SYSMST_LOG}" "${key_log}sysinit.target" "${key_log}shutdown.target" 'ERROR .* basic.target is not exist'
    sctl stop base
    check_status base inactive || return 1
    # clean
    kill_sysmaster

    # DefaultDependencies=false
    sed -i '/Description/ a DefaultDependencies=false' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl restart base
    check_status base active || return 1
    grep -aE "${key_log}|basic.target|sysinit.target|shutdown.target" "${SYSMST_LOG}"
    expect_eq $? 1 || cat "${SYSMST_LOG}"
    sctl stop base
    check_status base inactive || return 1
    # clean
    kill_sysmaster
}

test01 || exit 1
test02 || exit 1
test03 || exit 1
exit "${EXPECT_FAIL}"
