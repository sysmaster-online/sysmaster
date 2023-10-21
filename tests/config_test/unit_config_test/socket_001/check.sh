#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test Service
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/{base.service,base.socket} ${SYSMST_LIB_PATH} || return 1
    sed -i "/Socket/a ListenStream=${test_socket}" ${SYSMST_LIB_PATH}/base.socket
    cp -arf ${SYSMST_LIB_PATH}/base.service ${SYSMST_LIB_PATH}/base1.service
    sctl daemon-reload
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    ls -l ${test_socket}
    expect_eq $? 0
    echo A | nc -w1 -U "${test_socket}" &
    check_status base.service 'active (running)'
    expect_eq $? 0 || return 1
    check_status base.socket 'active (running)'
    expect_eq $? 0 || return 1
    sctl status base1.service
    expect_ne $? 0
    pkill -9 nc
    # stop socket before stop service
    sctl stop base.service &> log
    check_log log "asdasd"
    expect_eq $? 0
    rm -rf log
    check_status base.service active
    expect_eq $? 0 || return 1
    sctl stop base.socket
    sctl stop base.service
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_status base.socket inactive
    expect_eq $? 0 || return 1
    ls -l ${test_socket}
    expect_eq $? 0

    sed -i '/ListenStream/a Service="base1.service"' ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    sctl restart base.socket
    check_status base.socket 'active (listening)'
    expect_eq $? 0 || return 1
    echo A | nc -w1 -U "${test_socket}" &
    check_status base1.service active
    expect_eq $? 0 || return 1
    check_status base.socket 'active (running)'
    expect_eq $? 0 || return 1
    sctl status base.service
    expect_eq $? 1
    pkill -9 nc
    sctl stop base.socket
    sctl stop base1.service
    check_status base1.service inactive
    expect_eq $? 0 || return 1
    check_status base.socket inactive
    expect_eq $? 0 || return 1
    # clean
    rm -rf "${test_socket}"
}

# usage: test Symlinks/RemoveOnStop
function test02() {
    log_info "===== test02 ====="
    sed -i "/ListenStream/a Symlinks=${test_symlink}1 ${test_symlink}2" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    # multiple Symlinks
    sctl restart base.socket
    check_status base.socket active
    expect_eq $? 0 || return 1
    ls -l ${test_symlink}1 ${test_symlink}2
    expect_eq $? 0
    expect_str_eq "$(realpath ${test_symlink}1)" "${test_socket}"
    expect_str_eq "$(realpath ${test_symlink}2)" "${test_socket}"

    # default RemoveOnStop=false
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1
    ls -l ${test_symlink}1 ${test_symlink}2
    expect_eq $? 0
    expect_str_eq "$(realpath ${test_symlink}1)" "${test_socket}"
    expect_str_eq "$(realpath ${test_symlink}2)" "${test_socket}"

    # RemoveOnStop=true
    sed -i '/ListenStream/a RemoveOnStop=true' ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    sctl restart base.socket
    check_status base.socket active
    expect_eq $? 0 || return 1
    ls -l ${test_symlink}1 ${test_symlink}2
    expect_eq $? 0
    expect_str_eq "$(realpath ${test_symlink}1)" "${test_socket}"
    expect_str_eq "$(realpath ${test_symlink}2)" "${test_socket}"
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1
    ls -l "${test_socket}" || ls -l ${test_symlink}1 || ls -l ${test_symlink}2
    expect_eq $? 2
}

# usage: test Symlinks
function test03() {
    log_info "===== test03 ====="
    sed -i "s#ListenStream=.*#ListenStream=127.0.0.1:${seed}1#" ${SYSMST_LIB_PATH}/base.socket
    sctl daemon-reload
    # not AF_UNIX
    sctl restart base.socket
    check_status base.socket active
    expect_eq $? 0 || return 1
    check_netstat "127.0.0.1:${seed}1" 'tcp'
    expect_eq $? 0
    ls "${test_socket}" || ls -l ${test_symlink}1 || ls -l ${test_symlink}2
    expect_eq $? 2
    sctl stop base.socket
    check_status base.socket inactive
    expect_eq $? 0 || return 1
}

seed="1$((${RANDOM} % 1000))"
test_socket="/run/test_${seed}.socket"
test_symlink="/dev/test_${seed}_link"
yum install -y net-tools nmap || exit 1

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
test03 || exit 1
exit "${EXPECT_FAIL}"
