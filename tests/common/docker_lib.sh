#!/bin/bash
# Description: docker common functions

OS_VER="openEuler-22.03-LTS-SP1"
DOCKER_IMG_URL="https://mirrors.nju.edu.cn/openeuler/${OS_VER}/docker_img/$(arch)/"
DOCKER_TAR="openEuler-docker.$(arch).tar"
BASE_IMG="${OS_VER,,}"
SYSMST_BASE_IMG="sysmaster_base-${BASE_IMG}"

function load_docker_img() {
    if ! wget -P "${TMP_DIR}" "${DOCKER_IMG_URL}/${DOCKER_TAR}".xz &> "${TMP_DIR}"/wget.log; then
        cat "${TMP_DIR}"/wget.log
        return 1
    fi
    xz -d "${TMP_DIR}"/"${DOCKER_TAR}".xz
    if ! docker load --input "${TMP_DIR}"/"${DOCKER_TAR}" &> "${TMP_DIR}"/load.log; then
        cat "${TMP_DIR}"/load.log
        return 1
    fi
    pushd "${TMP_DIR}"
    rm -rf "${DOCKER_TAR}"* wget.log load.log
    popd
    docker images
}

function build_base_img() {
    cp -arf ${BUILD_PATH}/target/install "${TMP_DIR}"

    pushd "${TMP_DIR}"
    cat << EOF > Dockerfile
FROM ${BASE_IMG} as ${SYSMST_BASE_IMG}
COPY install/usr/bin/sctl /usr/bin/
COPY install/usr/bin/init /usr/bin/
RUN mkdir /usr/lib/sysmaster
COPY install/usr/lib/sysmaster /usr/lib/sysmaster/
EOF
    cat Dockerfile
    if ! docker build -t "${SYSMST_BASE_IMG}:latest" .; then
        log_error "build ${SYSMST_BASE_IMG} image failed!"
        popd
        return 1
    fi
    popd
    return 0
}
