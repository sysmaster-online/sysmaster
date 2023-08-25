#!/usr/bin/env bash
# Description: test frame functions for docker & vm integration test

TEST_PATH="${BUILD_PATH}"/tests
source "${TEST_PATH}"/common/lib.sh
source "${TEST_PATH}"/common/docker_lib.sh
source "${TEST_PATH}"/common/util_lib.sh

set +e
TMP_DIR=''

function test_setup() {
    setenforce 0

    if ! yum list sysmaster; then
        install_sysmaster || return 1
    fi

    if [ "${DOCKER_TEST}" == '1' ]; then
        if which isula-build; then
            setup_isula || return 1
        elif which docker; then
            setup_docker || return 1
        else
            return 1
        fi
    else
        test -f /usr/bin/sctl && return 0
        cp -arf "${BUILD_PATH}"/target/install/usr/* /usr/ || return 1
    fi

    return 0
}

function setup_docker() {
    export DOCKER_CMD='docker'
    TMP_DIR="$(mktemp -d /tmp/"${TEST_SCRIPT%.sh}"_XXXX)"

    docker images | grep "${SYSMST_BASE_IMG}" && return 0

    if ! docker images | grep "${BASE_IMG}"; then
        load_docker_img || return 1
    fi
    build_base_img || return 1
}

function setup_isula() {
    export DOCKER_CMD='isula'
    export opts='--net=host'
    TMP_DIR="$(mktemp -d /tmp/"${TEST_SCRIPT%.sh}"_XXXX)"

    isula images | grep "${SYSMST_BASE_IMG}" && return 0

    if ! isula-build ctr-img images | grep "${BASE_IMG}"; then
        load_isula_img || return 1
    fi
    build_isula_img || return 1
}

function test_cleanup() {
    [ -n "${TMP_DIR}" ] && rm -rf "${TMP_DIR}"

    [ -n "/run/sysmaster/reliability" ] &&  rm -rf /run/sysmaster/reliability/*

    if [ "${DOCKER_TEST}" == '1' ] && [ -n "${DOCKER_CMD}" ]; then
        cleanup_docker || return 1
    fi

    return 0
}

function cleanup_docker() {
    if ${DOCKER_CMD} ps -a | grep -v 'CONTAINER ID'; then
        ${DOCKER_CMD} ps -a | sed -n '2,$p' | awk '{print $1}' | xargs ${DOCKER_CMD} stop
        ${DOCKER_CMD} ps -a | sed -n '2,$p' | awk '{print $1}' | xargs ${DOCKER_CMD} rm
        ${DOCKER_CMD} ps -a
    fi
    if ${DOCKER_CMD} images | grep -vEw "IMAGE ID|${BASE_IMG}|${SYSMST_BASE_IMG}"; then
        ${DOCKER_CMD} images | grep -vEw "IMAGE ID|${BASE_IMG}|${SYSMST_BASE_IMG}" | awk '{print $3}' | xargs ${DOCKER_CMD} rmi
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

    log_info "===== test_run begin ====="
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
