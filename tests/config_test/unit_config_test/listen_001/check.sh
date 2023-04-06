#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test ListenDatagram
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/{base.service,base.socket} ${SYSMST_LIB_PATH} || return 1
    sed -i "/Socket/a ListenDatagram=\"${test01_socket}\"" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    # ListenDatagram is file
    sctl restart base.socket
    check_status base.socket 'active(listening)' || return 1
    check_status base.service inactive || return 1
    check_netstat "${test01_socket}" 'DGRAM' || return 1
    sctl stop base.socket
    check_status base.socket inactive || return 1
    # clean
    kill_sysmaster

    sed -i "s#${test01_socket}#@${test01_socket}#" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    sctl restart base.socket
    check_status base.socket 'active(listening)' || return 1
    check_status base.service inactive || return 1
    check_netstat "@${test01_socket}" 'DGRAM' || return 1
    sctl stop base.socket
    check_status base.socket inactive || return 1
    # clean
    kill_sysmaster

    # ListenDatagram is number
    sed -i "s#^ListenDatagram=.*#ListenDatagram=\"${seed}1\"#" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    sctl restart base.socket
    check_status base.socket 'active(listening)' || return 1
    check_status base.service inactive || return 1
    check_netstat ":::${seed}1" 'udp6' || return 1
    sctl stop base.socket
    check_status base.socket inactive || return 1
    # clean
    kill_sysmaster

    # ListenDatagram is IPv4:port
    sed -i "s#^ListenDatagram=.*#ListenDatagram=\"127.0.0.1:${seed}1\"#" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    sctl restart base.socket
    check_status base.socket 'active(listening)' || return 1
    check_status base.service inactive || return 1
    check_netstat "127.0.0.1:${seed}1" 'udp' || return 1
    sctl stop base.socket
    check_status base.socket inactive || return 1
    # clean
    kill_sysmaster

    # ListenDatagram is IPv6:port
    sed -i "s#^ListenDatagram=.*#ListenDatagram=\"[::]:${seed}1\"#" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    sctl restart base.socket
    check_status base.socket 'active(listening)' || return 1
    check_status base.service inactive || return 1
    check_netstat ":::${seed}1" 'udp6' || return 1
    sctl stop base.socket
    check_status base.socket inactive || return 1
    # clean
    kill_sysmaster
}

# usage: test ListenStream
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.socket ${SYSMST_LIB_PATH} || return 1
    sed -i "/Socket/a ListenStream=\"${test02_socket}\"" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    # ListenStream is file
    sctl restart base.socket
    check_status base.socket 'active(listening)' || return 1
    check_status base.service inactive || return 1
    check_netstat "${test02_socket}" 'STREAM' || return 1
    sctl stop base.socket
    check_status base.socket inactive || return 1
    # clean
    kill_sysmaster

    sed -i "s#${test02_socket}#@${test02_socket}#" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    sctl restart base.socket
    check_status base.socket 'active(listening)' || return 1
    check_status base.service inactive || return 1
    check_netstat "@${test02_socket}" 'STREAM' || return 1
    sctl stop base.socket
    check_status base.socket inactive || return 1
    # clean
    kill_sysmaster

    # ListenStream is number
    sed -i "s#^ListenStream=.*#ListenStream=\"${seed}2\"#" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    sctl restart base.socket
    check_status base.socket 'active(listening)' || return 1
    check_status base.service inactive || return 1
    check_netstat ":::${seed}2" 'tcp6' || return 1
    sctl stop base.socket
    check_status base.socket inactive || return 1
    # clean
    kill_sysmaster

    # ListenStream is IPv4:port
    sed -i "s#^ListenStream=.*#ListenStream=\"127.0.0.1:${seed}2\"#" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    sctl restart base.socket
    check_status base.socket 'active(listening)' || return 1
    check_status base.service inactive || return 1
    check_netstat "127.0.0.1:${seed}2" 'tcp' || return 1
    sctl stop base.socket
    check_status base.socket inactive || return 1
    # clean
    kill_sysmaster

    # ListenStream is IPv6:port
    sed -i "s#^ListenStream=.*#ListenStream=\"[::]:${seed}2\"#" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    sctl restart base.socket
    check_status base.socket 'active(listening)' || return 1
    check_status base.service inactive || return 1
    check_netstat ":::${seed}2" 'tcp6' || return 1
    sctl stop base.socket
    check_status base.socket inactive || return 1
    # clean
    kill_sysmaster
}

# usage: test ListenSequentialPacket
function test03() {
    log_info "===== test03 ====="
    cp -arf "${work_dir}"/tmp_units/base.socket ${SYSMST_LIB_PATH} || return 1
    sed -i "/Socket/a ListenSequentialPacket=\"${test03_socket}\"" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    # ListenSequentialPacket is file
    sctl restart base.socket
    check_status base.socket 'active(listening)' || return 1
    check_status base.service inactive || return 1
    check_netstat "${test03_socket}" 'SEQPACKET' || return 1
    sctl stop base.socket
    check_status base.socket inactive || return 1
    # clean
    kill_sysmaster

    sed -i "s#${test03_socket}#@${test03_socket}#" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    sctl restart base.socket
    check_status base.socket 'active(listening)' || return 1
    check_status base.service inactive || return 1
    check_netstat "@${test03_socket}" 'SEQPACKET' || return 1
    sctl stop base.socket
    check_status base.socket inactive || return 1
    # clean
    kill_sysmaster

    # ListenSequentialPacket is number (not supported)
    sed -i "s#^ListenSequentialPacket=.*#ListenSequentialPacket=\"${seed}3\"#" ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    sctl restart base.socket
    expect_eq $? 1
    check_status base.socket failed || return 1
    check_log "${SYSMST_LOG}" 'failed: OtherError asda'
    # clean
    kill_sysmaster
}

# usage: test ListenNetlink
function test04() {
    log_info "===== test04 ====="
    cp -arf "${work_dir}"/tmp_units/base.socket ${SYSMST_LIB_PATH} || return 1
    sed -i '/Socket/a ListenNetlink="route 0"' ${SYSMST_LIB_PATH}/base.socket
    run_sysmaster || return 1

    ss -f netlink
    num="$(ss -f netlink | grep sysmaster | wc -l)"
    sctl restart base.socket
    check_status base.socket 'active(listening)' || return 1
    check_status base.service inactive || return 1
    ss -f netlink
    expect_eq "$(ss -f netlink | grep sysmaster | wc -l)" "$((num + 1))"
    sctl stop base.socket
    check_status base.socket inactive || return 1
    ss -f netlink
    expect_eq "$(ss -f netlink | grep sysmaster | wc -l)" "${num}"
    # clean
    kill_sysmaster
}

seed="1$((${RANDOM} % 1000))"
test01_socket="/run/test01_${seed}.socket"
test02_socket="/run/test02_${seed}.socket"
test03_socket="/run/test03_${seed}.socket"
install_pkg net-tools iproute || exit 1

test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
exit "${EXPECT_FAIL}"
