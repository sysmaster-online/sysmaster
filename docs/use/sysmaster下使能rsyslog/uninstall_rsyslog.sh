#!/bin/bash

install_target=/lib/sysmaster/system
enable_target=/etc/sysmaster/system/multi-user.target.wants/

test -f ${enable_target}/syslog.service && sctl disable syslog.service

test -f ${install_target}/syslog.service && rm -f ${install_target}/syslog.service

test -f /etc/rsyslog.conf_ori && mv -f /etc/rsyslog.conf_ori /etc/rsyslog.conf
