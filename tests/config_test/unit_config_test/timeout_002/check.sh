#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

startpre_log="startpre operation time out. enter StopSigterm"
startpost_log="StartPost operation time out. enter StopSigterm"
stop_log="Stop operation time out. enter FinalSigterm"
stoppost_log="StopPost operation time out. enter FinalSigterm"

# usage: test TimeoutSec
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/timeout.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Service/ a TimeoutSec=0' ${SYSMST_LIB_PATH}/timeout.service
    sctl daemon-reload
    # TimeoutSec=0 means infinity
    sctl start timeout &
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpre)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpost)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'active (running)'
    expect_eq $? 0
    main_pid="$(get_pids timeout)"
    ps -elf | grep -v grep | grep 'sleep 100' | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 0

    sctl stop timeout &
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stop)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stop)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stoppost)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'inactive (dead)'
    expect_eq $? 0

    # TimeoutSec=1, start-pre and stop-post both timeout
    sed -i 's/TimeoutSec=.*/TimeoutSec=1/' ${SYSMST_LIB_PATH}/timeout.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    sctl start timeout &
    sleep 0.9
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpre)'
    expect_eq $? 0
    sleep 0.2
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stoppost)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'failed'
    expect_eq $? 0
    check_log "${SYSMST_LOG}" "${startpre_log}" "${stoppost_log}"
    expect_eq $? 0
    grep -aE "${startpost_log}|${stop_log}" "${SYSMST_LOG}"

    # TimeoutSec=3, no timeout
    sed -i 's/TimeoutSec=.*/TimeoutSec=3/' ${SYSMST_LIB_PATH}/timeout.service
    sctl daemon-reload
    # TimeoutSec > ExecStartPre + ExecStartPost, no timeout
    sctl start timeout &
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpre)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpost)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'active (running)'
    expect_eq $? 0
    main_pid="$(get_pids timeout)"
    ps -elf | grep -v grep | grep 'sleep 100' | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 0
    # TimeoutSec > single ExecStop, TimeoutSec > ExecStopPost, no timeout
    sctl stop timeout &
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stop)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stop)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stoppost)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'inactive (dead)'
    expect_eq $? 0

    # TimeoutSec=2, only start-post timeout
    sed -i 's/TimeoutSec=.*/TimeoutSec=2/' ${SYSMST_LIB_PATH}/timeout.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    # TimeoutSec < ExecStartPre + ExecStartPost, timeout
    # TimeoutSec > single ExecStop, TimeoutSec > ExecStopPost, no timeout
    sctl start timeout &
    sleep 0.9
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpre)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpost)'
    expect_eq $? 0
    sleep 0.2
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stoppost)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'failed'
    expect_eq $? 0
    check_log "${SYSMST_LOG}" "${startpost_log}"
    expect_eq $? 0
    grep -aE "${startpre_log}|${stop_log}|${stoppost_log}" "${SYSMST_LOG}"
    expect_eq $? 1

    # TimeoutSec=1, only start-pre timeout
    sed -i 's/ExecStopPost=.*/ExecStopPost=/bin/sleep 0.5/' ${SYSMST_LIB_PATH}/timeout.service
    sed -i 's/TimeoutSec=.*/TimeoutSec=1/' ${SYSMST_LIB_PATH}/timeout.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    sctl start timeout &
    sleep 0.9
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpre)'
    expect_eq $? 0
    sleep 0.2
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stoppost)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'failed'
    expect_eq $? 0
    check_log "${SYSMST_LOG}" "${startpre_log}"
    expect_eq $? 0
    grep -aE "${startpost_log}|${stop_log}|${stoppost_log}" "${SYSMST_LOG}"
    expect_eq $? 1
}

