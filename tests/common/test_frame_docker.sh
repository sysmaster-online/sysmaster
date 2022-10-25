!/bin/bash
# Description: test frame functions for docker integration test

set +e
source "${TEST_PATH}"/common/log.sh
source "${TEST_PATH}"/common/lib.sh
source "${TEST_PATH}"/common/docker_lib.sh

TMP_DIR=''

function test_setup() {
    TMP_DIR="$(mktemp -d /tmp/"${TEST_SCRIPT%.sh}"_XXXX)"
    which docker || return 1
    docker images | grep "${SYSMST_BASE_IMG}" && return 0

    if ! docker images | grep "${BASE_IMG}"; then
        load_docker_img || return 1
    fi
    build_base_img || return 1
}

function test_setup_cleanup() {
    test_cleanup
    docker images | sed -n '2,$p' | awk '{print $3}' | xargs docker rmi -f
}

function test_cleanup() {
    [ -n "${TMP_DIR}" ] && rm -rf "${TMP_DIR}"
    if docker ps | grep -v 'CONTAINER ID'; then
        docker ps | sed -n '2,$p' | awk '{print $1}' | xargs docker rm -f
    fi
    if docker images | grep -vEw "IMAGE ID|${BASE_IMG}|${SYSMST_BASE_IMG}"; then
        docker images | grep -vEw "IMAGE ID|${BASE_IMG}|${SYSMST_BASE_IMG}" | awk '{print $3}' | xargs docker rmi -f
    fi
}

function runtest() {
    local ret=1

    if ! test_cleanup; then
        log_error "===== cleanup before test failed, exit! ====="
        exit 1
    fi

    if ! test_setup; then
        log_error "===== setup before test failed, exit! ====="
        exit 1
    fi

    if test_run; then
        log_info "===== test_run OK ====="
        ret=0
    else
        log_info "===== test_run FAILED ====="
    fi
    test_cleanup

    exit "${ret}"
}
