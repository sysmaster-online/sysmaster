#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e
key_log_1="ERROR sysmaster.*No ExecStart command"
key_log_2="ERROR sysmaster.* load unit .*base.service] failed: Confique  error"
key_log_3="ERROR sysmaster.* load unit .*base.service] failed: unit configuration error: 'More than Oneshot ExecStart command is configured, service type is not oneshot'"

# usage: test unit without ExecStart
function test01() {
    log_info "===== test01 ====="

    # no ExecStart
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/ExecStart/d' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    expect_eq $? 1
    check_log ${SYSMST_LOG} "${key_log_1}"
    # check_load base false
    # check_status base inactive
    sctl status base &> log
    check_log log 'base.service: NotExisted'
    rm -rf log
    # clean
    kill_sysmaster

    # null ExecStart
    sed -i "/Service]/a ExecStart=\"\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    expect_eq $? 1
    check_log ${SYSMST_LOG} "${key_log_2}"
    # check_load base false
    # check_status base inactive
    # clean
    kill_sysmaster
}


# usage: test multiple ExecStart
function test02() {
    log_info "===== test02 ====="

    # multiple commands in single ExecStart
    sed -i "s#ExecStart=.*#ExecStart=\"/bin/sleep 2; /bin/sleep 222\"#" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    expect_eq $? 1
    check_log ${SYSMST_LOG} "${key_log_3}"
    # check_load base false
    # check_status base inactive
    # clean
    kill_sysmaster

    # Type="oneshot": multiple commands in single ExecStart
    sed -i '/ExecStart/ i Type="oneshot"' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base &
    check_load base true
    check_status base activating
    main_pid_1="$(get_pids base)"
    ps -elf | grep -v grep | grep 'sleep 2$' | awk '{print $4}' | grep -w "${main_pid_1}"
    expect_eq $? 0 || ps -elf
    sleep 2
    main_pid_2="$(get_pids base)"
    ps -elf | grep -v grep | grep 'sleep 222$' | awk '{print $4}' | grep -w "${main_pid_2}"
    expect_eq $? 0 || ps -elf
    expect_gt "${main_pid_2}" "${main_pid_1}"
    # clean
    sctl stop base
    check_status base inactive
    kill_sysmaster

    # single command in multiple ExecStart
    sed -i "s#ExecStart=.*#ExecStart=\"/bin/sleep 99\"#" ${SYSMST_LIB_PATH}/base.service
    sed -i "/Service]/a ExecStart=\"/bin/sleep 100\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    expect_eq $? 1
    check_log ${SYSMST_LOG} "${key_log_2}"
    # check_load base false
    # check_status base inactive
    # clean
    kill_sysmaster
}

# usage: test invalid ExecStart
function test03() {
    log_info "===== test03 ====="

    # inexecutable
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i "s#ExecStart=\".*\"*#ExecStart=\"/inexec\"#" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    touch /inexec
    chmod 400 /inexec
    sctl start base
    expect_eq $? 0
    check_load base true
    check_status base failed
    # clean
    rm -rf /inexec
    kill_sysmaster

    # failed
    sed -i "s#ExecStart=\".*\"#ExecStart=\"/usr/bin/false\"#" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    expect_eq $? 0
    check_load base true
    check_status base failed
    # clean
    kill_sysmaster

    # failed but ignore
    sed -i "s#ExecStart=\".*\"#ExecStart=\"-/usr/bin/false\"#" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    expect_eq $? 0
    check_load base true
    check_status base inactive
    # clean
    kill_sysmaster
}

# usage: test ExecStartPre/ExecStartPost/ExecStop/ExecStopPost
function test04() {
    log_info "===== test04 ====="

    # exec success
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    sctl start exec
    check_status exec inactive
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_1 start_post_1 start_post_2 start_post_3 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill_sysmaster

    # ExecStartPre failed
    sed -i 's/echo echo_start_pre_2_echo/false/' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec failed
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill_sysmaster

    # ExecStartPre failed but ignore
    sed -i 's#/usr/bin/false#-/usr/bin/false#' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec inactive
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_3 start_1 start_post_1 start_post_2 start_post_3 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill_sysmaster

    # ExecStart failed
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/echo echo_start_1_echo/false/' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec failed
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill_sysmaster

    # ExecStart failed but ignore
    sed -i 's#/usr/bin/false#-/usr/bin/false#' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec failed
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill_sysmaster

    # ExecStartPost failed
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/echo echo_start_post_1_echo/false/' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec inactive
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_1 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill_sysmaster

    # ExecStartPost failed but ignore
    sed -i 's#/usr/bin/false#-/usr/bin/false#' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec inactive
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_1 start_post_2 start_post_3 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill_sysmaster

    # ExecStop failed
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/echo echo_start_1_echo/sleep 100/' ${SYSMST_LIB_PATH}/exec.service
    sed -i 's/echo echo_stop_1_echo/false/' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec active
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 ' || cat ${SYSMST_LOG}
    sctl stop exec
    check_status exec failed
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill_sysmaster

    # ExecStop failed but ignore
    sed -i 's#/usr/bin/false#-/usr/bin/false#' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec active
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 ' || cat ${SYSMST_LOG}
    sctl stop exec
    check_status exec inactive
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill_sysmaster

    # ExecStopPost failed
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/echo stop_post_1_echo/false/' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec inactive
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_1 start_post_1 start_post_2 start_post_3 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill_sysmaster
}

test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
exit "${EXPECT_FAIL}"
