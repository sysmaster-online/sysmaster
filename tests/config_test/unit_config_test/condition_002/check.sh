#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test ConditionACPower
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i "/Description=/ a ConditionACPower=true" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl start base
    check_status base active
    expect_eq $? 0 || return 1


    sed -i '/ConditionACPower=/ s#true#false#' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base inactive
    expect_eq $? 0 || return 1
    sctl stop base
}

# usage: test ConditionFirstBoot
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i "/Description=/ a ConditionFirstBoot=false" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl start base
    check_status base active
    expect_eq $? 0 || return 1

    # create first-boot file
    sed -i '/ConditionFirstBoot=/ s#false#true#' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base inactive
    expect_eq $? 0 || return 1
    sctl stop base
    ls -l /run/sysmaster/first-boot
    expect_eq $? 2
    touch /run/sysmaster/first-boot
    sctl start base
    check_status base active
    expect_eq $? 0 || return 1

    # clean
    rm -rf /run/sysmaster/first-boot
    sctl stop base
}

# usage: test ConditionNeedsUpdate
function test03() {
    log_info "===== test03 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i "/Description=/ a ConditionNeedsUpdate=/etc" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    stat /etc.updated
    stat /usr
    sctl start base
    check_status base active
    expect_eq $? 0 || return 1

    sleep 1.5
    touch /usr/aaa
    stat /etc.updated
    stat /usr
    sctl stop base
    sctl start base
    check_status base inactive
    expect_eq $? 0 || return 1
    rm -rf /usr/aaa

    # clean
    sctl stop base
}

# usage: test ConditionUser
function test04() {
    log_info "===== test04 ====="
    test_user_1="test1_${RANDOM}"
    test_user_2="test2_${RANDOM}"
    user_pw_1="PW!test1_${RANDOM}"
    user_pw_2="PW!test2_${RANDOM}"
    yum install -y shadow sudo
    expect_eq $? 0 || return 1
    useradd "${test_user_1}"
    useradd "${test_user_2}"
    # echo "${user_pw_1}" | passwd --stdin "${test_user_1}"
    # echo "${user_pw_2}" | passwd --stdin "${test_user_2}"

    # run sysmaster as root
    # user = root
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Description=/ a ConditionUser="root"' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl start base
    check_status base active
    expect_eq $? 0 || return 1

    # user = 0
    sed -i 's/ConditionUser=.*/ConditionUser="0"/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1

    # user = @system
    sed -i 's/ConditionUser=.*/ConditionUser="@system"/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1

    # user = normal user
    sed -i "s/ConditionUser=.*/ConditionUser=${test_user_1}/" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    # start service as root
    sctl start base
    check_status base inactive
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'Starting failed the unit condition test failed'
    expect_eq $? 0

    # user = normal user id
    # id not exist
    sed -i 's/ConditionUser=.*/ConditionUser="9999"/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    # start service as root
    sctl start base
    check_status base inactive
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'Starting failed the unit condition test failed'
    expect_eq $? 0

    # id exist
    id="$(grep -w "${test_user_1}" /etc/passwd | awk -F ':' '{print $3}')"
    expect_eq $? 0
    sed -i "s/ConditionUser=.*/ConditionUser=${id}/" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    # start service as root
    sctl start base
    check_status base inactive
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'Starting failed the unit condition test failed'
    expect_eq $? 0

    # run sysmaster as normal user
    # run_sysmaster "${test_user_1}" || return 1

    # start service as root
    # sctl start base
    # check_status base inactive
    # expect_eq $? 0 || return 1
    # check_log "${SYSMST_LOG}" 'Starting failed the unit condition test failed'
    # expect_eq $? 0
    # echo > "${SYSMST_LOG}"

    # start service as correct user
    # sudo -u "${test_user_1}" "sctl start base; sleep 1; sctl status base"
    # expect_eq $? 0 || return 1
    # sudo -u "${test_user_1}" "sctl stop base; sleep 1; sctl status base"
    # expect_eq $? 3 || return 1

    # start service as incorrect user
    # sudo -u "${test_user_2}" "sctl start base; sleep 1; sctl status base"
    # expect_eq $? 3 || return 1

    # clean
    userdel -rf "${test_user_1}"
    userdel -rf "${test_user_2}"
}

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
# ConditionNeedsUpdate not implemented yet
# test03 || exit 1
# user mode not implemented yet
test04 || exit 1
exit "${EXPECT_FAIL}"
