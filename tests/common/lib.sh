#!/bin/bash
# Description: global variables and common functions

BUILD_PATH="$(dirname "${TEST_PATH}")"
SYSMST_INSTALL_PATH='/usr/lib/process1'
test -d "${BUILD_PATH}/target/release" && SYSMST_INSTALL_SOURCE="${BUILD_PATH}/target/release" || SYSMST_INSTALL_SOURCE="${BUILD_PATH}/target/debug"
