#!/usr/bin/env bash
# Description: test for example

TEST_SCRIPT="$(basename "$0")"
TEST_SCRIPT_PATH="$(dirname "$0")"

source "${BUILD_PATH}"/tests/test_frame.sh
set +e

function test_run() {
    local ret
    mkdir -p "${TMP_DIR}"/opt
    cp -arf "$(realpath "${TEST_SCRIPT_PATH}"/check.sh)" "${TMP_DIR}"/opt
    chmod -R 777 "${TMP_DIR}"
    ${DOCKER_CMD} run --privileged --rm -v "${TMP_DIR}"/opt:/opt ${opts} "${SYSMST_BASE_IMG}" sh -c "sh -x /opt/check.sh &> /opt/check.log"
    ret=$?
    cat "${TMP_DIR}"/opt/check.log
    return "${ret}"
}

runtest
