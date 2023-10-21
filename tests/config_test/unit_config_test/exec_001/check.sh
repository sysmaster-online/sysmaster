#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e
key_log_1="ERROR sysmaster.*No ExecStart command"
key_log_2="ERROR sysmaster.* load unit .*base.service] failed: Confique error"
key_log_3="ERROR sysmaster.* load unit .*base.service] failed: unit configuration error: 'More than Oneshot ExecStart command is configured, service type is not oneshot'"

# usage: test unit without ExecStart
function test01() {
    log_info "===== test01 ====="

    # no ExecStart
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/ExecStart/d' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    expect_eq $? 1
    check_log ${SYSMST_LOG} "${key_log_1}"
    expect_eq $? 0
    sctl status base 2>&1 &> log
    check_log log 'base.service: NotExisted'
    expect_eq $? 0
    rm -rf log

    # null ExecStart
    sed -i "/Service]/a ExecStart=" ${SYSMST_LIB_PATH}/base.service
    echo > ${SYSMST_LOG}
    sctl daemon-reload
    sctl restart base
    expect_eq $? 1
    check_log ${SYSMST_LOG} "${key_log_2}"
    expect_eq $? 0
    sctl status base 2>&1 &> log
    check_log log 'base.service: NotExisted'
    expect_eq $? 0
    rm -rf log
}

# usage: test multiple ExecStart
function test02() {
    log_info "===== test02 ====="

    # multiple commands in single ExecStart
    sed -i "s#ExecStart=.*#ExecStart=/bin/sleep 2 ; /bin/sleep 222#" ${SYSMST_LIB_PATH}/base.service
    echo > ${SYSMST_LOG}
    sctl daemon-reload
    sctl restart base
    expect_eq $? 1
    check_log ${SYSMST_LOG} "${key_log_3}"
    expect_eq $? 0
    sctl status base 2>&1 &> log
    check_log log 'base.service: NotExisted'
    expect_eq $? 0
    rm -rf log

    # Type="oneshot": multiple commands in single ExecStart
    sed -i '/ExecStart/ i Type="oneshot"' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base &
    check_load base true
    expect_eq $? 0
    check_status base activating
    expect_eq $? 0
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
    expect_eq $? 0

    # single command in multiple ExecStart
    sed -i "s#ExecStart=.*#ExecStart=/bin/sleep 99#" ${SYSMST_LIB_PATH}/base.service
    sed -i "/Service]/a ExecStart=/bin/sleep 100" ${SYSMST_LIB_PATH}/base.service
    echo > ${SYSMST_LOG}
    sctl daemon-reload
    sctl restart base
    expect_eq $? 1
    check_log ${SYSMST_LOG} "${key_log_2}"
    expect_eq $? 0
    sctl status base 2>&1 &> log
    check_log log 'base.service: NotExisted'
    expect_eq $? 0
    rm -rf log
}

# usage: test invalid ExecStart
function test03() {
    log_info "===== test03 ====="

    # inexecutable
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i "s#ExecStart=.**#ExecStart=/inexec#" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    touch /inexec
    chmod 400 /inexec
    sctl restart base
    expect_eq $? 0
    check_load base true
    expect_eq $? 0
    check_status base failed
    expect_eq $? 0
    # clean
    rm -rf /inexec

    # failed
    sed -i "s#ExecStart=.*#ExecStart=/usr/bin/false#" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    expect_eq $? 0
    check_load base true
    expect_eq $? 0
    check_status base failed
    expect_eq $? 0

    # failed but ignore
    sed -i "s#ExecStart=.*#ExecStart=-/usr/bin/false#" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    expect_eq $? 0
    check_load base true
    expect_eq $? 0
    check_status base inactive
    expect_eq $? 0
}

# usage: test ExecStartPre/ExecStartPost/ExecStop/ExecStopPost
function test04() {
    log_info "===== test04 ====="

    # exec success
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec
    check_status exec inactive
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_1 start_post_1 start_post_2 start_post_3 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}

    # ExecStartPre failed
    sed -i 's/echo echo_start_pre_2_echo/false/' ${SYSMST_LIB_PATH}/exec.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec
    check_status exec failed
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}

    # ExecStartPre failed but ignore
    sed -i 's#/usr/bin/false#-/usr/bin/false#' ${SYSMST_LIB_PATH}/exec.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec
    check_status exec inactive
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_3 start_1 start_post_1 start_post_2 start_post_3 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}

    # ExecStart failed
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/echo echo_start_1_echo/false/' ${SYSMST_LIB_PATH}/exec.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec
    check_status exec failed
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}

    # ExecStart failed but ignore
    sed -i 's#/usr/bin/false#-/usr/bin/false#' ${SYSMST_LIB_PATH}/exec.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec
    check_status exec failed
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}

    # ExecStartPost failed
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/echo echo_start_post_1_echo/false/' ${SYSMST_LIB_PATH}/exec.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec
    check_status exec inactive
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_1 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}

    # ExecStartPost failed but ignore
    sed -i 's#/usr/bin/false#-/usr/bin/false#' ${SYSMST_LIB_PATH}/exec.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec
    check_status exec inactive
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_1 start_post_2 start_post_3 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}

    # ExecStop failed
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/echo echo_start_1_echo/sleep 100/' ${SYSMST_LIB_PATH}/exec.service
    sed -i 's/echo echo_stop_1_echo/false/' ${SYSMST_LIB_PATH}/exec.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec
    check_status exec active
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 ' || cat ${SYSMST_LOG}
    sctl stop exec
    check_status exec failed
    expect_eq $? 0
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}

    # ExecStop failed but ignore
    sed -i 's#/usr/bin/false#-/usr/bin/false#' ${SYSMST_LIB_PATH}/exec.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec
    check_status exec active
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 ' || cat ${SYSMST_LOG}
    sctl stop exec
    check_status exec inactive
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_post_1 start_post_2 start_post_3 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}

    # ExecStopPost failed
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sed -i 's/echo stop_post_1_echo/false/' ${SYSMST_LIB_PATH}/exec.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec
    check_status exec inactive
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'start_pre_1 start_pre_2 start_pre_3 start_1 start_post_1 start_post_2 start_post_3 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
}

