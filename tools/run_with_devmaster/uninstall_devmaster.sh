#!/bin/bash

services=(devctl-trigger.service devmaster.service devmaster-simu-udev.service)
tools=(ata_id)

etc_conf_install_dir=/etc/devmaster
etc_netconf_install_dir=${etc_conf_install_dir}/network.d
etc_rules_install_dir=${etc_conf_install_dir}/rules.d

lib_devmaster_dir=/lib/devmaster
lib_rules_install_dir=${lib_devmaster_dir}/rules.d

service_install_dir=/lib/sysmaster/system
sysinit_target_dir=/etc/sysmaster/system/sysinit.target.wants

for s in ${services[@]}; do
    rm -f ${service_install_dir}/$s
    test -f ${sysinit_target_dir}/$s && unlink ${sysinit_target_dir}/$s
done

rm -rf ${lib_devmaster_dir} ${etc_conf_install_dir}

sync
