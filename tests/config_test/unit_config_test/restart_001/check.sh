#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# test status when manually sctl stop: always inactive, no restart
function test_stop() {
    local service="${1:-restart_001}"
    local flag=0

    sctl restart "${service}"
    check_status "${service}" active
    expect_eq $? 0 || return 1
    sctl stop "${service}"
    check_status "${service}" inactive
    expect_eq $? 0 || flag=1
    sctl status "${service}"
    return "${flag}"
}

# test status when service finish successfully
function test_success(){
    local restart="${1:-yes}"
    local service="${2:-restart_001}"
    local flag=0
    local main_pid_1 main_pid_2 status

    if [ "${restart}" = yes ]; then
        status='active'
    elif [ "${restart}" = no ]; then
        status='inactive'
    else
        add_failure
        return 1
    fi

    sctl restart "${service}"
    check_status "${service}" active
    expect_eq $? 0 || return 1
    main_pid_1="$(get_pids "${service}")"
    expect_eq $? 0 || return 1
    sleep 1.5
    check_status "${service}" "${status}"
    expect_eq $? 0 || flag=1
    sctl status "${service}"

    if [ "${restart}" = yes ]; then
        main_pid_2="$(get_pids "${service}")"
        expect_eq $? 0 || flag=1
        expect_gt "${main_pid_2}" "${main_pid_1}" || flag=1
    fi

    sctl stop "${service}"
    [[ "${flag}" -ne 0 ]] && return 1
    return 0
}

# test status when service failed
function test_fail(){
    local restart="${1:-yes}"
    local service="${2:-restart_002}"
    local flag=0
    local status

    if [ "${restart}" = yes ]; then
        status='activating'
    elif [ "${restart}" = no ]; then
        status='failed'
    else
        add_failure
        return 1
    fi

    sctl restart "${service}" &
    check_status "${service}" activating
    expect_eq $? 0 || return 1
    check_status "${service}" failed
    expect_eq $? 0 || return 1
    # restart_002 has RestartSec=2
    sleep 2
    check_status "${service}" "${status}"
    expect_eq $? 0 || flag=1
    sctl status "${service}"

    # sctl reset-failed "${service}"
    sctl stop "${service}"
    [[ "${flag}" -ne 0 ]] && return 1
    return 0
}

# test status when service catch abnormal signal
function test_signal(){
    local sig="$1"
    local restart="${2:-yes}"
    local status="$3"
    local service="${4:-restart_001}"
    local flag=0
    local main_pid_1 main_pid_2

    [ "${restart}" = yes ] && status='active'

    sctl restart "${service}"
    check_status "${service}" 'active'
    expect_eq $? 0 || return 1
    main_pid_1="$(get_pids "${service}")"
    expect_eq $? 0 || return 1
    kill -"${sig}" "${main_pid}"
    check_status "${service}" "${status}"
    expect_eq $? 0 || flag=1
    sctl status "${service}"

    if [ "${restart}" = yes ]; then
        main_pid_2="$(get_pids "${service}")"
        expect_eq $? 0 || flag=1
        expect_gt "${main_pid_2}" "${main_pid_1}" || flag=1
    fi

    # sctl reset-failed "${service}"
    sctl stop "${service}"
    [[ "${flag}" -ne 0 ]] && return 1
    return 0
}

# test status when service timeout
function test_timeout(){
    local restart="${1:-yes}"
    local service="${2:-restart_003}"
    local flag=0
    local main_pid_1 main_pid_2 status

    if [ "${restart}" = yes ]; then
        status='activating'
    elif [ "${restart}" = no ]; then
        status='failed'
    else
        add_failure
        return 1
    fi

    sctl restart "${service}" &
    check_status "${service}" activating
    expect_eq $? 0 || return 1
    main_pid_1="$(get_pids "${service}")"
    expect_eq $? 0 || return 1
    sleep 2.5
    check_status "${service}" failed
    expect_eq $? 0 || return 1
    # restart_002 has RestartSec=2
    sleep 2
    check_status "${service}" "${status}"
    expect_eq $? 0 || flag=1
    sctl status "${service}"

    if [ "${restart}" = yes ]; then
        main_pid_2="$(get_pids "${service}")"
        expect_eq $? 0 || flag=1
        expect_gt "${main_pid_2}" "${main_pid_1}" || flag=1
    fi

    # sctl reset-failed "${service}"
    sctl stop "${service}"
    [[ "${flag}" -ne 0 ]] && return 1
    return 0
}

# usage: Restart=always
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/restart_00*.service ${SYSMST_LIB_PATH} || return 1
    for sev in ${SYSMST_LIB_PATH}/restart_00*.service; do
        sed -i 's/^Restart=.*/Restart=always/' "${sev}"
    done
    sctl daemon-reload
    # stop manually: no restart
    test_stop || return 1
    # finish successfully : restart
    test_success || return 1
    test_signal 'HUP' || return 1
    test_signal 'INT' || return 1
    test_signal 'TERM' || return 1
    # abnormal exit : restart
    test_fail || return 1
    # force kill : restart
    test_signal 'BUS' || return 1
    test_signal 'ABRT' || return 1
    test_signal 'KILL' || return 1
    # timeout : restart
    test_timeout || return 1
    # RestartPreventExitStatus prevent restart : no restart
    test_signal PIPE 'no' 'inactive' || return 1
    test_fail 'no' 'restart_004'  || return 1
    # clean abort core
    rm -rf "${CORE_DIR}"/*sleep*

    # clean
    sctl stop restart_001
    sctl stop restart_002
    sctl stop restart_003
    sctl stop restart_004
    check_status restart_001 inactive
    expect_eq $? 0
    check_status restart_002 inactive
    expect_eq $? 0
    check_status restart_003 inactive
    expect_eq $? 0
    check_status restart_004 inactive
    expect_eq $? 0
}

run_sysmaster || exit 1
test01 || exit 1
exit "${EXPECT_FAIL}"
