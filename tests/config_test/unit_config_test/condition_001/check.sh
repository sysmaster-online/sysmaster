#!/usr/bin/env bash

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
        sed -i "/Description=/ a ConditionPathExists=/tmp/path_exist" ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        sed -i "/Description=/ a AssertPathExists=/tmp/path_exist" ${SYSMST_LIB_PATH}/base.service
    fi
    rm -rf /tmp/path_exist
    sctl daemon-reload
    echo > "${SYSMST_LOG}"

    # path not exist
    sctl start base.service &> log
    check_status base.service inactive
    expect_eq $? 0 || return 1
    if [ "${condition_test}" -eq 0 ]; then
        check_log log "${key_log_2}"
        expect_eq $? 0
    elif [ "${condition_test}" -eq 1 ]; then
        expect_str_eq "$(cat log)" ''
    fi
    check_log "${SYSMST_LOG}" "${key_log_1}"
    expect_eq $? 0
    rm -rf log

    # file path
    touch /tmp/path_exist
    sctl stop base.service
    sctl start base.service
    check_status base.service active
    expect_eq $? 0 || return 1

    # dir path
    sctl stop base.service
    rm -rf /tmp/path_exist
    mkdir /tmp/path_exist
    sctl start base.service
    check_status base.service active
    expect_eq $? 0 || return 1

    # clean
    sctl stop base.service
    rm -rf /tmp/path_exist
}

# usage: test ConditionFileNotEmpty/AssertFileNotEmpty
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    if [ "${condition_test}" -eq 1 ]; then
        sed -i "/Description=/ a ConditionFileNotEmpty=/tmp" ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        sed -i "/Description=/ a AssertFileNotEmpty=/tmp" ${SYSMST_LIB_PATH}/base.service
    fi
    sctl daemon-reload
    echo > "${SYSMST_LOG}"

    # path is directory
    sctl restart base.service &> log
    sleep 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    if [ "${condition_test}" -eq 0 ]; then
        check_log log "${key_log_2}"
        expect_eq $? 0
    elif [ "${condition_test}" -eq 1 ]; then
        expect_str_eq "$(cat log)" ''
    fi
    check_log "${SYSMST_LOG}" "${key_log_1}"
    expect_eq $? 0
    rm -rf log

    rm -rf /tmp/file_not_empty
    if [ "${condition_test}" -eq 1 ]; then
        sed -i '/ConditionFileNotEmpty=/ s#/tmp#/tmp/file_not_empty#' ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        sed -i '/AssertFileNotEmpty=/ s#/tmp#/tmp/file_not_empty#' ${SYSMST_LIB_PATH}/base.service
    fi
    sctl daemon-reload

    # path not exist
    sctl restart base.service &> log
    sleep 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    if [ "${condition_test}" -eq 0 ]; then
        check_log log "${key_log_2}"
    elif [ "${condition_test}" -eq 1 ]; then
        expect_str_eq "$(cat log)" ''
    fi
    rm -rf log

    # path is an empty file
    touch /tmp/file_not_empty
    sctl restart base.service &> log
    sleep 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
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
    check_status base.service active
    expect_eq $? 0 || return 1

    # clean
    sctl stop base.service
    rm -rf /tmp/file_not_empty
}

# usage: test ConditionPathIsReadWrite/AssertPathIsReadWrite
function test03() {
    log_info "===== test03 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    if [ "${condition_test}" -eq 1 ]; then
        sed -i "/Description=/ a ConditionPathIsReadWrite=/path_rw" ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        sed -i "/Description=/ a AssertPathIsReadWrite=/path_rw" ${SYSMST_LIB_PATH}/base.service
    fi
    sctl daemon-reload
    echo > "${SYSMST_LOG}"

    # path not exist
    sctl restart base &> log
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1
    if [ "${condition_test}" -eq 0 ]; then
        check_log log "${key_log_2}"
        expect_eq $? 0
    elif [ "${condition_test}" -eq 1 ]; then
        expect_str_eq "$(cat log)" ''
    fi
    check_log "${SYSMST_LOG}" "${key_log_1}"
    expect_eq $? 0
    rm -rf log

    # valid path
    mkdir /path_rw
    sctl stop base
    sctl start base
    check_status base active
    expect_eq $? 0 || return 1

    # ro mounted path
    which mount || yum install -y /usr/bin/mount
    expect_eq $? 0 || return 1
    dd if=/dev/zero of=/tmp/mountfile bs=1M count=10
    mkfs.ext4 /tmp/mountfile
    expect_eq $? 0
    mount -o ro /tmp/mountfile /path_rw
    expect_eq $? 0
    mount | grep path_rw
    sctl restart base &> log
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1
    if [ "${condition_test}" -eq 0 ]; then
        check_log log "${key_log_2}"
        expect_eq $? 0
    elif [ "${condition_test}" -eq 1 ]; then
        expect_str_eq "$(cat log)" ''
    fi
    rm -rf log

    # clean
    sctl stop base
    umount /path_rw || umount -l /path_rw
    rm -rf /tmp/mountfile /path_rw
}

