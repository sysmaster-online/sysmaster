#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e
sec=100

# usage: test default KillMode=control-group
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/fork.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/ExecStart/a PIDFile=/run/pidfile' ${SYSMST_LIB_PATH}/fork.service
    sed -i '/ExecStart/a ExecStop=/usr/bin/echo ThisIsExecStop' ${SYSMST_LIB_PATH}/fork.service
    sed -i "s#^ExecStart=.*#ExecStart=/opt/kill_mode ${sec} /run/pidfile#" ${SYSMST_LIB_PATH}/fork.service
    sctl daemon-reload
    sctl restart fork
    check_status fork active
    expect_eq $? 0 || return 1
    sctl status fork
    ps -elf | grep -v grep | grep kill_mode
    main_pid="$(get_pids fork)"
    child_pid="$(ps -elf | grep -v grep | grep kill_mode | grep -vw "${main_pid}" | awk '{print $4}')"
    expect_eq $? 0 || ps -elf

    # default KillMode=control-group
    # all remaining processes in the control group will be killed on unit stop
    # for services: after ExecStop cmd is executed
    echo > "${SYSMST_LOG}"
    sctl stop fork
    check_status fork inactive
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" "ThisIsExecStop$"
    expect_eq $? 0
    grep -a 'SIGHUP' "${SYSMST_LOG}"
    expect_eq $? 1
    ps -elf | grep -v grep | grep -E "${main_pid}|${child_pid}"
    expect_eq $? 1 || ps -elf

    # clean
    #kill -9 "${main_pid}" "${child_pid}"
}

# usage: test KillMode=process
function test02() {
    log_info "===== test02 ====="
    sed -i '/ExecStart/a KillMode=process' ${SYSMST_LIB_PATH}/fork.service
    sctl daemon-reload
    sctl restart fork
    check_status fork active
    expect_eq $? 0 || return 1
    sctl status fork
    ps -elf | grep -v grep | grep kill_mode
    main_pid="$(get_pids fork)"
    child_pid="$(ps -elf | grep -v grep | grep kill_mode | grep -vw "${main_pid}" | awk '{print $4}')"
    expect_eq $? 0 || ps -elf

    # KillMode=process
    # only main process will be killed on unit stop
    echo > "${SYSMST_LOG}"
    sctl stop fork
    check_status fork inactive
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" "ThisIsExecStop$"
    expect_eq $? 0
    grep -a 'SIGHUP' "${SYSMST_LOG}"
    expect_eq $? 1
    ps -elf | grep -v grep | grep -w "${main_pid}"
    expect_eq $? 1 || ps -elf
    ps -elf | grep -v grep | grep -w "${child_pid}"
    expect_eq $? 0 || ps -elf

    # clean
    kill -9 "${main_pid}" "${child_pid}"
}

# usage: test KillMode=none
function test03() {
    log_info "===== test03 ====="
    sed -i '/KillMode/ s/process/none/' ${SYSMST_LIB_PATH}/fork.service
    sctl daemon-reload
    sctl restart fork
    check_status fork active
    expect_eq $? 0 || return 1
    sctl status fork
    ps -elf | grep -v grep | grep kill_mode
    main_pid="$(get_pids fork)"
    child_pid="$(ps -elf | grep -v grep | grep kill_mode | grep -vw "${main_pid}" | awk '{print $4}')"
    expect_eq $? 0 || ps -elf

    # KillMode=none
    # no process will be killed on unit stop
    # only ExecStop cmd is executed
    echo > "${SYSMST_LOG}"
    sctl stop fork
    check_status fork inactive
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" "ThisIsExecStop$"
    expect_eq $? 0
    grep -a 'SIGHUP' "${SYSMST_LOG}"
    expect_eq $? 1
    ps -elf | grep -v grep | grep -w "${main_pid}" && ps -elf | grep -v grep | grep -w "${child_pid}"
    expect_eq $? 0 || ps -elf

    # clean
    kill -9 "${main_pid}" "${child_pid}"
}

# usage: test KillMode=mixed
function test04() {
    log_info "===== test04 ====="
    sed -i '/KillMode/ s/process/mixed/' ${SYSMST_LIB_PATH}/fork.service
    sctl daemon-reload
    sctl restart fork
    check_status fork active
    expect_eq $? 0 || return 1
    sctl status fork
    ps -elf | grep -v grep | grep kill_mode
    main_pid="$(get_pids fork)"
    child_pid="$(ps -elf | grep -v grep | grep kill_mode | grep -vw "${main_pid}" | awk '{print $4}')"
    expect_eq $? 0 || ps -elf

    # KillMode=mixed
    # main process will be killed by KillSignal(default: SIGTERM) on unit stop
    # all remaining process in unit cgroup will be killed by subsequent SIGKILL
    echo > "${SYSMST_LOG}"
    sctl stop fork
    check_status fork inactive
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" "ThisIsExecStop$"
    expect_eq $? 0
    grep -a 'SIGHUP' "${SYSMST_LOG}"
    expect_eq $? 1
    ps -elf | grep -v grep | grep -E "${main_pid}|${child_pid}"
    expect_eq $? 1 || ps -elf

    # clean
    #kill -9 "${main_pid}" "${child_pid}"
}

# usage: test KillSignal=SIGKILL
function test05() {
    log_info "===== test05 ====="
    sed -i '/KillMode/a KillSignal=SIGKILL' ${SYSMST_LIB_PATH}/fork.service
    sctl daemon-reload
    sctl restart fork
    check_status fork active
    expect_eq $? 0 || return 1
    sctl status fork
    ps -elf | grep -v grep | grep kill_mode
    main_pid="$(get_pids fork)"
    child_pid="$(ps -elf | grep -v grep | grep kill_mode | grep -vw "${main_pid}" | awk '{print $4}')"
    expect_eq $? 0 || ps -elf

    # KillMode=mixed
    # main process will be killed by SIGKILL on unit stop
    # all remaining process in unit cgroup will be killed by subsequent SIGKILL
    echo > "${SYSMST_LOG}"
    sctl stop fork
    check_status fork failed
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" "ThisIsExecStop$"
    expect_eq $? 0
    grep -a 'SIGHUP' "${SYSMST_LOG}"
    expect_eq $? 1
    ps -elf | grep -v grep | grep -E "${main_pid}|${child_pid}"
    expect_eq $? 1 || ps -elf

    # clean
    #kill -9 "${main_pid}" "${child_pid}"
}

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
# KillMode=none not implemented
# test03 || exit 1
test04 || exit 1
test05 || exit 1
exit "${EXPECT_FAIL}"
