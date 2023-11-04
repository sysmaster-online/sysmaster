#!/bin/bash

install_target=/lib/sysmaster/system


test -f ${install_target}/syslog.service && rm -f ${install_target}/syslog.service
