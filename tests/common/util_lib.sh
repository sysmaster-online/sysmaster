#!/bin/bash

SYSMST_LIB_PATH='/usr/lib/sysmaster'
SYSMST_LOG='/opt/sysmaster.log'

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

function expect_eq() {
    local actual=${1:-1}
    local expect=${2:-0}
    local msg=${3:-}

    [ "${actual}" -eq "${expect}" ] && return 0
    log_error "expect_eq(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

function expect_ne() {
    local actual=${1:-1}
    local expect=${2:-1}
    local msg=${3:-}

    [ "${actual}" -ne "${expect}" ] && return 0
    log_error "expect_ne(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

function expect_gt() {
    local actual=${1:-0}
    local expect=${2:-1}
    local msg=${3:-}

    [ "${actual}" -gt "${expect}" ] && return 0
    log_error "expect_gt(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

function expect_ge() {
    local actual=${1:-0}
    local expect=${2:-1}
    local msg=${3:-}

    [ "${actual}" -ge "${expect}" ] && return 0
    log_error "expect_ge(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

function expect_lt() {
    local actual=${1:-1}
    local expect=${2:-0}
    local msg=${3:-}

    [ "${actual}" -lt "${expect}" ] && return 0
    log_error "expect_lt(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

function expect_le() {
    local actual=${1:-1}
    local expect=${2:-0}
    local msg=${3:-}

    [ "${actual}" -le "${expect}" ] && return 0
    log_error "expect_le(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

function expect_str_eq() {
    local actual=$1
    local expect=$2
    local msg=$3

    [ "${actual}" = "${expect}" ] && return 0
    log_error "expect_str_eq(${actual}, ${expect}, msg=${msg}) - $(get_file_line)"
    return 1
}

# ================== util ==================
# usage: run sysmaster as daemon
function run_sysmaster() {
    /usr/lib/sysmaster/sysmaster &> "${SYSMST_LOG}" &
    sysmaster_pid=$!
    # wait sysmaster init done
    sleep 3
    ps aux | grep -v grep | grep sysmaster | grep -w "${sysmaster_pid}" && return 0
    cat "${SYSMST_LOG}"
    return 1
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
    cat "${file_name}" || return 1

    shift 1
    expect_gt $# 0 'Parameter missing: key log info not defined!' || return 1
    while [ $# -gt 0 ]; do
        grep -aE "$1" "${file_name}"
        expect_eq $? 0 "check log failed, '$1' not found in ${file_name}!" || return 1
        shift 1
    done
}

# usage: check unit status/state
# input: $1: unit name
#        $2: expect status/state
function check_status() {
    local service="$1"
    local exp_status="$2"

    for ((cnt = 0; cnt < 3; ++cnt)); do
        sctl status "${service}"
        sctl status "${service}" | grep -w 'Active:' | head -n1 | awk '{print $2}' | grep -qw "${exp_status}" && return 0 || sleep 1
    done
    sctl status "${service}"
    return 1
}
