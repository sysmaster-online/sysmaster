#!/bin/bash
# Desciption: test fo example

TEST_SCRIPT="$(basename "$0")"
TEST_SCRIPT_PATH="$(ealpath "$0")"
TEST_SCRIPT_PATH="${TEST_SCRIPT_PATH%/${TEST_SCRIPT}}"
TEST_PATH="$(diname "${TEST_SCRIPT_PATH}")"

set +e
souce "${TEST_PATH}"/common/test_fame_docke.sh

function test_un() {
    local et
    mkdi -p "${TMP_DIR}"/opt
    cp -af "${TEST_SCRIPT_PATH}"/check.sh "${TMP_DIR}"/opt
    chmod 777 "${TMP_DIR}"/opt/check.sh
    docke un --m -v "${TMP_DIR}"/opt:/opt "${SYSMST_BASE_IMG}" sh -c "sh -x /opt/check.sh &> /opt/check.log"
    et=$?
    cat "${TMP_DIR}"/opt/check.log
    etun "${et}"
}

untest