# usage: test TimeoutStartSec/TimeoutStopSec
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/timeout.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Service/ a TimeoutStartSec=0' ${SYSMST_LIB_PATH}/timeout.service
    sed -i '/Service/ a TimeoutStopSec=0' ${SYSMST_LIB_PATH}/timeout.service
    sctl daemon-reload
    # TimeoutStartSec/TimeoutStopSec=0 means infinity
    sctl start timeout &
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpre)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpost)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'active (running)'
    expect_eq $? 0
    main_pid="$(get_pids timeout)"
    ps -elf | grep -v grep | grep 'sleep 100' | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 0

    sctl stop timeout &
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stop)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stop)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stoppost)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'inactive (dead)'
    expect_eq $? 0

    # TimeoutStartSec/TimeoutStopSec have higher priority than TimeoutSec
    # only stop timeout (the first ExecStop + ExecStopPost)
    sed -i '/ExecStartPre/ i TimeoutSec=0' ${SYSMST_LIB_PATH}/timeout.service
    sed -i 's/TimeoutStartSec=.*/TimeoutStartSec=3/' ${SYSMST_LIB_PATH}/timeout.service
    sed -i 's/TimeoutStopSec=.*/TimeoutStopSec=1/' ${SYSMST_LIB_PATH}/timeout.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    # TimeoutStartSec > ExecStartPre + ExecStartPost, no timeout
    sctl start timeout &
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpre)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpost)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'active (running)'
    expect_eq $? 0
    main_pid="$(get_pids timeout)"
    ps -elf | grep -v grep | grep 'sleep 100' | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 0
    # TimeoutStopSec < the first ExecStop, timeout
    # ExecStopPost will be executed anyway
    # TimeoutStopSec = ExecStopPost, timeout again
    sctl stop timeout &
    sleep 0.9
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stop)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stoppost)'
    sleep 0.2
    sctl status timeout
    sctl status timeout 2>&1 | grep 'failed'
    expect_eq $? 0
    check_log "${SYSMST_LOG}" "${stop_log}" "${stoppost_log}"
    expect_eq $? 0
    grep -aE "${startpre_log}|${startpost_log}" "${SYSMST_LOG}"
    expect_eq $? 1

    # only stop timeout (the second ExecStop)
    sed -i 's/ExecStop=.*/ExecStop="/bin/sleep 0.9 ; /bin/sleep 1.5"/' ${SYSMST_LIB_PATH}/timeout.service
    sed -i 's/ExecStopPost=.*/ExecStopPost="/bin/sleep 0.9"/' ${SYSMST_LIB_PATH}/timeout.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    # TimeoutStartSec > ExecStartPre + ExecStartPost, no timeout
    sctl start timeout &
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpre)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpost)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'active (running)'
    expect_eq $? 0
    main_pid="$(get_pids timeout)"
    ps -elf | grep -v grep | grep 'sleep 100' | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 0
    # TimeoutStopSec > the first ExecStop, no timeout
    # TimeoutStopSec < the second ExecStop, timeout
    # ExecStopPost will be executed anyway
    # TimeoutStopSec > ExecStopPost, no timeout
    sctl stop timeout &
    sleep 0.8
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stop)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stop)'
    expect_eq $? 0
    sleep 0.9
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stoppost)'
    expect_eq $? 0
    sleep 0.2
    sctl status timeout
    sctl status timeout 2>&1 | grep 'failed'
    expect_eq $? 0
    check_log "${SYSMST_LOG}" "${stop_log}"
    expect_eq $? 0
    grep -aE "${startpre_log}|${startpost_log}|${stoppost_log}" "${SYSMST_LOG}"
    expect_eq $? 1

    # only stop-post timeout
    sed -i 's/ExecStop=.*/ExecStop=/bin/sleep 0.9 ; /bin/sleep 0.9/' ${SYSMST_LIB_PATH}/timeout.service
    sed -i 's/ExecStopPost=.*/ExecStopPost=/bin/sleep 1.5/' ${SYSMST_LIB_PATH}/timeout.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    # TimeoutStartSec > ExecStartPre + ExecStartPost, no timeout
    sctl start timeout &
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpre)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpost)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'active (running)'
    expect_eq $? 0
    main_pid="$(get_pids timeout)"
    ps -elf | grep -v grep | grep 'sleep 100' | awk '{print $4}' | grep -w "${main_pid}"
    expect_eq $? 0
    # TimeoutStopSec > the first ExecStop, no timeout
    # TimeoutStopSec > the second ExecStop, no timeout
    # TimeoutStopSec < ExecStopPost, timeout
    sctl stop timeout &
    sleep 0.8
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stop)'
    expect_eq $? 0
    sleep 0.9
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stop)'
    expect_eq $? 0
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stoppost)'
    expect_eq $? 0
    sleep 0.2
    sctl status timeout
    sctl status timeout 2>&1 | grep 'failed'
    expect_eq $? 0
    check_log "${SYSMST_LOG}" "${stoppost_log}"
    expect_eq $? 0
    grep -aE "${startpre_log}|${startpost_log}|${stop_log}" "${SYSMST_LOG}"
    expect_eq $? 1

    # only start timeout
    sed -i 's/TimeoutStartSec=.*/TimeoutStartSec=2/' ${SYSMST_LIB_PATH}/timeout.service
    sed -i 's/TimeoutStopSec=.*/TimeoutStopSec=2/' ${SYSMST_LIB_PATH}/timeout.service
    sctl daemon-reload
    echo > "${SYSMST_LOG}"
    sctl start timeout &
    sleep 1
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpre)'
    expect_eq $? 0
    sleep 0.9
    sctl status timeout
    sctl status timeout 2>&1 | grep 'activating (startpost)'
    expect_eq $? 0
    sleep 0.2
    sctl status timeout
    sctl status timeout 2>&1 | grep 'deactivating (stoppost)'
    expect_eq $? 0
    sleep 1.5
    sctl status timeout
    sctl status timeout 2>&1 | grep 'failed'
    expect_eq $? 0
    check_log "${SYSMST_LOG}" "${startpost_log}"
    expect_eq $? 0
    grep -aE "${startpre_log}|${stop_log}|${stoppost_log}" "${SYSMST_LOG}"
    expect_eq $? 1
}

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
exit "${EXPECT_FAIL}"
