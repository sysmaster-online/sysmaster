#!/usr/bin/env bash

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
    sed -i "/Service/a User=${test_user}" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    # user not exist
    sctl restart base
    expect_ne $? 0
    check_log "${SYSMST_LOG}" "unit configuration error: 'invalid user'"
    expect_eq $? 0 || return 1

    # group not exist
    sed -i "s/User=.*/Group=${test_grp}/" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    sctl restart base
    expect_ne $? 0
    check_log "${SYSMST_LOG}" "unit configuration error: 'invalid group'"
    expect_eq $? 0 || return 1

    # user/group exist
    yum install -y shadow
    expect_eq $? 0 || return 1
    useradd "${test_user}"
    groupadd "${test_grp}"
    sed -i "/Service/a User=${test_user}" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base.service active
    expect_eq $? 0 || return 1
    main_pid="$(get_pids base)"
    ps -eo pid,user,group,cmd | grep -v grep | grep "${main_pid}"
    ps -eo pid,user,group,cmd | grep -v grep | grep "${main_pid}" | grep test_us | grep test_gr
    expect_eq $? 0
    # clean
    sctl stop base
    check_status base.service inactive
    expect_eq $? 0 || return 1

    userdel -rf "${test_user}"
    groupdel -f "${test_grp}"
}

# usage: test UMask
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/ExecStart/a ExecStartPre="/bin/touch /opt/umask"' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl start base
    check_status base.service active
    expect_eq $? 0 || return 1
    ls -l /opt/umask
    expect_eq $? 0
    stat_1="$(stat -c %a /opt/umask)"
    # clean
    rm -rf /opt/umask

    sed -i '/Service/a UMask="0377"' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base.service active
    expect_eq $? 0 || return 1
    ls -l /opt/umask
    expect_eq $? 0
    stat_2="$(stat -c %a /opt/umask)"
    expect_eq "${stat_2}" 400
    expect_lt "${stat_2}" "${stat_1}"
    # clean
    sctl stop base
    check_status base.service inactive
    expect_eq $? 0 || return 1
    rm -rf /opt/umask
}

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
exit "${EXPECT_FAIL}"
