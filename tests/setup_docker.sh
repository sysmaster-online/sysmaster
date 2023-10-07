#!/usr/bin/env bash
# Description: build docker env for test local sysmaster

BUILD_PATH="$(cd $(dirname $0);cd ..;pwd)"
source "${BUILD_PATH}"/tests/common/docker_lib.sh

export DOCKER_CMD='docker'
TMP_DIR="$(mktemp -d /tmp/"${TEST_SCRIPT%.sh}"_XXXX)"

docker images | grep "${SYSMST_BASE_IMG}" && return 0

if ! docker images | grep "${BASE_IMG}"; then
    load_docker_img || return 1
fi
build_base_img local || return 1
