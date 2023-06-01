#!/usr/bin/env bash
# Description: global variables and common functions

if test -f "${BUILD_PATH}/target/release/sysmaster"; then
    MODE='release'
elif test -f "${BUILD_PATH}/target/debug/sysmaster"; then
    MODE='debug'
else
    exit 1
fi

function install_sysmaster() {
    test -d "${BUILD_PATH}"/target/install && return 0
    pushd "${BUILD_PATH}"
    sh -x install.sh "${MODE}" || { popd; return 1;}
    popd
}
