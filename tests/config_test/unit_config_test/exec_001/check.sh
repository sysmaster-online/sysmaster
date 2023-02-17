#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e
key_log_1="ERROR sysmaster.*base.service] failed:.* Unable to determine the current process pid: No ExecStart command is configured and RemainAfterExit if false"
key_log_2="ERROR sysmaster.*base.service] failed:.* Unable to determine the current process pid: More than Oneshot ExecStart command is configured, service type is not oneshot"
key_log_3="ERROR sysmaster.*base.service] failed:.* failed to deserialize configuration from file"

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
    check_load base false
    check_status base inactive
    # clean
    kill -9 "${sysmaster_pid}"

    # null ExecStart
    sed -i "/Service]/a ExecStart=\"\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    expect_eq $? 1
    check_log ${SYSMST_LOG} "${key_log_1}"
    check_load base false
    check_status base inactive
    # clean
    kill -9 "${sysmaster_pid}"
}


# usage: test multiple ExecStart
function test02() {
    log_info "===== test02 ====="

    # multiple commands in single ExecStart
    sed -i "s#ExecStart=.*#ExecStart=\"/bin/sleep 100; /bin/sleep 100\"#" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    expect_eq $? 1
    check_log ${SYSMST_LOG} "${key_log_2}"
    check_load base false
    check_status base inactive
    # clean
    kill -9 "${sysmaster_pid}"

    # single command in multiple ExecStart
    sed -i "s#ExecStart=.*#ExecStart=\"/bin/sleep 99\"#" ${SYSMST_LIB_PATH}/base.service
    sed -i "/Service]/a ExecStart=\"/bin/sleep 100\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    expect_eq $? 1
    check_log ${SYSMST_LOG} "${key_log_3}"
    check_load base false
    check_status base inactive
    # clean
    kill -9 "${sysmaster_pid}"
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
    kill -9 "${sysmaster_pid}"

    # failed
    sed -i "s#ExecStart=\".*\"#ExecStart=\"/usr/bin/false\"#" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    expect_eq $? 0
    check_load base true
    check_status base failed
    # clean
    kill -9 "${sysmaster_pid}"

    # failed but ignore
    sed -i "s#ExecStart=\".*\"#ExecStart=\"-/usr/bin/false\"#" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    expect_eq $? 0
    check_load base true
    check_status base inactive
    # clean
    kill -9 "${sysmaster_pid}"
}

# usage: test ExecStartPre/ExecStartPost/ExecStop/ExecStopPost
function test04() {
    log_info "===== test04 ====="

    # exec success
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    sctl start exec
    check_status exec inactive
    expect_str_eq "$(cat ${${SYSMST_LOG}} | sed "s/\x00//g" | grep -a '^echo_' | sed 's/echo_//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_1 start_post_1 start_post_2 start_post_3 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill -9 "${sysmaster_pid}"

    # ExecStartPre failed
    sed -i 's/echo echo_start_pre_2/false/' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec failed
    expect_str_eq "$(cat ${${SYSMST_LOG}} | sed "s/\x00//g" | grep -a '^echo_' | sed 's/echo_//g' | tr '\n' ' ')" \
        'start_pre_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill -9 "${sysmaster_pid}"

    # ExecStartPre failed but ignore
    sed -i 's#/usr/bin/false#-/usr/bin/false#' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec inactive
    expect_str_eq "$(cat ${${SYSMST_LOG}} | sed "s/\x00//g" | grep -a '^echo_' | sed 's/echo_//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_3 start_1 start_post_1 start_post_2 start_post_3 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill -9 "${sysmaster_pid}"

    # ExecStart failed
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/echo echo_start_1/false/' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec failed
    expect_str_eq "$(cat ${${SYSMST_LOG}} | sed "s/\x00//g" | grep -a '^echo_' | sed 's/echo_//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill -9 "${sysmaster_pid}"

    # ExecStart failed but ignore
    sed -i 's#/usr/bin/false#-/usr/bin/false#' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec failed
    expect_str_eq "$(cat ${${SYSMST_LOG}} | sed "s/\x00//g" | grep -a '^echo_' | sed 's/echo_//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill -9 "${sysmaster_pid}"

    # ExecStartPost failed
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/echo echo_start_post_1/false/' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec inactive
    expect_str_eq "$(cat ${${SYSMST_LOG}} | sed "s/\x00//g" | grep -a '^echo_' | sed 's/echo_//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_1 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill -9 "${sysmaster_pid}"

    # ExecStartPost failed but ignore
    sed -i 's#/usr/bin/false#-/usr/bin/false#' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec inactive
    expect_str_eq "$(cat ${${SYSMST_LOG}} | sed "s/\x00//g" | grep -a '^echo_' | sed 's/echo_//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_1 start_post_2 start_post_3 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill -9 "${sysmaster_pid}"

    # ExecStop failed
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/echo echo_start_1/sleep 100/' ${SYSMST_LIB_PATH}/exec.service
    sed -i 's/echo echo_stop_1/false/' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec active
    expect_str_eq "$(cat ${${SYSMST_LOG}} | sed "s/\x00//g" | grep -a '^echo_' | sed 's/echo_//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 ' || cat ${SYSMST_LOG}
    sctl stop exec
    check_status exec failed
    expect_str_eq "$(cat ${${SYSMST_LOG}} | sed "s/\x00//g" | grep -a '^echo_' | sed 's/echo_//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill -9 "${sysmaster_pid}"

    # ExecStop failed but ignore
    sed -i 's#/usr/bin/false#-/usr/bin/false#' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec active
    expect_str_eq "$(cat ${${SYSMST_LOG}} | sed "s/\x00//g" | grep -a '^echo_' | sed 's/echo_//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 ' || cat ${SYSMST_LOG}
    sctl stop exec
    check_status exec inactive
    expect_str_eq "$(cat ${${SYSMST_LOG}} | sed "s/\x00//g" | grep -a '^echo_' | sed 's/echo_//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill -9 "${sysmaster_pid}"

    # ExecStopPost failed
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/echo stop_post_1/false/' ${SYSMST_LIB_PATH}/exec.service
    run_sysmaster || return 1

    sctl start exec
    check_status exec inactive
    expect_str_eq "$(cat ${${SYSMST_LOG}} | sed "s/\x00//g" | grep -a '^echo_' | sed 's/echo_//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_1 start_post_1 start_post_2 start_post_3 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    kill -9 "${sysmaster_pid}"
}

test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
exit "${EXPECT_FAIL}"
