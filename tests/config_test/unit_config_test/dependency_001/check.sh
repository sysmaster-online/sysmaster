#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test dependency not exist
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/{conflicts.service,requires.service,wants.service} ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    sctl status base.service &> log
    check_log log 'No such file or directory' || return 1
    rm -rf log

    # Requires: dependency not exist leads to start failure
    sctl start requires.service
#    expect_ne $? 0 || return 1
    check_status requires.service inactive
#    expect_eq $? 0 || return 1

    # Wants: start normally when dependency not exist
    sctl start wants.service
    expect_eq $? 0 || return 1
    check_status wants.service active || return 1

    # clean
    sctl stop requires.service wants.service
    kill -9 "${sysmaster_pid}"
}

# usage: test dependency inactive
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    # Requires: dependency inactive leads to inactive
    sctl start requires.service
    expect_eq $? 0 || return 1
    check_status requires.service active || return 1
    check_status base.service active || return 1
    sctl stop base.service
    check_status requires.service inactive || return 1

    # Wants: stay active when dependency inactive leads to inactive
    sctl start wants.service
    expect_eq $? 0 || return 1
    check_status wants.service active || return 1
    check_status base.service active || return 1
    sctl stop base.service
    check_status wants.service active || return 1

    # clean
    sctl stop wants.service
    kill -9 "${sysmaster_pid}"
}

# usage: test conflict dependency
function test03() {
    log_info "===== test03 ====="
    run_sysmaster || return 1

    sctl start base.service
    check_status base.service active || return 1

    sctl start conflicts.service
    check_status conflicts.service active || return 1
    check_status base.service inactive || return 1

    sctl start base.service
    check_status base.service active || return 1
    check_status conflicts.service inactive || return 1

    # clean
    sctl stop conflicts.service
    kill -9 "${sysmaster_pid}"
}

# usage: test contradictory dependency
function test04() {
    log_info "===== test04 ====="
    sed -i "/Conflicts/a Requires=\"base.service\"" ${SYSMST_LIB_PATH}/conflicts.service
    run_sysmaster || return 1

    sctl start conflicts.service
    check_status conflicts.service inactive || return 1

    # clean
    kill -9 "${sysmaster_pid}"
}

# usage: test loop dependency
function test05() {
    log_info "===== test05 ====="
    sed -i "/Description/a Requires=\"requires.service\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start requires.service
    check_status requires.service active || return 1
    check_status base.service active || return 1

    # clean
    sctl stop base.service requires.service
    kill -9 "${sysmaster_pid}"
}

test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
test05 || exit 1
exit "${EXPECT_FAIL}"
