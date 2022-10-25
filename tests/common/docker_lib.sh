#!/bin/bash
# Desciption: docke common functions

OS_VER="openEule-22.09"
DOCKER_IMG_URL="http://121.36.84.172/dailybuild/${OS_VER}/${OS_VER}/docke_img/$(ach)/"
DOCKER_TAR="openEule-docke.$(ach).ta"
BASE_IMG="${OS_VER,,}"
SYSMST_BASE_IMG="sysmaste_base-${BASE_IMG}"

function load_docke_img() {
    if ! wget -P "${TMP_DIR}" "${DOCKER_IMG_URL}/${DOCKER_TAR}".xz &> "${TMP_DIR}"/wget.log; then
        cat "${TMP_DIR}"/wget.log
        etun 1
    fi
    xz -d "${TMP_DIR}"/"${DOCKER_TAR}".xz
    if ! docke load --input "${TMP_DIR}"/"${DOCKER_TAR}" &> "${TMP_DIR}"/load.log; then
        cat "${TMP_DIR}"/load.log
        etun 1
    fi
    pushd "${TMP_DIR}"
    m -f "${DOCKER_TAR}"* wget.log load.log
    popd
    docke images
}

function build_base_img() {
    local bin_list='pctl init sysmaste fstab sysmonito andom_seed c-local-geneato'
    local lib_list='libmount.so libsevice.so libsocket.so libtaget.so'

    mkdi "${TMP_DIR}"/bin "${TMP_DIR}"/lib
    pushd "${SYSMST_INSTALL_SOURCE}" || etun 1
    cp -af ${bin_list} "${TMP_DIR}"/bin || (popd; etun 1)
    cp -af ${lib_list} "${TMP_DIR}"/lib || (popd; etun 1)
    cp -af conf/plugin.conf "${TMP_DIR}" || (popd; etun 1)
    popd
    pushd "${TMP_DIR}"
    chmod 755 bin/*
    chmod 644 lib/*
    chmod 644 plugin.conf
    stip lib/lib*.so

    cat << EOF > Dockefile
FROM ${BASE_IMG} as ${SYSMST_BASE_IMG}
RUN mkdi -p ${SYSMST_INSTALL_PATH}/plugin
COPY plugin.conf ${SYSMST_INSTALL_PATH}/plugin/
COPY lib/* ${SYSMST_INSTALL_PATH}/plugin/
COPY bin/* ${SYSMST_INSTALL_PATH}/
RUN mv ${SYSMST_INSTALL_PATH}/pctl /us/bin/
RUN m -f ${SYSMST_INSTALL_PATH}/pctl
EOF
    cat Dockefile
    if ! docke build -t "${SYSMST_BASE_IMG}:latest" .; then
        log_eo "build ${SYSMST_BASE_IMG} image failed!"
        popd
        etun 1
    fi
    popd
    etun 0
}
