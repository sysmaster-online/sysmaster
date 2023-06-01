#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

## usage: background daemon-reload
function stress() {
    while ((1)); do
        sleep 600
        sctl daemon-reload &
    done
}

## usage: check unit function
function check_start_fun() {
    local index=$1

    while ((1)); do
        sctl restart reliable_${index}01
        expect_eq $? 0
        check_status reliable_${index}01 active
        expect_eq $? 0

        sctl stop reliable_${index}01
        expect_eq $? 0
        check_status reliable_${index}01.service inactive
        expect_eq $? 0
    done
}

## usage: check unit function
function check_enable_fun() {
    local index=$1

    while ((1)); do
        sctl enable reliable_${index}02
        expect_eq $? 0
        sctl start base${index}
        expect_eq $? 0
        check_status base${index} active
        expect_eq $? 0
        check_status reliable_${index}02 active
        expect_eq $? 0

        sctl stop reliable_${index}02
        expect_eq $? 0
        check_status base${index} inactive
        expect_eq $? 0
        check_status reliable_${index}02 inactive
        expect_eq $? 0

        sctl disable reliable_${index}02
        expect_eq $? 0
        sctl start base${index}
        expect_eq $? 0
        check_status base${index} active
        expect_eq $? 0
        # check_status reliable_${index}02 inactive
        # expect_eq $? 0

        # sctl stop base${index}
        sctl stop base${index} reliable_${index}02
        expect_eq $? 0
        check_status base${index} inactive
        expect_eq $? 0
        check_status reliable_${index}02 inactive
        expect_eq $? 0
    done
}

cp -arf "${work_dir}"/tmp_units/*.service ${SYSMST_LIB_PATH} || exit 1
sed -i '/Description/ a StartLimitBurst=0' ${SYSMST_LIB_PATH}/base.service
sed -i '/Description/ a StartLimitBurst=0' ${SYSMST_LIB_PATH}/reliable_001.service
sed -i '/Description/ a StartLimitBurst=0' ${SYSMST_LIB_PATH}/reliable_002.service
for ((i=1; i<10; ++i)); do
    cp -arf ${SYSMST_LIB_PATH}/base.service ${SYSMST_LIB_PATH}/base${i}.service
    cp -arf ${SYSMST_LIB_PATH}/reliable_001.service ${SYSMST_LIB_PATH}/reliable_${i}01.service
    cp -arf ${SYSMST_LIB_PATH}/reliable_002.service ${SYSMST_LIB_PATH}/reliable_${i}02.service
    sed -i "s/base/base${i}/" ${SYSMST_LIB_PATH}/reliable_${i}02.service
done
sed -i "s/base/base0/" ${SYSMST_LIB_PATH}/reliable_002.service
mv ${SYSMST_LIB_PATH}/base.service ${SYSMST_LIB_PATH}/base0.service

run_sysmaster || exit 1
sh -x /opt/monitor.sh &> /dev/null &
# for ((i=0; i<100; ++i)); do
    stress &> /dev/null &
#     pid_list="${pid_list} $!"
# done

for ((i=0; i<10; ++i)); do
    check_start_fun $i &> /opt/check_start_fun_${i}.log &
    check_enable_fun $i &> /opt/check_enable_fun_${i}.log &
done

wait
# kill -9 ${pid_list}
exit "${EXPECT_FAIL}"
