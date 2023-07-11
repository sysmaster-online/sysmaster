#!/usr/bin/env bash

while ((1)); do
    time="$(date)"
    init_data="$(ps aux | grep -v grep | grep '/usr/lib/sysmaster/init')"
    sysmst_data="$(ps aux | grep -v grep | grep '/usr/lib/sysmaster/sysmaster')"

    pid=$(echo ${init_data} | awk '{print $2}')
    rss=$(echo ${init_data} | awk '{print $6}')
    line="${time}     init(pid:${pid}  rss:$rss  fd:$(ls /proc/${pid}/fd | wc -l))"
    pid=$(echo ${sysmst_data} | awk '{print $2}')
    rss=$(echo ${sysmst_data} | awk '{print $6}')
    line="${line}     sysmaster(pid:${pid}  rss:$rss  fd:$(ls /proc/${pid}/fd | wc -l))"
    echo "${line}" >> /opt/monitor.log
    sleep 60
done
