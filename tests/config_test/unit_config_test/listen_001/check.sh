#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test ListenDatagram
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/{base.service,base.socket} ${SYSMST_LIB_PATH} || return 1
    sed -i "/Socket/a ListenDatagram=${test01_socket}" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    # ListenDatagram is file
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_netstat "${test01_socket}" 'DGRAM'
    expect_eq $? 0 || return 1
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1

    sed -i "s#${test01_socket}#@${test01_socket}#" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_netstat "@${test01_socket}" 'DGRAM'
    expect_eq $? 0 || return 1
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1

    # ListenDatagram is number
    sed -i "s#^ListenDatagram=.*#ListenDatagram=${seed}1#" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_netstat ":::${seed}1" 'udp6'
    expect_eq $? 0 || return 1
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1

    # ListenDatagram is IPv4:port
    sed -i "s#^ListenDatagram=.*#ListenDatagram=127.0.0.1:${seed}1#" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_netstat "127.0.0.1:${seed}1" 'udp'
    expect_eq $? 0 || return 1
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1

    # ListenDatagram is IPv6:port
    sed -i "s#^ListenDatagram=.*#ListenDatagram=[::]:${seed}1#" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_netstat ":::${seed}1" 'udp6'
    expect_eq $? 0 || return 1
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1
}

# usage: test ListenStream
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.socket ${SYSMST_LIB_PATH} || return 1
    sed -i "/Socket/a ListenStream=${test02_socket}" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    # ListenStream is file
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_netstat "${test02_socket}" 'STREAM'
    expect_eq $? 0 || return 1
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1

    sed -i "s#${test02_socket}#@${test02_socket}#" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_netstat "@${test02_socket}" 'STREAM'
    expect_eq $? 0 || return 1
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1

    # ListenStream is number
    sed -i "s#^ListenStream=.*#ListenStream=${seed}2#" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_netstat ":::${seed}2" 'tcp6'
    expect_eq $? 0 || return 1
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1

    # ListenStream is IPv4:port
    sed -i "s#^ListenStream=.*#ListenStream=127.0.0.1:${seed}2#" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_netstat "127.0.0.1:${seed}2" 'tcp'
    expect_eq $? 0 || return 1
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1

    # ListenStream is IPv6:port
    sed -i "s#^ListenStream=.*#ListenStream=[::]:${seed}2#" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_netstat ":::${seed}2" 'tcp6'
    expect_eq $? 0 || return 1
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1
}

# usage: test ListenSequentialPacket
function test03() {
    log_info "===== test03 ====="
    cp -arf "${work_dir}"/tmp_units/base.socket ${SYSMST_LIB_PATH} || return 1
    sed -i "/Socket/a ListenSequentialPacket=${test03_socket}" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    # ListenSequentialPacket is file
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_netstat "${test03_socket}" 'SEQPACKET'
    expect_eq $? 0 || return 1
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1

    sed -i "s#${test03_socket}#@${test03_socket}#" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_netstat "@${test03_socket}" 'SEQPACKET'
    expect_eq $? 0 || return 1
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1

    # ListenSequentialPacket is number (not supported)
    sed -i "s#^ListenSequentialPacket=.*#ListenSequentialPacket=${seed}3#" ${SYSMST_LIB_PATH}/base.socket
    echo > "${SYSMST_LOG}"
    sctl daemon-reload
    sctl restart base.socket
    expect_eq $? 0
    check_status base.socket failed
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'ESOCKTNOSUPPORT: Socket type not supported'
    expect_eq $? 0
}

# usage: test ListenNetlink
function test04() {
    log_info "===== test04 ====="
    cp -arf "${work_dir}"/tmp_units/base.socket ${SYSMST_LIB_PATH} || return 1
    sed -i '/Socket/a ListenNetlink="route 0"' ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    ss -f netlink
    num="$(ss -f netlink | grep sysmaster | wc -l)"
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    ss -f netlink
    expect_eq "$(ss -f netlink | grep sysmaster | wc -l)" "$((num + 1))"
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1
    ss -f netlink
    expect_eq "$(ss -f netlink | grep sysmaster | wc -l)" "${num}"
}

seed="1$((${RANDOM} % 1000))"
test01_socket="/run/test01_${seed}.socket"
test02_socket="/run/test02_${seed}.socket"
test03_socket="/run/test03_${seed}.socket"
yum install -y net-tools iproute || exit 1

run_sysmaster || return 1
test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
exit "${EXPECT_FAIL}"
