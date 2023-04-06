#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test forking
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/fork.service ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    sctl restart fork
    check_status fork activating || return 1
    main_pid="$(get_pids fork)"
    ps -elf | grep -v grep | grep -w sleep | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 0 || ps -elf
    sctl stop fork
    check_status fork inactive || return 1

    # clean
    kill_sysmaster
}

# usage: test PIDFile
function test02() {
    local sec=123456

    # PIDFile=pidfile
    sed -i '/ExecStart/a PIDFile="pidfile"' ${SYSMST_LIB_PATH}/fork.service
    sed -i "s#^ExecStart=.*#ExecStart=\"/opt/fork_exec ${sec} /run/pidfile\"#" ${SYSMST_LIB_PATH}/fork.service
    run_sysmaster || return 1

    sctl restart fork
    check_status fork active || return 1
    main_pid="$(get_pids fork)"
    ps -elf | grep -v grep | grep -w fork_exec | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 0 || ps -elf
    expect_str_eq "$(cat /run/pidfile)" "${main_pid}"
    sctl stop fork
    check_status fork inactive || return 1
    ls /run/pidfile
    expect_eq $? 2
    # clean
    kill_sysmaster

    # PIDFile=/tmp/pidfile
    sed -i '/PIDFile/ s#pidfile#/tmp/pidfile#' ${SYSMST_LIB_PATH}/fork.service
    sed -i '/fork_exec/ s/run/tmp/' ${SYSMST_LIB_PATH}/fork.service
    run_sysmaster || return 1

    sctl restart fork
    check_status fork active || return 1
    main_pid="$(get_pids fork)"
    ps -elf | grep -v grep | grep -w fork_exec | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 0 || ps -elf
    expect_str_eq "$(cat /tmp/pidfile)" "${main_pid}"
    sctl stop fork
    check_status fork inactive || return 1
    ls /tmp/pidfile
    expect_eq $? 2
    # clean
    kill_sysmaster
}

test01 || exit 1
test02 || exit 1
exit "${EXPECT_FAIL}"
