#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

seed="${RANDOM}"
test_user=test_user_"${seed}"
test_grp=test_grp_"${seed}"

# usage: test User/Group
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i "/Service/a User=\"${test_user}\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    # user not exist
    sctl restart base
    check_status base.service failed || return 1
    check_log "${SYSMST_LOG}" "Failed to add user to execute parameters"
    # clean
    kill_sysmaster

    # group not exist
    sed -i "s/User=.*/Group=\"${test_grp}\"/" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl restart base
    check_status base.service failed || return 1
    check_log "${SYSMST_LOG}" "Failed to add group to execute parameters"
    # clean
    kill_sysmaster

    # user/group exist
    install_pkg shadow
    expect_eq $? 0 || return 1
    useradd "${test_user}"
    groupadd "${test_grp}"
    sed -i "/Service/a User=\"${test_user}\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl restart base
    check_status base.service active || return 1
    main_pid="$(get_pids base)"
    ps -eo pid,user,group,cmd | grep -v grep | grep "${main_pid}"
    ps -eo pid,user,group,cmd | grep -v grep | grep "${main_pid}" | grep test_us | grep test_gr
    expect_eq $? 0
    # clean
    sctl stop base
    check_status base.service inactive || return 1
    kill_sysmaster

    userdel -rf "${test_user}"
    groupdel -f "${test_grp}"
}

# usage: test UMask
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/ExecStart/a ExecStartPre="/bin/touch /opt/umask"' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    check_status base.service active || return 1
    ls -l /opt/umask
    expect_eq $? 0
    stat_1="$(stat -c %a /opt/umask)"
    # clean
    sctl stop base
    check_status base.service inactive || return 1
    rm -rf /opt/umask
    kill_sysmaster

    sed -i '/Service/a UMask="0377"' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    check_status base.service active || return 1
    ls -l /opt/umask
    expect_eq $? 0
    stat_2="$(stat -c %a /opt/umask)"
    expect_eq "${stat_2}" 400
    expect_lt "${stat_2}" "${stat_1}"
    # clean
    sctl stop base
    check_status base.service inactive || return 1
    rm -rf /opt/umask
    kill_sysmaster
}

test01 || exit 1
test02 || exit 1
exit "${EXPECT_FAIL}"
