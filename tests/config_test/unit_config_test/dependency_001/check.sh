#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test dependency not exist
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/{conflicts.service,requires.service,wants.service,requisite.service,partof.service,bindsto.service} ${SYSMST_LIB_PATH} || return 1
    sctl daemon-reload
    sctl status base.service 2>&1 &> log
    expect_ne $? 0
    check_log log 'Failed to show the status of base.service: NotExisted'
    expect_eq $? 0 || return 1
    rm -rf log

    # Requires: dependency not exist leads to start failure
    for serv in requires requisite bindsto; do
        sctl start ${serv}
        expect_ne $? 0 || return 1
        check_status ${serv}.service inactive
        expect_eq $? 0 || return 1
    done

    # Wants/Partof: start normally when dependency not exist
    for serv in wants partof; do
        sctl start ${serv}
        expect_eq $? 0 || return 1
        check_status ${serv} active
        expect_eq $? 0 || return 1
    done

    # clean
    sctl stop requires.service wants.service requisite.service partof.service bindsto.service
}

# usage: test dependency inactive
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sctl daemon-reload

    # Requires: dependency inactive leads to inactive
    sctl restart requires.service
    expect_eq $? 0 || return 1
    check_status requires.service active
    expect_eq $? 0 || return 1
    check_status base.service active
    expect_eq $? 0 || return 1
    sctl stop base.service
    check_status requires.service inactive
    expect_eq $? 0 || return 1

    # Requires: dependency finish or condition check failed
    sed -i 's/sleep 100/sleep 2/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart requires.service
    expect_eq $? 0 || return 1
    check_status requires.service active
    expect_eq $? 0 || return 1
    check_status base.service active
    expect_eq $? 0 || return 1
    sleep 2
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_status requires.service active
    expect_eq $? 0 || return 1
    sctl stop requires.service
    check_status requires.service inactive
    expect_eq $? 0 || return 1

    sed -i '/Description/a ConditionPathExists="/notexist"' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl restart requires.service
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_log ${SYSMST_LOG} ${cond_fail_log}
    expect_eq $? 0
    check_status requires.service active
    expect_eq $? 0 || return 1

    sed -i '/Description/a After="base.service"' ${SYSMST_LIB_PATH}/requires.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl restart requires.service
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_log ${SYSMST_LOG} ${cond_fail_log}
    expect_eq $? 0
    check_status requires.service active
    expect_eq $? 0 || return 1

    # Requisite: dependency inactive leads to inactive
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sctl daemon-reload
    sctl restart requisite.service
    expect_eq $? 0 || return 1
    check_status requisite.service inactive
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    sctl restart base.service
    check_status base.service active
    expect_eq $? 0 || return 1
    sctl restart requisite.service
    expect_eq $? 0 || return 1
    check_status requisite.service active
    expect_eq $? 0 || return 1
    sctl stop base.service
    check_status requisite.service inactive
    expect_eq $? 0 || return 1

    # Bindsto: dependency inactive
    sctl start bindsto.service
    expect_eq $? 0 || return 1
    check_status bindsto.service active
    expect_eq $? 0 || return 1
    check_status base.service active
    expect_eq $? 0 || return 1
    sctl stop base.service
    check_status bindsto.service inactive
    expect_eq $? 0 || return 1

    # Bindsto: dependency finish or condition check failed
    sed -i 's/sleep 100/sleep 2/' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart bindsto.service
    expect_eq $? 0 || return 1
    check_status bindsto.service active
    expect_eq $? 0 || return 1
    check_status base.service active
    expect_eq $? 0 || return 1
    sleep 2
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_status bindsto.service inactive
    expect_eq $? 0 || return 1

    sed -i '/Description/a ConditionPathExists="/notexist"' ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl restart bindsto.service
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_log ${SYSMST_LOG} ${cond_fail_log}
    expect_eq $? 0
    check_status bindsto.service inactive
    expect_eq $? 0 || return 1

    sed -i '/Description/a After="base.service"' ${SYSMST_LIB_PATH}/bindsto.service
    sctl daemon-reload
    echo > ${SYSMST_LOG}
    sctl restart bindsto.service
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_log ${SYSMST_LOG} ${cond_fail_log}
    expect_eq $? 0
    check_status bindsto.service inactive
    expect_eq $? 0 || return 1

    # Wants: stay active when dependency inactive leads to inactive
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sctl daemon-reload
    sctl restart wants.service
    expect_eq $? 0 || return 1
    check_status wants.service active
    expect_eq $? 0 || return 1
    check_status base.service active
    expect_eq $? 0 || return 1
    sctl stop base.service
    check_status wants.service active
    expect_eq $? 0 || return 1
    sctl stop wants.service

    # PartOf: only for dependency stop or restart
    sctl start partof.service
    expect_eq $? 0 || return 1
    check_status partof.service active
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1
    sctl start base.service
    check_status base.service active
    expect_eq $? 0 || return 1
    check_status partof.service active
    expect_eq $? 0 || return 1
    base_pid_1="$(get_pids base.service)"
    partof_pid_1="$(get_pids partof.service)"
    sctl restart base.service
    expect_eq $? 0 || return 1
    sleep 0.1
    check_status base.service active
    expect_eq $? 0 || return 1
    check_status partof.service active
    expect_eq $? 0 || return 1
    base_pid_2="$(get_pids base.service)"
    partof_pid_2="$(get_pids partof.service)"
    expect_gt "${base_pid_2}" "${base_pid_1}"
    expect_gt "${partof_pid_2}" "${partof_pid_1}"
    sctl restart partof.service
    expect_eq $? 0 || return 1
    sleep 0.1
    check_status partof.service active
    expect_eq $? 0 || return 1
    check_status base.service active
    expect_eq $? 0 || return 1
    expect_eq "$(get_pids base.service)" "${base_pid_2}"
    expect_gt "$(get_pids partof.service)" "${partof_pid_2}"
    sctl stop base.service
    check_status base.service inactive
    expect_eq $? 0 || return 1
    check_status partof.service inactive
    expect_eq $? 0 || return 1

    sctl stop base.service partof.service
}

