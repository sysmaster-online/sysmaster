#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test enable unit without [Install]
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    sctl enable base &> log
    expect_eq $? 0
    check_log log 'The unit files have no installation config'
    ls ${SYSMST_ETC_PATH} && find ${SYSMST_ETC_PATH} -name base.service | grep base.service
    expect_ne $? 0

    # clean
    rm -rf log
    kill_sysmaster
}

# usage: test WantedBy
function test02() {
    log_info "===== test02 ====="
    cp -arf ${SYSMST_LIB_PATH}/base.service ${SYSMST_LIB_PATH}/wantedby.service
    cp -arf ${SYSMST_LIB_PATH}/base.service ${SYSMST_LIB_PATH}/requiredby.service
    echo "\[Install]" >> ${SYSMST_LIB_PATH}/base.service
    echo "WantedBy=\"wantedby.service\"" >> ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl enable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/system/wantedby.service.wants/base.service
    expect_eq $? 0

    sctl start wantedby
    check_status wantedby active
    check_status base active
    sctl stop base
    check_status base inactive
    check_status wantedby active

    sctl disable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/system/wantedby.service.wants/base.service
    expect_eq $? 2
    sctl start wantedby
    check_status wantedby active
    check_status base inactive

    # clean
    sctl stop wantedby
    kill_sysmaster
}

# usage: test RequiredBy
function test03() {
    log_info "===== test03 ====="
    sed -i "s/WantedBy=.*/RequiredBy=\"requiredby.service\"/" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl enable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/system/requiredby.service.wants/base.service
    expect_eq $? 0

    sctl start requiredby
    check_status requiredby active
    check_status requiredby active
    sctl stop base
    check_status base inactive
    check_status requiredby inactive

    sctl disable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/system/requiredby.service.wants/base.service
    expect_eq $? 2
    sctl start requiredby
    check_status requiredby active
    check_status base inactive

    # clean
    sctl stop requiredby
    kill_sysmaster
}

# usage: test multiple Also
function test04() {
    log_info "===== test04 ====="
    cp -arf ${SYSMST_LIB_PATH}/wantedby.service ${SYSMST_LIB_PATH}/wantedby2.service
    echo "Also=\"wantedby.service wantedby2.service\"" >> ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl enable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/system/requiredby.service.wants/{wantedby.service,wantedby2.service}
    expect_eq $? 0
    sctl disable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/system/requiredby.service.wants | grep base
    expect_eq $? 1

    # clean
    kill_sysmaster
}

# usage: test multiple Alias
function test05() {
    log_info "===== test05 ====="
    sed -i '/Also/d' ${SYSMST_LIB_PATH}/base.service
    echo "Alias=\"base1.service base2.service\"" >> ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl enable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/system/requiredby.service.wants/{base1.service,base2.service}
    expect_eq $? 0
    sctl start base1
    expect_eq $? 0
    check_status base active
    check_status base1 active
    check_status base2 active
    sctl stop base
    expect_eq $? 0
    check_status base inactive
    check_status base1 inactive
    check_status base2 inactive

    sctl disable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/system/requiredby.service.wants | grep base
    expect_eq $? 1
    sctl start base1 &> log
    expect_eq $? 1
    check_log log 'No such file'
    sctl start base
    check_status base active
    sctl status base2 | grep 'No such file'
    expect_eq $? 0 || sctl status base2

    # clean
    rm -rf log
    kill_sysmaster
}

# usage: test duplicate Alias
function test06() {
    log_info "===== test06 ====="
    echo "Alias=\"base1.service base3.service base4\"" >> ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl enable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/system/requiredby.service.wants/{base1.service,base2.service,base3.service}
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/system/requiredby.service.wants | grep base4
    expect_eq $? 1

    sctl disable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/system/requiredby.service.wants | grep base
    expect_eq $? 1

    # clean
    kill_sysmaster
}

test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
test05 || exit 1
test06 || exit 1
exit "${EXPECT_FAIL}"