# usage: test ExecCondition
function test05() {
    log_info "===== test05 ====="
    cp -arf "${work_dir}"/tmp_units/exec.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Service/ a ExecCondition="/usr/bin/echo echo_condition_1_echo ; /usr/bin/echo echo_condition_2_echo"' ${SYSMST_LIB_PATH}/exec.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    # multiple success ExecCondition
    sctl start exec
    check_status exec inactive
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'condition_1 condition_2 start_pre_1 start_pre_2 start_pre_3 start_1 start_post_1 start_post_2 start_post_3 stop_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}

    # ExecCondition return 1
    sed -i 's#ExecCondition=.*#ExecCondition="/usr/bin/echo echo_condition_1_echo ; /usr/bin/false ; /usr/bin/echo echo_condition_2_echo"#' ${SYSMST_LIB_PATH}/exec.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec
    check_status exec inactive
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'condition_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}

    # ExecCondition return 254
    sed -i 's#ExecCondition=.*#ExecCondition="/usr/bin/echo echo_condition_1_echo ; /usr/bin/test.sh ; /usr/bin/echo echo_condition_2_echo"#' ${SYSMST_LIB_PATH}/exec.service
    echo "exit 254" > /usr/bin/test.sh
    chmod +x /usr/bin/test.sh
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec
    check_status exec inactive
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'condition_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    rm -rf /usr/bin/test.sh
    echo > "${SYSMST_LOG}"

    # ExecCondition return 255
    echo "exit 255" > /usr/bin/test.sh
    chmod +x /usr/bin/test.sh
    sctl start exec
    check_status exec failed
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'condition_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}
    # clean
    rm -rf /usr/bin/test.sh
    echo > "${SYSMST_LOG}"

    # ExecCondition return 255 (cmd not exist)
    sctl start exec
    check_status exec failed
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'condition_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}

    # ExecCondition killed
    sed -i 's#ExecCondition=.*#ExecCondition="/usr/bin/echo echo_condition_1_echo ; /usr/bin/sleep 12321 ; /usr/bin/echo echo_condition_2_echo"#' ${SYSMST_LIB_PATH}/exec.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec &
    sleep 1
    check_status exec 'activating (condition)'
    expect_eq $? 0
    pid="$(ps aux | grep -v grep | grep 'sleep 12321' | awk '{print $2}')"
    kill -9 "${pid}"
    check_status exec 'failed'
    expect_eq $? 0
    sync
    sleep 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'condition_1 stop_post_1 stop_post_2 stop_post_3 ' \
        || cat ${SYSMST_LOG}

    # ExecCondition timeout
    sed -i '/ExecCondition/a TimeoutSec=2' ${SYSMST_LIB_PATH}/exec.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl start exec &
    sleep 1
    check_status exec 'activating (condition)'
    expect_eq $? 0
    sleep 1.5
    check_status exec 'failed'
    expect_eq $? 0
    sync
    sleep 1
    check_log ${SYSMST_LOG} 'condition operation time out. enter StopSigterm'
    expect_eq $? 0
    grep -a 'operation time out' ${SYSMST_LOG} | grep -v 'condition'
    expect_eq $? 1
    expect_str_eq "$(cat ${SYSMST_LOG} | sed "s/\x00//g" | grep -a '_echo$' | sed 's/.*echo_//g; s/_echo//g' | tr '\n' ' ')" \
        'condition_1 stop_post_1 stop_post_2 stop_post_3 '
}

# usage: test ExecReload
function test06() {
    log_info "===== test06 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/ExecStart/ a ExecReload="/bin/kill -9 \$MAINPID"' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl start base
    check_status base active
    expect_eq $? 0 || return 1
    sctl reload base
    expect_eq $? 0
    check_status base failed
    expect_eq $? 0 || return 1
}

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
test05 || exit 1
test06 || exit 1
exit "${EXPECT_FAIL}"
