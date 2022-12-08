#!/bin/bash
# Description: test for example

TEST_SCRIPT="$(basename "$0")"
TEST_SCRIPT_PATH="$(realpath "$0")"
TEST_SCRIPT_PATH="$(dirname "${TEST_SCRIPT_PATH}")"
TEST_PATH="$(cd ${TEST_SCRIPT_PATH}; cd ../../; pwd)"
source "${TEST_PATH}"/common/test_frame_docker.sh
set +e

function test_run() {
    local ret
    mkdir -p "${TMP_DIR}"/opt
    cp -arf "${TEST_SCRIPT_PATH}"/check.sh "${TMP_DIR}"/opt
    chmod -R 777 "${TMP_DIR}"
    docker run --privileged --rm -v "${TMP_DIR}"/opt:/opt "${SYSMST_BASE_IMG}" sh -c "sh -x /opt/check.sh &> /opt/check.log"
    ret=$?
    cat "${TMP_DIR}"/opt/check.log
    return "${ret}"
}

runtest