# usage: test conflict dependency
function test03() {
    log_info "===== test03 ====="
    sctl daemon-reload
    sctl restart base.service
    check_status base.service active
    expect_eq $? 0 || return 1

    sctl start conflicts.service
    check_status conflicts.service active
    expect_eq $? 0 || return 1
    check_status base.service inactive
    expect_eq $? 0 || return 1

    sctl start base.service
    check_status base.service active
    expect_eq $? 0 || return 1
    check_status conflicts.service inactive
    expect_eq $? 0 || return 1

    # clean
    sctl stop conflicts.service
}

# usage: test contradictory dependency
function test04() {
    log_info "===== test04 ====="
    sed -i "/Conflicts=/a Requires=base.service" ${SYSMST_LIB_PATH}/conflicts.service
    sctl daemon-reload
    sctl restart conflicts.service &> log
    expect_eq $? 53
    cat log
    grep 'Failed to restart .*: Conflict' log
    expect_eq $? 0
    rm -rf log
    check_status conflicts.service inactive
    expect_eq $? 0 || return 1
}

# usage: test loop dependency
function test05() {
    log_info "===== test05 ====="
    sed -i "/Description/a Requires=requires.service" ${SYSMST_LIB_PATH}/base.service
    sctl daemon-reload
    sctl restart requires.service
    check_status requires.service active
    expect_eq $? 0 || return 1
    check_status base.service active
    expect_eq $? 0 || return 1

    # clean
    sctl stop base.service requires.service
}

# usage: test dependency restart
function test06() {
    log_info "===== test06 ====="

    # Requires: dependency restart leads to restart
    sctl start requires.service
    expect_eq $? 0 || return 1
    check_status requires.service active
    expect_eq $? 0 || return 1
    check_status base.service active
    expect_eq $? 0 || return 1
    base_pid="$(get_pids base.service)"
    requires_pid="$(get_pids requires.service)"
    sctl restart base.service
    expect_eq $? 0 || return 1
    sleep 0.1
    check_status base.service active
    expect_eq $? 0 || return 1
    check_status requires.service active
    expect_eq $? 0 || return 1
    expect_gt "$(get_pids base.service)" "${base_pid}"
    expect_gt "$(get_pids requires.service)" "${requires_pid}"

    # Wants: stay active when dependency restart
    sctl start wants.service
    expect_eq $? 0 || return 1
    check_status wants.service active
    expect_eq $? 0 || return 1
    check_status base.service active
    expect_eq $? 0 || return 1
    base_pid="$(get_pids base.service)"
    wants_pid="$(get_pids wants.service)"
    sctl restart base.service
    expect_eq $? 0 || return 1
    sleep 0.1
    check_status base.service active
    expect_eq $? 0 || return 1
    check_status wants.service active
    expect_eq $? 0 || return 1
    expect_gt "$(get_pids base.service)" "${base_pid}"
    expect_eq "$(get_pids wants.service)" "${wants_pid}"

    # clean
    sctl stop base.service requires.service wants.service
}

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
test05 || exit 1
test06 || exit 1
exit "${EXPECT_FAIL}"
