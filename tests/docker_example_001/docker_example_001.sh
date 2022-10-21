#!/bin/bash
# Description: test for example

TEST_SCRIPT="$(basename "$0")"
TEST_SCRIPT_PATH="$(realpath "$0")"
TEST_SCRIPT_PATH="${TEST_SCRIPT_PATH%/${TEST_SCRIPT}}"
TEST_PATH="$(dirname "${TEST_SCRIPT_PATH}")"

set +e
source "${TEST_PATH}"/common/test_frame_docker.sh

function test_run() {
    local ret
    mkdir -p "${TMP_DIR}"/opt
    cp -arf "${TEST_SCRIPT_PATH}"/check.sh "${TMP_DIR}"/opt
    chmod 777 "${TMP_DIR}"/opt/check.sh
    docker run --rm -v "${TMP_DIR}"/opt:/opt "${SYSMST_BASE_IMG}" sh -c "sh -x /opt/check.sh &> /opt/check.log"
    ret=$?
    cat "${TMP_DIR}"/opt/check.log
    return "${ret}"
}

runtest