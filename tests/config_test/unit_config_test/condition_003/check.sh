#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test ConditionCapability
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Description=/ a ConditionCapability="CAP_SYS_ADMIN"' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1

    # ! means reverse
    sed -i 's/CAP_SYS_ADMIN/!CAP_SYS_ADMIN/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base inactive
    expect_eq $? 0 || return 1

    sed -i 's/!CAP_SYS_ADMIN/CAP_SSR/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base inactive
    expect_eq $? 0 || return 1
}

# usage: test ConditionKernelCommandLine
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1

    # single word
    sed -i '/Description=/ a ConditionKernelCommandLine="ro"' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1

    sed -i 's/^ConditionKernelCommandLine=.*/ConditionKernelCommandLine="crash"/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base inactive
    expect_eq $? 0 || return 1

    sed -i 's/^ConditionKernelCommandLine=.*/ConditionKernelCommandLine="crashkernel"/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1

    # key=value
    cmdline_para="$(cat /proc/cmdline | grep -oP 'crashkernel=\S+' | head -n1)"
    [ -z "${cmdline_para}" ] && return 1
    sed -i 's/^ConditionKernelCommandLine=.*/ConditionKernelCommandLine="crashkernel=9999M"/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base inactive
    expect_eq $? 0 || return 1

    sed -i "s/^ConditionKernelCommandLine=.*/ConditionKernelCommandLine=${cmdline_para}/" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1

    sed -i "s/^ConditionKernelCommandLine=.*/ConditionKernelCommandLine=${cmdline_para#crash}/" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base inactive
    expect_eq $? 0 || return 1

    sed -i "s/^ConditionKernelCommandLine=.*/ConditionKernelCommandLine=${cmdline_para%%M*}/" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base inactive
    expect_eq $? 0 || return 1
    # clean
    sctl stop base
}

# usage: test ConditionSecurity
function test03() {
    log_info "===== test03 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Description=/ a ConditionSecurity="selinux"' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base inactive
    expect_eq $? 0 || return 1

    sed -i 's/^ConditionSecurity=.*/ConditionSecurity="audit"/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base active
    expect_eq $? 0 || return 1
    # clean
    sctl stop base
}

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
test03 || exit 1
exit "${EXPECT_FAIL}"
