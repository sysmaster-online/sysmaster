#!/bin/bash
# Description: test frame log functions

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
