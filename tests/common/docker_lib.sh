#!/bin/bash
# Description: docker common functions

OS_VER="openEuler-22.09"
#DOCKER_IMG_URL="http://121.36.84.172/dailybuild/${OS_VER}/${OS_VER}/docker_img/$(arch)/"
DOCKER_IMG_URL="http://121.36.84.172/dailybuild/${OS_VER}/openeuler-2022-12-05-20-54-44/docker_img/$(arch)"
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
    mkdir "${TMP_DIR}"/bin "${TMP_DIR}"/lib
    pushd "${SYSMST_INSTALL_SOURCE}" || return 1
    cp -arf ${BIN_LIST} "${TMP_DIR}"/bin || { popd; return 1;}
    cp -arf ${LIB_LIST} "${TMP_DIR}"/lib || { popd; return 1;}
    cp -arf conf/plugin.conf "${TMP_DIR}" || { popd; return 1;}
    popd
    pushd "${TMP_DIR}"
    chmod 755 bin/*
    chmod 644 lib/*
    chmod 644 plugin.conf
    strip lib/lib*.so

    cat << EOF > Dockerfile
FROM ${BASE_IMG} as ${SYSMST_BASE_IMG}
RUN mkdir -p ${SYSMST_INSTALL_PATH}/plugin
COPY plugin.conf ${SYSMST_INSTALL_PATH}/plugin/
COPY lib/* ${SYSMST_INSTALL_PATH}/plugin/
COPY bin/* ${SYSMST_INSTALL_PATH}/
RUN mv ${SYSMST_INSTALL_PATH}/pctrl /usr/bin/
RUN rm -rf ${SYSMST_INSTALL_PATH}/pctrl
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
