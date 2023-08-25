#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

MAX=10
key_log='INFO  sysmaster sysmaster running in system mode'
# support signals:
# SIGQUIT SIGILL SIGABRT SIGBUS SIGFPE SIGSEGV SIGSYS
signals=(3 4 6 7 8 11 31)
signal_num="$(echo ${signals[*]} | wc -w)"

## usage: background unit operation
function stress() {
    while ((1)); do
        sleep $((RANDOM % 3 + 1))
        date
        sctl start reliable_002.service

        sleep $((RANDOM % 3 + 1))
        date
        sctl stop reliable_002.service
    done
}

## usage: check unit function
function check_fun() {
    local main_pid

    sctl start reliable_001.service
    expect_eq $? 0 || return 1
    check_status reliable_001.service active || return 1
    sleep 3
    main_pid="$(get_pids reliable_001.service)"
    expect_eq "${main_pid}" "$(ps -elf | grep -v grep | grep 'sleep 888' | awk '{print $4}')" || return 1

    sctl stop reliable_001.service
    expect_eq $? 0 || return 1
    check_status reliable_001.service inactive || return 1
    sleep 3
    sctl status reliable_001.service 2>&1 | grep 'PID:  No process'
    expect_eq $? 0 || return 1
    ps -elf | grep -v grep | grep 'sleep 888'
    expect_eq $? 1 || return 1

    return 0
}

## usage: background random kill
function random_kill() {
    local signal_index="$((RANDOM % signal_num))"

    echo > /opt/sysmaster.log

    # random kill with random signal
    eval kill -\${signals[${signal_index}]} "${sysmaster_pid}"

    # check sysmaster
    sleep 3
    ps -elf | grep -v grep | grep -w sysmaster
    expect_eq "$(ps aux | grep -v grep | grep -w sysmaster | wc -l)" 1 'check the number of sysmaster process failed!'
    expect_eq "$(ps -elf | grep -v grep | grep -w sysmaster | awk '{print $4}')" "${sysmaster_pid}" 'sysmaster pid changed!'

    # check log
    check_log "${SYSMST_LOG}" "${key_log}"

    # wait sysmaster recover done and check function
    sleep 3
    check_fun

    return "${EXPECT_FAIL}"
}

## usage: clean process
function clean() {
    local pid

    if ps -elf | grep -v grep | grep -w sysmaster; then
        kill -9 "$(ps -elf | grep -v grep | grep -w sysmaster | awk '{print $4}')"
    fi
    [ -n "${stress_pid}" ] && kill -9 "${stress_pid}"
}

cp -arf "${work_dir}"/tmp_units/*.service ${SYSMST_LIB_PATH} || exit 1
mkdir -p "${RELIAB_SWITCH_PATH}"
touch "${RELIAB_SWITCH_PATH}"/"${RELIAB_SWITCH}" "${RELIAB_SWITCH_PATH}"/"${RELIAB_CLR}"
run_sysmaster || exit 1

check_fun
stress &> /opt/stress.log &
stress_pid=$!

for ((i = 0; i < ${MAX}; ++i)); do
    sleep "$((RANDOM % 5 + 1))"
    random_kill && continue
    clean
    exit 1
done

clean
[ "${EXPECT_FAIL}" -eq 0 ] || cat /opt/stress.log
rm -rf /opt/stress.log
exit "${EXPECT_FAIL}"
