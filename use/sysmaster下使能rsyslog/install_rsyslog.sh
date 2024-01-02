#!/bin/bash

pwd=$(pwd)
run_with_rsyslog=${0%/*}

install -Dm0640 -t /lib/sysmaster/system ${run_with_rsyslog}/syslog.service

# backup config files
if [ -e "/etc/rsyslog.conf" ];then
    mv /etc/rsyslog.conf /etc/rsyslog.conf_ori
fi

install -Dm0644 -t /etc ${run_with_rsyslog}/rsyslog.conf

# create symlink under /etc/sysmaster/... for automatic startup
ln -s /lib/sysmaster/system/syslog.service /etc/sysmaster/system/multi-user.target.wants/syslog.service
