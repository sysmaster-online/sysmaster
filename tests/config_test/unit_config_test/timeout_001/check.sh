#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

timeout_log='Job Start of unit base.service timeout'

# usage: test JobTimeoutSec: job running timeout
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Description/ a JobTimeoutSec=0' ${SYSMST_LIB_PATH}/base.service
    sed -i '/ExecStart/ i Type="oneshot"' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    # JobTimeoutSec=0 means infinity
    sctl start base
    sleep 3
    main_pid="$(get_pids base)"
    check_status base activating
    expect_eq $? 0 || return 1
    grep -a "${timeout_log}" "${SYSMST_LOG}"
    expect_eq $? 1 || cat "${SYSMST_LOG}"
    ps -elf | grep -v grep | grep -w 'sleep' | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 0 || ps -elf

    # clean
    sctl stop base
    check_status base inactive
    expect_eq $? 0

    # JobTimeoutSec=3 means 3 sec
    # timeout: remain status when timeout, main process still exists
    sed -i 's/JobTimeoutSec=.*/JobTimeoutSec=3/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    sctl start base
    sleep 2.5
    main_pid="$(get_pids base)"
    check_status base activating
    expect_eq $? 0 || return 1
    sleep 1
    check_status base activating
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" "${timeout_log}"
    expect_eq $? 0 || return 1
    ps -elf | grep -v grep | grep -w 'sleep' | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 0 || ps -elf

    # clean
    sctl stop base
    check_status base inactive
    expect_eq $? 0

    # no timeout
    sed -i 's/sleep 100/sleep 2/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    sctl start base
    sleep 1.5
    main_pid="$(get_pids base)"
    check_status base activating
    expect_eq $? 0 || return 1
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1
    grep -a "${timeout_log}" "${SYSMST_LOG}"
    expect_eq $? 1 || cat "${SYSMST_LOG}"
    ps -elf | grep -v grep | grep -w 'sleep' | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 1 || ps -elf
}

# usage: test JobTimeoutSec: job starting timeout
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Description/ a JobTimeoutSec=0' ${SYSMST_LIB_PATH}/base.service
    sed -i 's/sleep 100/sleep 2/' ${SYSMST_LIB_PATH}/base.service
    sed -i '/ExecStart/ a ExecStartPre="/usr/bin/sleep 2"' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    # JobTimeoutSec=0 means infinity
    sctl start base
    sleep 1
    pre_pid="$(get_pids base)"
    check_status base activating
    expect_eq $? 0
    sleep 1.5
    main_pid="$(get_pids base)"
    check_status base active || return 1
    expect_eq $? 0
    sleep 2
    check_status base inactive || return 1
    expect_eq $? 0
    grep -a "${timeout_log}" "${SYSMST_LOG}"
    expect_eq $? 1 || cat "${SYSMST_LOG}"
    expect_gt "${main_pid}" "${pre_pid}"
    ps -elf | grep -v grep | grep -w 'sleep' | grep -Ew "${pre_pid}|${main_pid}"
    expect_eq $? 1 || ps -elf

    # JobTimeoutSec=3 means 3 sec
    # no timeout: pre start < job timeout sec < pre start + main start
    sed -i 's/JobTimeoutSec=.*/JobTimeoutSec=3/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    sctl start base
    sleep 1
    pre_pid="$(get_pids base)"
    check_status base activating
    expect_eq $? 0
    sleep 1.5
    main_pid="$(get_pids base)"
    check_status base active
    expect_eq $? 0 || return 1
    sleep 2
    check_status base inactive
    expect_eq $? 0 || return 1
    grep -a "${timeout_log}" "${SYSMST_LOG}"
    expect_eq $? 1 || cat "${SYSMST_LOG}"
    expect_gt "${main_pid}" "${pre_pid}"
    ps -elf | grep -v grep | grep -w 'sleep' | grep -Ew "${pre_pid}|${main_pid}"
    expect_eq $? 1 || ps -elf

    # JobTimeoutSec=1 means 1 sec
    # timeout: pre start > job timeout sec
    # remain status when timeout, main process still exists
    sed -i 's/JobTimeoutSec=.*/JobTimeoutSec=1/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    sctl start base
    sleep 0.5
    pre_pid_1="$(get_pids base)"
    check_status base activating
    expect_eq $? 0 || return 1
    sleep 1
    pre_pid_2="$(get_pids base)"
    check_status base activating
    expect_eq $? 0 || return 1
    expect_eq "${pre_pid_1}" "${pre_pid_2}"
    ps -elf | grep -v grep | grep -w 'sleep' | grep -w "${pre_pid_1}"
    expect_eq $? 0 || ps -elf
    check_log "${SYSMST_LOG}" "${timeout_log}"
    expect_eq $? 0 || return 1

    # clean
    sctl stop base
    check_status base inactive
    expect_eq $? 0
}

# usage: test JobTimeoutAction
function test03() {
    log_info "===== test03 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Description/ a JobTimeoutSec=1' ${SYSMST_LIB_PATH}/base.service
    sed -i '/Description/ a JobTimeoutAction=' ${SYSMST_LIB_PATH}/base.service
    sed -i '/ExecStart/ a ExecStartPre=/usr/bin/sleep 2' ${SYSMST_LIB_PATH}/base.service

    # service success: reboot, poweroff, exit
    local cmd_list="reboot poweroff exit"
    for cmd in ${cmd_list}; do
        cp -arf "${work_dir}"/tmp_units/"${cmd}".target ${SYSMST_LIB_PATH} || return 1
        sed -i "s/JobTimeoutAction=.*/JobTimeoutAction=${cmd}/" ${SYSMST_LIB_PATH}/base.service
        sctl daemon-reload
        sleep 2
        echo > "${SYSMST_LOG}"
        sctl start base
        sleep 0.5
        pre_pid="$(get_pids base)"
        check_status base activating
        expect_eq $? 0 || return 1
        sleep 1
        check_status base activating
        expect_eq $? 0 || return 1
        ps -elf | grep -v grep | grep -w 'sleep' | grep -w "${pre_pid}"
        expect_eq $? 0 || ps -elf
        ps aux | grep -v grep | awk '{print $2}' | grep -w "${sysmaster_pid}"
        expect_eq $? 0 || ps -elf
        check_log "${SYSMST_LOG}" "by starting .*target caused by the job of unit base.service timedout"
        expect_eq $? 0 || return 1

        # clean
        sctl stop base
        check_status base inactive
        expect_eq $? 0
    done
}

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
test03 || exit 1
exit "${EXPECT_FAIL}"
