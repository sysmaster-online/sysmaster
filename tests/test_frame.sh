#!/bin/bash
# Description: test frame functions for docker & vm integration test

TEST_PATH="${BUILD_PATH}"/tests
source "${TEST_PATH}"/common/lib.sh
source "${TEST_PATH}"/common/docker_lib.sh
source "${TEST_PATH}"/common/util_lib.sh

set +e
TMP_DIR=''

function test_setup() {
    setenforce 0

    install_sysmaster || return 1

    if [ "${DOCKER_TEST}" == '1' ]; then
        setup_docker || return 1
    else
        test -f /usr/bin/sctl && return 0
        cp -arf "${BUILD_PATH}"/target/install/usr/* /usr/ || return 1
    fi

    return 0
}

function setup_docker() {
    TMP_DIR="$(mktemp -d /tmp/"${TEST_SCRIPT%.sh}"_XXXX)"

    if ! which docker; then
        sudo yum install -y docker || return 1
    fi
    docker images | grep "${SYSMST_BASE_IMG}" && return 0

    if ! docker images | grep "${BASE_IMG}"; then
        load_docker_img || return 1
    fi
    build_base_img || return 1
}

function test_cleanup() {
    [ -n "${TMP_DIR}" ] && rm -rf "${TMP_DIR}"
    rm -rf /usr/bin/sctl "${SYSMST_LIB_PATH}"

    if [ "${DOCKER_TEST}" == '1' ]; then
        cleanup_docker || return 1
    fi

    return 0
}

function cleanup_docker() {
    if docker ps | grep -v 'CONTAINER ID'; then
        docker ps | sed -n '2,$p' | awk '{print $1}' | xargs docker rm -f
    fi
    if docker images | grep -vEw "IMAGE ID|${BASE_IMG}|${SYSMST_BASE_IMG}"; then
        docker images | grep -vEw "IMAGE ID|${BASE_IMG}|${SYSMST_BASE_IMG}" | awk '{print $3}' | xargs docker rmi -f
    fi
}

function runtest() {
    local ret=1

    log_info "===== cleanup before test ====="
    test_cleanup

    if test_setup; then
        log_info "===== setup before test OK ====="
    else
        log_error "===== setup before test failed, exit! ====="
        exit 1
    fi

    if [ "$(type -t test_pre)" = 'function' ]; then
        if test_pre; then
            log_info "===== test prepare OK ====="
        else
            log_error "===== test prepare failed, cleanup & exit ! ====="
            test_cleanup
            exit 1
        fi
    fi

    if test_run; then
        log_info "===== test_run OK ====="
        ret=0
    else
        log_info "===== test_run FAILED ====="
    fi
    log_info "===== cleanup after test ====="
    test_cleanup

    exit "${ret}"
}
