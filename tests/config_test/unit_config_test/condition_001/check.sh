#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e
if [ "${condition_test}" -eq 1 ]; then
    key_log_1="${cond_fail_log}"
elif [ "${condition_test}" -eq 0 ]; then
    key_log_1="${asst_fail_log}"
fi
key_log_2='asdasda'

# usage: test ConditionPathExists/AssertPathExists
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    if [ "${condition_test}" -eq 1 ]; then
        sed -i "/Description=/ a ConditionPathExists=\"/tmp/path_exist\"" ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        sed -i "/Description=/ a AssertPathExists=\"/tmp/path_exist\"" ${SYSMST_LIB_PATH}/base.service
    fi
    rm -rf /tmp/path_exist
    run_sysmaster || return 1

    # path not exist
    sctl start base.service &> log
    check_status base.service inactive || return 1
    if [ "${condition_test}" -eq 0 ]; then
        check_log log "${key_log_2}"
    elif [ "${condition_test}" -eq 1 ]; then
        expect_str_eq "$(cat log)" ''
    fi
    check_log "${SYSMST_LOG}" "${key_log_1}"
    rm -rf log

    # file path
    touch /tmp/path_exist
    sctl stop base.service
    sctl start base.service
    check_status base.service active || return 1

    # dir path
    sctl stop base.service
    rm -rf /tmp/path_exist
    mkdir /tmp/path_exist
    sctl start base.service
    check_status base.service active || return 1

    # clean
    sctl stop base.service
    rm -rf /tmp/path_exist
    kill_sysmaster
}

# usage: test ConditionFileNotEmpty/AssertFileNotEmpty
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    if [ "${condition_test}" -eq 1 ]; then
        sed -i "/Description=/ a ConditionFileNotEmpty=\"/tmp\"" ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        sed -i "/Description=/ a AssertFileNotEmpty=\"/tmp\"" ${SYSMST_LIB_PATH}/base.service
    fi
    run_sysmaster || return 1

    # path is directory
    sctl start base.service &> log
    check_status base.service inactive || return 1
    if [ "${condition_test}" -eq 0 ]; then
        check_log log "${key_log_2}"
    elif [ "${condition_test}" -eq 1 ]; then
        expect_str_eq "$(cat log)" ''
    fi
    check_log "${SYSMST_LOG}" "${key_log_1}"
    rm -rf log

    # clean
    kill_sysmaster

    rm -rf /tmp/file_not_empty
    if [ "${condition_test}" -eq 1 ]; then
        sed -i '/ConditionFileNotEmpty=/ s#/tmp#/tmp/file_not_empty#' ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        sed -i '/AssertFileNotEmpty=/ s#/tmp#/tmp/file_not_empty#' ${SYSMST_LIB_PATH}/base.service
    fi
    run_sysmaster || return 1

    # path not exist
    sctl start base.service &> log
    check_status base.service inactive || return 1
    if [ "${condition_test}" -eq 0 ]; then
        check_log log "${key_log_2}"
    elif [ "${condition_test}" -eq 1 ]; then
        expect_str_eq "$(cat log)" ''
    fi
    rm -rf log

    # path is an empty file
    touch /tmp/file_not_empty
    sctl stop base.service
    sctl start base.service &> log
    check_status base.service inactive || return 1
    if [ "${condition_test}" -eq 0 ]; then
        check_log log "${key_log_2}"
    elif [ "${condition_test}" -eq 1 ]; then
        expect_str_eq "$(cat log)" ''
    fi
    rm -rf log

    # valid file path
    echo 1 > /tmp/file_not_empty
    sctl stop base.service
    sctl start base.service
    check_status base.service active || return 1

    # clean
    sctl stop base.service
    rm -rf /tmp/file_not_empty
    kill_sysmaster
}

# usage: test ConditionPathIsReadWrite/AssertPathIsReadWrite
function test03() {
    log_info "===== test03 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    if [ "${condition_test}" -eq 1 ]; then
        sed -i "/Description=/ a ConditionPathIsReadWrite=\"/path_rw\"" ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        sed -i "/Description=/ a AssertPathIsReadWrite=\"/path_rw\"" ${SYSMST_LIB_PATH}/base.service
    fi
    run_sysmaster || return 1

    # path not exist
    sctl start base &> log
    check_status base inactive || return 1
    if [ "${condition_test}" -eq 0 ]; then
        check_log log "${key_log_2}"
    elif [ "${condition_test}" -eq 1 ]; then
        expect_str_eq "$(cat log)" ''
    fi
    check_log "${SYSMST_LOG}" "${key_log_1}"
    rm -rf log

    # valid path
    mkdir /path_rw
    sctl stop base
    sctl start base
    check_status base active || return 1

    # ro mounted path
    which mount || install_pkg /usr/bin/mount
    expect_eq $? 0 || return 1
    dd if=/dev/zero of=/tmp/mountfile bs=1M count=10
    mkfs.ext4 /tmp/mountfile
    mount -o ro /tmp/mountfile /path_rw
    expect_eq $? 0
    sctl stop base
    sctl start base &> log
    check_status base inactive || return 1
    if [ "${condition_test}" -eq 0 ]; then
        check_log log "${key_log_2}"
    elif [ "${condition_test}" -eq 1 ]; then
        expect_str_eq "$(cat log)" ''
    fi
    rm -rf log

    # clean
    sctl stop base
    umount /path_rw
    rm -rf /tmp/mountfile /path_rw
    kill_sysmaster
}

test01 || exit 1
test02 || exit 1
test03 || exit 1
exit "${EXPECT_FAIL}"
