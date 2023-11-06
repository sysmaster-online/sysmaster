#!/bin/bash

pwd=$(pwd)
run_with_rsyslog=${pwd}

install -Dm0640 -t /lib/sysmaster/system ${run_with_rsyslog}/syslog.service

install -Dm0644 -t /etc ${run_with_rsyslog}/rsyslog.conf
