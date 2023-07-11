#!/usr/bin/env bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e
max=1000

yum install -y nginx
expect_eq $? 0 || exit 1

cp -arf "${work_dir}"/tmp_units/*.service ${SYSMST_LIB_PATH} || exit 1
run_sysmaster || exit 1

sctl restart nginx
check_status nginx active
expect_eq $? 0 || exit 1

echo "$(date)    init: $(cat /proc/${sysmaster_pid}/stat | awk '{print $14,$15}')" >> /opt/cpu_time_data
for ((cnt = 0; cnt < ${max}; ++cnt)); do
    sctl restart nginx
    expect_eq $? 0 "restart for ${cnt} times"
    sleep 0.1
done
echo "$(date)    end: $(cat /proc/${sysmaster_pid}/stat | awk '{print $14,$15}')" >> /opt/cpu_time_data
cat /opt/cpu_time_data

check_status nginx active
expect_eq $? 0

exit "${EXPECT_FAIL}"
