#!/bin/bash

mode="${1:-debug}"
pwd=$(pwd)
target_dir=${pwd}/target/${mode}

services=(devctl-trigger.service devmaster.service devmaster-simu-udev.service)
tools=(ata_id)

run_with_devmaster=${0%/*}
service_dir=${run_with_devmaster}/service

etc_dir=exts/devmaster/config
etc_rules_dir=${etc_dir}/rules.d
etc_netconf_dir=${etc_dir}/network.d

dracut_modules=exts/devmaster/dracut_modules/98devmaster

etc_conf_install_dir=/etc/devmaster
etc_netconf_install_dir=${etc_conf_install_dir}/network.d
etc_rules_install_dir=${etc_conf_install_dir}/rules.d

lib_devmaster_dir=/lib/devmaster
lib_rules_install_dir=${lib_devmaster_dir}/rules.d

service_install_dir=/lib/sysmaster/system
sysinit_target_dir=/etc/sysmaster/system/sysinit.target.wants
multi_user_target_dir=/etc/sysmaster/system/multi-user.target.wants

# Install binaries.
install -Dm0550 -t /usr/bin ${target_dir}/devctl || exit 1
install -Dm0550 -t ${lib_devmaster_dir} ${run_with_devmaster}/simulate_udev.sh || exit 1
ln -sf -T /usr/bin/devctl ${lib_devmaster_dir}/devmaster || exit 1
for tool in $tools; do
    install -Dm0550 -t ${lib_devmaster_dir} ${target_dir}/$tool || exit 1
done

# Install configurations under /etc.
install -Dm0640 -t ${etc_conf_install_dir} ${etc_dir}/config.toml || exit 1
install -Dm0640 -t ${etc_netconf_install_dir} ${etc_netconf_dir}/*.link || exit 1
install -Dm0640 -t ${etc_rules_install_dir} ${etc_rules_dir}/*.rules || exit 1

# Install services.
install -Dm0640 -t ${service_install_dir} ${service_dir}/*.service || exit 1

ln -sf ${service_install_dir}/devmaster.service ${sysinit_target_dir}/devmaster.service
ln -sf ${service_install_dir}/devctl-trigger.service ${multi_user_target_dir}/devctl-trigger.service

# Disable udev rules if they exists
test -f ${sysinit_target_dir}/udevd.service && unlink ${sysinit_target_dir}/udevd.service
test -f ${multi_user_target_dir}/udev-trigger.service && unlink ${multi_user_target_dir}/udev-trigger.service

# Install dracut module of devmaster
install -Dm0755 -t /lib/dracut/modules.d/98devmaster ${dracut_modules}/* || exit 1

sync
