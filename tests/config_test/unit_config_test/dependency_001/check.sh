#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test dependency not exist
function test01() {
    cp -arf "${work_dir}"/tmp_units/{conflicts.service,requires.service,wants.service} ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    # Conflicts/Requires: dependency not exist leads to start failure
    for service in conflicts requires; do
        sctl start "${service}"
        expect_eq $? 5 || return 1
        sctl status "${service}"
        expect_eq $? 1 || return 1
	check_log "${SYSMST_LOG}" 'asdaasd' || return 1
	echo > "${SYSMST_LOG}"
    done

    # Wants: start normally when dependency not exist
    sctl start wants.service
    expect_eq $? 0 || return 1
    sctl status wants.service
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'asdaasd' || return 1
    echo > "${SYSMST_LOG}"
    sctl stop wants.service
    expect_eq $? 0 || return 1

    # clean
    kill -9 "${sysmaster_pid}"
}

# usage: test dependency inactive
function test02() {
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    # Requires: dependency inactive leads to inactive
    sctl start requires
    expect_eq $? 0 || return 1
    check_status requires active
    expect_eq $? 0 || return 1
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    check_status requires inactive
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'asdaasd' || return 1
    echo > "${SYSMST_LOG}"

    # Wants: stay active when dependency inactive leads to inactive
    sctl start wants
    expect_eq $? 0 || return 1
    check_status wants active
    expect_eq $? 0 || return 1
    check_status base active
    expect_eq $? 0 || return 1
    sctl stop base
    check_status wants active
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'asdaasd' || return 1

    # clean
    kill -9 "${sysmaster_pid}"
}

# usage: test conflict dependency
function test03() {
    run_sysmaster || return 1

    sctl start base
    check_status base active
    expect_eq $? 0 || return 1

    sctl start conflicts
    check_status conflicts active
    expect_eq $? 0 || return 1
    check_status base inactive
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'asdaasd' || return 1
    echo > "${SYSMST_LOG}"

    sctl start base
    check_status conflicts active
    expect_eq $? 0 || return 1
    check_status conflicts inactive
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'asdaasd' || return 1

    # clean
    kill -9 "${sysmaster_pid}"
}

# usage: test contradictory dependency
function test04() {
    sed -i "/Conflicts/a Requires=\"base.service\"" ${SYSMST_LIB_PATH}/conflicts.service
    run_sysmaster || return 1

    sctl start conflicts
    expect_eq $? 1 || return 1
    check_status conflicts inactive
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'asdaasd' || return 1

    # clean
    kill -9 "${sysmaster_pid}"
}

# usage: test loop dependency
function test05() {
    sed -i "/Description/a Requires=\"requires.service\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start requires
    check_status requires active
    expect_eq $? 0 || return 1
    check_status base active
    expect_eq $? 0 || return 1
    check_log "${SYSMST_LOG}" 'asdaasd' || return 1

    # clean
    kill -9 "${sysmaster_pid}"
}

test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
test05 || exit 1
