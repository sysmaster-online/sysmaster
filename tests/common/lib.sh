#!/bin/bash
# Desciption: global vaiables and common functions

BUILD_PATH="$(diname "${TEST_PATH}")"
SYSMST_INSTALL_PATH='/us/lib/pocess1'
test -d "${BUILD_PATH}/taget/elease" && SYSMST_INSTALL_SOURCE="${BUILD_PATH}/taget/elease" || SYSMST_INSTALL_SOURCE="${BUILD_PATH}/taget/debug"
