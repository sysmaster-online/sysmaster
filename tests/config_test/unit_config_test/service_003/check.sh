#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test Sockets
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/{base.service,base.socket} ${SYSMST_LIB_PATH} || return 1
    sed -i "/Socket/a ListenStream=\"${test_socket}\"" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    sctl restart base
    check_status base.service active || return 1
    sctl status base.socket
    expect_ne $? 0
    # clean
    sctl stop base
    check_status base.service inactive || return 1
    kill_sysmaster

    # no matching socket
    mv ${SYSMST_LIB_PATH}/base.socket ${SYSMST_LIB_PATH}/base1.socket
    run_sysmaster || return 1

    sctl restart base
    check_status base.service active || return 1
    sctl status base1.socket
    expect_ne $? 0
    # clean
    sctl stop base
    check_status base.service inactive || return 1
    kill_sysmaster

    # use Sockets, without matching service
    sed -i '/ExecStart/a Sockets="base1.socket"' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl restart base
    check_status base.service active || return 1
    sctl status base1.socket
    expect_ne $? 0
    # clean
    sctl stop base
    check_status base.service inactive || return 1
    kill_sysmaster

    # use Sockets, with matching service
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH}/base1.service
    run_sysmaster || return 1

    sctl restart base
    check_status base.service active || return 1
    check_status base1.service inactive || return 1
    check_status base1.socket 'active (listening)' || return 1
    # clean
    sctl stop base1.socket
    sctl stop base
    check_status base.service inactive || return 1
    check_status base1.socket inactive || return 1
    kill_sysmaster
}

seed="${RANDOM}"
test_socket="/run/test_${seed}.socket"

test01 || exit 1
exit "${EXPECT_FAIL}"
