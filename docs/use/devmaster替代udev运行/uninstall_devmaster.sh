#!/bin/bash

services=(devctl-trigger.service devmaster.service devmaster-simu-udev.service)
tools=(ata_id)

etc_conf_install_dir=/etc/devmaster
etc_netconf_install_dir=${etc_conf_install_dir}/network.d
etc_rules_install_dir=${etc_conf_install_dir}/rules.d

lib_devmaster_dir=/lib/devmaster

dracut_modules=/lib/dracut/modules.d/98devmaster

service_install_dir=/lib/sysmaster/system
sysinit_target_dir=/etc/sysmaster/system/sysinit.target.wants
multi_user_target_dir=/etc/sysmaster/system/multi-user.target.wants

for s in ${services[@]}; do
    rm -f ${service_install_dir}/$s
done

unlink ${sysinit_target_dir}/devmaster.service
unlink ${multi_user_target_dir}/devctl-trigger.service

rm -rf ${lib_devmaster_dir} ${etc_conf_install_dir} ${dracut_modules}

sync
