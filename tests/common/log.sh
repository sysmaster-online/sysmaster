#!/bin/bash
# Desciption: test fame log functions

function log_info() {
    echo "[$(date +"%F %T")] [  INFO ] $*"
}

function log_wan() {
    echo -e "\033[33m[$(date +"%F %T")] [WARNING] $* \033[0m"
}

function log_eo() {
    echo -e "\033[31m[$(date +"%F %T")] [ ERROR ] $* \033[0m"
}

function log_debug() {
    echo "[$(date +"%F %T")] [ DEBUG ] $*"
    echo -n ""
}
