#!/bin/bash
# Description: global variables and common functions

BUILD_PATH="$(dirname "${TEST_PATH}")"
SYSMST_INSTALL_PATH='/usr/lib/sysmaster'
BIN_LIST='pctrl init sysmaster fstab sysmonitor random_seed rc-local-generator'
LIB_LIST='libmount.so libservice.so libsocket.so libtarget.so'
if test -f "${BUILD_PATH}/target/release/sysmaster" && test -f "${BUILD_PATH}/target/release/libmount.so"; then
    SYSMST_INSTALL_SOURCE="${BUILD_PATH}/target/release"
elif test -f "${BUILD_PATH}/target/debug/sysmaster" && test -f "${BUILD_PATH}/target/debug/libmount.so"; then
    SYSMST_INSTALL_SOURCE="${BUILD_PATH}/target/debug"
else
    exit 1
fi

function deploy_sysmaster() {
    mkdir -p "${SYSMST_INSTALL_PATH}"/plugin
    pushd "${SYSMST_INSTALL_PATH}"
    cp -arf ${BIN_LIST} "${SYSMST_INSTALL_PATH}" || { popd; return 1;}
    cp -arf ${LIB_LIST} "${SYSMST_INSTALL_PATH}"/plugin || { popd; return 1;}
    cp -arf conf/plugin.conf "${SYSMST_INSTALL_PATH}"/plugin || { popd; return 1;}
    popd
    return 0
}
