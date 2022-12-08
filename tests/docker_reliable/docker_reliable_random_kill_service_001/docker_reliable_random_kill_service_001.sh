#!/bin/bash
# Description: test for example

TEST_SCRIPT="$(basename "$0")"
TEST_SCRIPT_PATH="$(realpath "$0")"
TEST_SCRIPT_PATH="$(dirname "${TEST_SCRIPT_PATH}")"
TEST_PATH="$(cd ${TEST_SCRIPT_PATH}; cd ../../; pwd)"
source "${TEST_PATH}"/common/test_frame_docker.sh
set +e

function test_run() {
    local ret pid
    mkdir -p "${TMP_DIR}"/opt
    cp -arf "${TEST_SCRIPT_PATH}"/check.sh "${TEST_SCRIPT_PATH}"/test1.service "${TEST_SCRIPT_PATH}"/test2.service "${TMP_DIR}"/opt
    chmod -R 777 "${TMP_DIR}"

    log_info "============== exec sysmaster on host to mount cgroup =============="
    "${SYSMST_INSTALL_PATH}"/sysmaster &> sysmaster_host.log &
    pid=$!
    sleep 5
    kill -9 "${pid}"
    ps aux | grep -v grep | grep sysmaster
    cat sysmaster_host.log
    rm -rf sysmaster_host.log

    log_info "============== start check =============="
    docker run --privileged --rm -v "${TMP_DIR}"/opt:/opt "${SYSMST_BASE_IMG}" sh -c "sh -x /opt/check.sh &> /opt/check.log"
    ret=$?

    log_info "============== print check.log =============="
    cat "${TMP_DIR}"/opt/check.log
    return "${ret}"
}

runtest
