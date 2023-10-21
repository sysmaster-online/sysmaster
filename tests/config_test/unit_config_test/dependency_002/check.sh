#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test enable unit without [Install]
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    cp -arf ${SYSMST_LIB_PATH}/base.service ${SYSMST_LIB_PATH}/wantedby.service || return 1
    cp -arf ${SYSMST_LIB_PATH}/base.service ${SYSMST_LIB_PATH}/requiredby.service || return 1
    sctl daemon-reload
    sctl enable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}
    expect_eq $? 2

    # dropin path config
    echo "[Install]
WantedBy=wantedby.service" >> ${SYSMST_LIB_PATH}/base.service
    mkdir -p ${SYSMST_ETC_PATH}/base.service.d
    echo "[Install]
WantedBy=requiredby.service" > ${SYSMST_ETC_PATH}/base.service.d/install.conf
    sctl daemon-reload
    sctl enable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/requiredby.service.wants/base.service
    expect_eq $? 0 || return 1
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base.service
    expect_eq $? 0
    expect_str_eq "$(realpath ${SYSMST_ETC_PATH}/requiredby.service.wants/base.service)" "${SYSMST_LIB_PATH}/base.service"
    expect_str_eq "$(realpath ${SYSMST_ETC_PATH}/wantedby.service.wants/base.service)" "${SYSMST_LIB_PATH}/base.service"
    sctl disable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/requiredby.service.wants/base.service || ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base.service
    expect_eq $? 2
    ls ${SYSMST_ETC_PATH}/requiredby.service.wants ${SYSMST_ETC_PATH}/wantedby.service.wants
    expect_eq $? 0

    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH}
    sctl daemon-reload
    sctl enable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/requiredby.service.wants/base.service
    expect_eq $? 0 || return 1
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base.service
    expect_eq $? 2
    expect_str_eq "$(realpath ${SYSMST_ETC_PATH}/requiredby.service.wants/base.service)" "${SYSMST_LIB_PATH}/base.service"
    sctl disable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/requiredby.service.wants/base.service || ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base.service
    expect_eq $? 2
    ls ${SYSMST_ETC_PATH}/requiredby.service.wants ${SYSMST_ETC_PATH}/wantedby.service.wants
    expect_eq $? 0

    rm -rf ${SYSMST_ETC_PATH}/base.service.d ${SYSMST_ETC_PATH}/wantedby.service.wants ${SYSMST_ETC_PATH}/requiredby.service.wants
    sctl daemon-reload
}

# usage: test WantedBy
function test02() {
    log_info "===== test02 ====="
    echo "[Install]
WantedBy=wantedby.service" >> ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl enable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base.service
    expect_eq $? 0

    sctl start wantedby
    check_status wantedby active
    expect_eq $? 0
    check_status base active
    expect_eq $? 0
    sctl stop base
    check_status base inactive
    expect_eq $? 0
    check_status wantedby active
    expect_eq $? 0

    sctl disable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base.service
    expect_eq $? 2
    sctl start wantedby
    check_status wantedby active
    expect_eq $? 0
    check_status base inactive
    expect_eq $? 0

    # clean
    sctl stop wantedby
}

# usage: test RequiredBy
function test03() {
    log_info "===== test03 ====="
    sed -i "s/WantedBy=.*/RequiredBy=requiredby.service/" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl enable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/requiredby.service.requires/base.service
    expect_eq $? 0

    sctl start requiredby
    check_status requiredby active
    expect_eq $? 0
    check_status requiredby active
    expect_eq $? 0
    sctl stop base
    check_status base inactive
    expect_eq $? 0
    check_status requiredby inactive
    expect_eq $? 0

    sctl disable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/requiredby.service.requires/base.service
    expect_eq $? 2
    sctl start requiredby
    check_status requiredby active
    expect_eq $? 0
    check_status base inactive
    expect_eq $? 0

    # clean
    sctl stop requiredby
}

# usage: test multiple Also
function test04() {
    log_info "===== test04 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH}
    cp -arf ${SYSMST_LIB_PATH}/base.service ${SYSMST_LIB_PATH}/base1.service
    cp -arf ${SYSMST_LIB_PATH}/base.service ${SYSMST_LIB_PATH}/base2.service
    echo "[Install]
Also=base1.service base2.service
WantedBy=wantedby.service" >> ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl enable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base.service
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base1.service
    expect_eq $? 2
    sctl disable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base.service
    expect_eq $? 2

    echo "[Install]
WantedBy=requiredby.service" >> ${SYSMST_LIB_PATH}/base1.service
    echo "[Install]
WantedBy=requiredby.service" >> ${SYSMST_LIB_PATH}/base2.service
    sctl daemon-reload
    sctl enable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base.service
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base1.service
    expect_eq $? 2
    ls ${SYSMST_ETC_PATH}/requiredby.service.wants/{base1.service,base2.service}
    expect_eq $? 0
    expect_str_eq "$(realpath ${SYSMST_ETC_PATH}/requiredby.service.wants/base1.service)" "${SYSMST_LIB_PATH}/base1.service"
    expect_str_eq "$(realpath ${SYSMST_ETC_PATH}/requiredby.service.wants/base2.service)" "${SYSMST_LIB_PATH}/base2.service"
    sctl disable base1
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base.service
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/requiredby.service.wants/base1.service
    expect_eq $? 2
    ls ${SYSMST_ETC_PATH}/requiredby.service.wants/base2.service
    expect_eq $? 0
    sctl disable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants | grep base
    expect_eq $? 1
    ls ${SYSMST_ETC_PATH}/requiredby.service.wants | grep base
    expect_eq $? 1
}

# usage: test multiple Alias
function test05() {
    log_info "===== test05 ====="
    sed -i '/Also/d' ${SYSMST_LIB_PATH}/base.service
    echo "Alias=base1.service base2.service" >> ${SYSMST_LIB_PATH}/base.service
    rm -rf ${SYSMST_LIB_PATH}/{base1.service,base2.service}
    sctl daemon-reload
    sctl enable base
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base.service
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants/base1.service
    expect_eq $? 2
    ls ${SYSMST_ETC_PATH}/requiredby.service.wants | grep base
    expect_eq $? 1
    ls ${SYSMST_ETC_PATH}/{base1.service,base2.service}
    expect_eq $? 0
    expect_str_eq "$(realpath ${SYSMST_ETC_PATH}/base1.service)" "${SYSMST_LIB_PATH}/base.service"
    expect_str_eq "$(realpath ${SYSMST_ETC_PATH}/base2.service)" "${SYSMST_LIB_PATH}/base.service"

    sctl restart base
    check_status base 'active'
    expect_eq $? 0
    check_status base1 'active'
    expect_eq $? 0
    check_status base2 'active'
    expect_eq $? 0
    sctl stop base
    check_status base 'inactive'
    expect_eq $? 0
    check_status base1 'inactive'
    expect_eq $? 0
    check_status base2 'inactive'
    expect_eq $? 0

    sctl disable base1
    expect_eq $? 0
    ls ${SYSMST_ETC_PATH}/wantedby.service.wants | grep base
    expect_eq $? 1
    ls ${SYSMST_ETC_PATH}/base1.service || ls ${SYSMST_ETC_PATH}/base2.service
    expect_eq $? 2
    sctl restart base
    check_status base 'active'
    expect_eq $? 0
    check_status base1 'inactive'
    expect_eq $? 0
    check_status base2 'inactive'
    expect_eq $? 0
    sctl stop base
    check_status base 'inactive'
    expect_eq $? 0
}

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
test05 || exit 1
exit "${EXPECT_FAIL}"
