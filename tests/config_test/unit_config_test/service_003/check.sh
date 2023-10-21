#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test Sockets
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/{base.service,base.socket} ${SYSMST_LIB_PATH} || return 1
    sed -i "/Socket/a ListenStream=${test_socket}" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    sctl restart base
    check_status base.service active
    expect_eq $? 0 || return 1
    sctl status base.socket
    expect_ne $? 0

    # no matching socket
    mv ${SYSMST_LIB_PATH}/base.socket ${SYSMST_LIB_PATH}/base1.socket
    sctl daemon-reload
    sctl restart base
    check_status base.service active
    expect_eq $? 0 || return 1
    sctl status base1.socket
    expect_ne $? 0

    # use Sockets, without matching service
    sed -i '/ExecStart/a Sockets=base1.socket' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart base
    check_status base.service active
    expect_eq $? 0 || return 1
    sctl status base1.socket
    expect_ne $? 0

    # use Sockets, with matching service
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH}/base1.service
    sctl daemon-reload
    sctl restart base
    check_status base.service active
    expect_eq $? 0 || return 1
    check_status base1.service inactive
    expect_eq $? 0 || return 1
    check_status base1.socket 'active (listening)'
    expect_eq $? 0 || return 1
    # clean
    sctl stop base1.socket
    sctl stop base
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_status base1.socket inactive
    expect_eq $? 0 || return 1
}

seed="${RANDOM}"
test_socket="/run/test_${seed}.socket"

run_sysmaster || exit 1
test01 || exit 1
exit "${EXPECT_FAIL}"
