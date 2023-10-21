#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test Environment
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/env.service ${SYSMST_LIB_PATH} || return 1
    sctl daemon-reload

    sctl restart env
    check_status env active
    expect_eq $? 0 || return 1
    main_pid="$(get_pids env)"
    cat /proc/"${main_pid}"/environ > log
    check_log log "VAR0=word;0" "VAR1=word1" "VAR2=word1" "VAR3=\"word 3\"" "VAR4=word 4" "VAR5=word=5"
    expect_eq $? 0

    # clean
    rm -rf log
    sctl stop env
    check_status env inactive
    expect_eq $? 0 || return 1
}

# usage: test EnvironmentFile
function test02() {
    log_info "===== test02 ====="
    sed -i '/Environment=/ a EnvironmentFile=/opt/env1 /opt/env2 -/opt/env3' ${SYSMST_LIB_PATH}/env.service
    sctl daemon-reload

    sctl restart env
    check_status env active
    expect_eq $? 0 || return 1
    main_pid="$(get_pids env)"
    cat /proc/"${main_pid}"/environ > log
    check_log log "VAR0=word;0" "VAR1=1" "VAR2=word1" "VAR3==== word3 ===" "VAR4=word 4" "VAR5=word=5" "VAR6=66" "VAR7=7" "VAR8=8"
    expect_eq $? 0
    grep VAR9 log
    expect_eq $? 1

    # clean
    rm -rf log
    sctl stop env
    check_status env inactive
    expect_eq $? 0 || return 1
}

cat << EOF > /opt/env1
VAR1=1
# VAR2=2
VAR3="=== word3 ==="

VAR6=6
VAR7=7
EOF

cat << EOF > /opt/env2
# VAR1=11
VAR6=66
VAR8=8
EOF

cat << EOF > /opt/env3
VAR1=111
VAR2=222
VAR9=9
EOF

cat /opt/env1
cat /opt/env2
cat /opt/env3

run_sysmaster || exit 1
test01 || exit 1
test02 || exit 1
rm -rf /opt/env1 /opt/env2 /opt/env3
exit "${EXPECT_FAIL}"
