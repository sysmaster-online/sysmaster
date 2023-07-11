#!/usr/bin/env bash
# Description: test for SuccessAction/FailureAction

TEST_SCRIPT="$(basename "$0")"
TEST_SCRIPT_PATH="$(dirname "$0")"

source "${BUILD_PATH}"/tests/test_frame.sh
set +e

function test_pre() {
    pushd "${TEST_SCRIPT_PATH}"
    rm -rf tmp_units
    mkdir tmp_units
    cp -arf "${TEST_PATH}"/test_units/tests/base.service tmp_units
    cp -arf "${TEST_PATH}"/test_units/tests/{reboot.target,poweroff.target,exit.target} tmp_units
    popd
}

function test_run() {
    local ret

    pushd "${TEST_SCRIPT_PATH}"
    if [ "${DOCKER_TEST}" == '1' ]; then
        mkdir -p "${TMP_DIR}"/opt
        cp -arf "$(realpath check.sh)" "${TMP_DIR}"/opt
        cp -arf "${TEST_PATH}"/common/util_lib.sh tmp_units "${TMP_DIR}"/opt
        chmod -R 777 "${TMP_DIR}"
        ${DOCKER_CMD} run --privileged --rm -v "${TMP_DIR}"/opt:/opt ${opts} "${SYSMST_BASE_IMG}" sh -c "sh -x /opt/check.sh &> /opt/check.log"
        ret=$?
        cat "${TMP_DIR}"/opt/check.log
        cat "${TMP_DIR}"/opt/sysmaster.log
    else
        cp -arf "${TEST_PATH}"/common/util_lib.sh ./
        sh -x check.sh &> check.log
        ret=$?
        cat check.log
        cat sysmaster.log
    fi

    rm -rf tmp_units check.log
    popd
    return "${ret}"
}

runtest
