#!/usr/bin/env bash

export EXPECT_FAIL=0
export SYSMST_LIB_PATH='/usr/lib/sysmaster/system'
export SYSMST_ETC_PATH='/etc/sysmaster/system'
export SYSMST_RUN_PATH='/run/sysmaster/system'
export SYSMST_LOG='/opt/sysmaster.log'
[ "${DOCKER_TEST}" -ne 1 ] && SYSMST_LOG='/var/log/messages'
if ! test -d /usr/lib/sysmaster/system; then
    SYSMST_LIB_PATH='/usr/lib/sysmaster'
    SYSMST_ETC_PATH='/etc/sysmaster'
    SYSMST_RUN_PATH='/run/sysmaster'
fi
export SYSMST_PATH='/usr/lib/sysmaster'
export RELIAB_SWITCH_PATH='/run/sysmaster/reliability'
export RELIAB_SWITCH='switch.debug'
export RELIAB_CLR='clear.debug'
export init_pid=''
export sysmaster_pid=''
export cond_fail_log='The condition check failed, not starting'
export asst_fail_log='Starting failed .* assert test failed'

# ================== log function ==================
function log_info() {
    echo "[$(date +"%F %T")] [  INFO ] $*"
}

function log_warn() {
    echo -e "\033[33m[$(date +"%F %T")] [WARNING] $* \033[0m"
}

function log_error() {
    echo -e "\033[31m[$(date +"%F %T")] [ ERROR ] $* \033[0m"
}

function log_debug() {
    echo "[$(date +"%F %T")] [ DEBUG ] $*"
    echo -n ""
}

# ================== assert function ==================
function get_file_line() {
    echo "$(basename "${BASH_SOURCE[2]}")": ${BASH_LINENO[1]}
}

function add_failure() {
    local msg=${1:-}

    ((++EXPECT_FAIL))
    log_error "add_failure(msg=${msg}) - $(get_file_line)"
    return 1
}

function expect_eq() {
    local actual=${1:-1}
    local expect=${2:-0}
    local msg=${3:-}

    [ "${actual}" -eq "${expect}" ] && return 0
    ((++EXPECT_FAIL))
    log_error "expect_eq(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

function expect_ne() {
    local actual=${1:-1}
    local expect=${2:-1}
    local msg=${3:-}

    [ "${actual}" -ne "${expect}" ] && return 0
    ((++EXPECT_FAIL))
    log_error "expect_ne(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

function expect_gt() {
    local actual=${1:-0}
    local expect=${2:-1}
    local msg=${3:-}

    [ "${actual}" -gt "${expect}" ] && return 0
    ((++EXPECT_FAIL))
    log_error "expect_gt(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

function expect_ge() {
    local actual=${1:-0}
    local expect=${2:-1}
    local msg=${3:-}

    [ "${actual}" -ge "${expect}" ] && return 0
    ((++EXPECT_FAIL))
    log_error "expect_ge(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

function expect_lt() {
    local actual=${1:-1}
    local expect=${2:-0}
    local msg=${3:-}

    [ "${actual}" -lt "${expect}" ] && return 0
    ((++EXPECT_FAIL))
    log_error "expect_lt(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

function expect_le() {
    local actual=${1:-1}
    local expect=${2:-0}
    local msg=${3:-}

    [ "${actual}" -le "${expect}" ] && return 0
    ((++EXPECT_FAIL))
    log_error "expect_le(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

function expect_str_eq() {
    local actual=$1
    local expect=$2
    local msg=$3

    [ "${actual}" = "${expect}" ] && return 0
    ((++EXPECT_FAIL))
    log_error "expect_str_eq(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

# ================== util ==================
# usage: run sysmaster as daemon
function run_sysmaster() {
    local user="${1:-root}"

    init_pid="$(ps -elf | grep -v grep | grep " $(echo ${user} | cut -c1-7)" | grep -w "${SYSMST_PATH}"/init | awk '{print $4}')"
    if [ -z "${init_pid}" ]; then
        if [ "${user}" = root ]; then
            RUST_BACKTRACE=full "${SYSMST_PATH}"/init &> "${SYSMST_LOG}" &
        else
            chmod 777 "${SYSMST_PATH}"/init
            chmod -R 777 "${SYSMST_PATH}" /run
            chmod -R 777 "$(dirname "${SYSMST_LOG}")"
            sudo -u "${user}" "${SYSMST_PATH}"/init &> "${SYSMST_LOG}" &
        fi
        init_pid=$!
    fi

    # wait sysmaster init done
    sleep 3
    for ((i = 0; i < 300; ++i)); do
        sleep 1
        ps -elf | grep -v grep | grep " $(echo ${user} | cut -c1-7)" | grep -w "${SYSMST_PATH}"/sysmaster | grep ep_pol && break
    done
    # debug
    ps -elf | grep -v grep | grep -Ew 'sysmaster|init'
    if ! ps -elf | grep -v grep | grep " $(echo ${user} | cut -c1-7)" | grep -wq "${SYSMST_PATH}"/sysmaster; then
        cat "${SYSMST_LOG}"
        return 1
    fi

    # get sysmaster pid
    sysmaster_pid="$(ps -elf | grep -v grep | grep " $(echo ${user} | cut -c1-7)" | grep -w "${SYSMST_PATH}"/sysmaster | awk '{print $4}')"
    echo > "${SYSMST_LOG}"
    return 0
}

# usage: kill sysmaster and init
function kill_sysmaster() {
    [ -n "${init_pid}" ] && kill -9 "${init_pid}"
    [ -n "${sysmaster_pid}" ] && kill -9 "${sysmaster_pid}"
}

# usage: check log info.
# input: $1: log file to check
#        $2: key log info (mandatory)
#        $3 ~ $N: key log info (optional)
# output: null
function check_log() {
    local file_name="$1"

    # debug
    sync
    sleep 1
    cat "${file_name}" | sed "s/\x00//g" || return 1

    shift 1
    while [ $# -gt 0 ]; do
        cat "${file_name}" | sed "s/\x00//g" | grep -aE "$1" || return 1
        shift 1
    done
}

# usage: check unit status
# input: $1: unit name
#        $2: expect status
function check_status() {
    local service="$1"
    local exp_status="$2"

    sleep 0.1
    for ((cnt = 0; cnt < 3; ++cnt)); do
        sctl status "${service}" 2>&1 |& grep -w 'Active:' | head -n1 | grep -w "${exp_status}" && return 0 || sleep 1
    done
    # debug
    sctl status "${service}"
    return 1
}

# usage: check unit load status
# input: $1: unit name
#        $2: expect load status
function check_load() {
    local service="$1"
    local exp_status="$2"

    sleep 0.1
    for ((cnt = 0; cnt < 3; ++cnt)); do
        sctl status "${service}" 2>&1 |& grep -w 'Loaded:' | head -n1 | awk '{print $2}' | grep -w "${exp_status}" && return 0 || sleep 1
    done
    # debug
    sctl status "${service}"
    return 1
}

# usage: get unit pids
# input: $1: unit name
function get_pids() {
    local service="$1"

    sctl status "${service}" 2>&1 |& sed -n '/PID:/,$p' | sed 's/PID://' | awk '{print $1}'
}

# usage: check netstat
# input: $1: path
#        $2: type
function check_netstat() {
    local path="$1"
    local type="$2"

    netstat -nap | grep -w "${path}" | grep -w "${type}" || return 1
}