# usage: test ConditionDirectoryNotEmpty
function test04() {
    log_info "===== test04 ====="
    local dir=dir_"${RANDOM}"
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    if [ "${condition_test}" -eq 1 ]; then
        sed -i "/Description=/ a ConditionDirectoryNotEmpty=/${dir}" ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        return
    fi
    sctl daemon-reload

    # directory not exist
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1

    # directory empty
    mkdir -p /${dir}
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1

    # directory not empty
    mkdir -p /${dir}/dir
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1
    rm -rf /${dir}/${dir}
    touch /${dir}/.file
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1

    # file
    rm -rf /${dir}
    touch /${dir}
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1

    # symbolic link to dir
    rm -rf /${dir}
    mkdir /${dir}_source
    ln -s /${dir}_source /${dir}
    expect_eq $? 0
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1
    touch /${dir}_source/file
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1
    # clean
    rm -rf /${dir}_source /${dir}
}

# usage: test ConditionFileIsExecutable
function test05() {
    log_info "===== test05 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    if [ "${condition_test}" -eq 1 ]; then
        sed -i "/Description=/ a ConditionFileIsExecutable=/tmp" ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        return
    fi
    sctl daemon-reload

    # directory
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1

    # file not exist
    local file=file_"$RANDOM"
    sed -i "s#ConditionFileIsExecutable=.*#ConditionFileIsExecutable=/${file}#" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1

    # file not executable
    touch /${file}
    chmod 400 /${file}
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1

    # file executable
    chmod +x /${file}
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1

    # symbolic link
    rm -rf /${file}
    touch /${file}_source
    chmod 400 /${file}_source
    ln -s /${file}_source /${file}
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1
    chmod +x /${file}_source
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1
    # clean
    rm -rf /${file}_source /${file}
}

# usage: test ConditionPathExistsGlob
function test06() {
    log_info "===== test06 ====="
    local file=file_"$RANDOM"
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    if [ "${condition_test}" -eq 1 ]; then
        sed -i "/Description=/ a ConditionPathExistsGlob=/tmp/${file}*" ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        return
    fi
    sctl daemon-reload
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1
    touch /tmp/file
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1
    touch /tmp/${file}_1
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    # clean
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1
    rm -rf /tmp/file /tmp/${file}_1
}

# usage: test ConditionPathIsDirectory
function test07() {
    log_info "===== test07 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    if [ "${condition_test}" -eq 1 ]; then
        sed -i "/Description=/ a ConditionPathIsDirectory=/tmp" ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        return
    fi
    sctl daemon-reload
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1

    # path not exist
    local path=path_${RANDOM}
    sed -i "s#ConditionPathIsDirectory=.*#ConditionPathIsDirectory=/${path}#" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1

    # path is file
    touch /${path}
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1

    # path is dir
    rm -rf /${path}
    mkdir /${path}
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1

    # path is symbolic link
    rm -rf /${path}
    mkdir /${path}_source
    ln -s /${path}_source /${path}
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1
    # clean
    rm -rf /${path}_source /${path}
}

# usage: test ConditionPathIsMountPoint
function test08() {
    log_info "===== test08 ====="
    for mnt in /tmp /boot /opt none; do
        df -ha | awk '{print $6}' | grep "^${mnt}$" && break
    done
    [ "${mnt}" = none ] && return 1
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    if [ "${condition_test}" -eq 1 ]; then
        sed -i "/Description=/ a ConditionPathIsMountPoint=${mnt}" ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        return
    fi
    sctl daemon-reload
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1

    # path not exist
    local path=path_${RANDOM}
    sed -i "s#ConditionPathIsMountPoint=.*#ConditionPathIsMountPoint=/${path}#" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1

    # path not mount point
    touch /${path}
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1
    rm -rf /${path}
    mkdir /${path}
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1

    # symbolic mount point
    rm -rf /${path}
    ln -s ${mnt} /${path}
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    # clean
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1
    rm -rf /${path}
}

# usage: test ConditionPathIsSymbolicLink
function test09() {
    log_info "===== test09 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    if [ "${condition_test}" -eq 1 ]; then
        sed -i "/Description=/ a ConditionPathIsSymbolicLink=/tmp" ${SYSMST_LIB_PATH}/base.service
    elif [ "${condition_test}" -eq 0 ]; then
        return
    fi
    sctl daemon-reload
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1

    # path not exist
    local path=path_${RANDOM}
    sed -i "s#ConditionPathIsSymbolicLink=.*#ConditionPathIsSymbolicLink=/${path}#" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    sleep 1
    check_status base inactive
    expect_eq $? 0 || return 1

    # path is symbolic
    rm -rf /${path}
    ln -s /boot /${path}
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1
    rm -rf /${path}
    touch /${path}_source
    ln -s /${path}_source /${path}
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    check_status base inactive
    expect_eq $? 0 || return 1
    # clean
    rm -rf /${path}_source /${path}
}

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
test05 || exit 1
test06 || exit 1
test07 || exit 1
test08 || exit 1
test09 || exit 1
exit "${EXPECT_FAIL}"
