#!/usr/bin/env bash

mode="${1:-debug}"
pwd=$(pwd)
target_dir=${pwd}/target/${mode}
units_dir=${pwd}/factory/usr/lib/sysmaster/system
run_with_vm_dir=${0%/*}
sysmaster_install_target=/usr/lib/sysmaster
conf_install_target=/etc/sysmaster
rules_dir=${pwd}/factory/usr/lib/udev/rules.d

multi_user_target=(dbus.service fstab.service getty.target hostname-setup.service \
NetworkManager.service sshd.service udev-trigger.service)
sysinit_target=(udevd.service)
getty_target=(getty@.service serial-getty@.service)

mkdir -p ${sysmaster_install_target}/system-generators

# Install binaries of sysmaster.
install -Dm0550 -t /usr/bin ${target_dir}/sctl || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/init || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/sysmaster || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/fstab || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/sysmonitor || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/random_seed || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/rc-local-generator || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/hostname_setup || exit 1
install -Dm0550 -t ${sysmaster_install_target}/system-generators ${target_dir}/getty-generator || exit 1

# Install '.service', '.socket', and '.target' units.
install -Dm0640 -t ${sysmaster_install_target}/system ${units_dir}/* || exit 1

for service in ${multi_user_target[@]} ${sysinit_target[@]}; do
    install -Dm0640 -t ${sysmaster_install_target}/system ${run_with_vm_dir}/${service} || exit 1
done

for service in ${getty_target[@]}; do
    install -Dm0640 -t ${sysmaster_install_target}/system ${run_with_vm_dir}/${service} || exit 1
done

install -Dm0640 -t ${sysmaster_install_target}/system ${run_with_vm_dir}/*.socket || exit 1

# Simulate `sctl enable` command to automatically start services after bootup.
mkdir -p /etc/sysmaster/system/multi-user.target.wants
mkdir -p /etc/sysmaster/system/sysinit.target.wants
mkdir -p /etc/sysmaster/system/syslog.target.wants

for unit in ${multi_user_target[@]}; do
    ln -sf ${sysmaster_install_target}/system/${unit} /etc/sysmaster/system/multi-user.target.wants/${unit}
done

for unit in ${sysinit_target[@]}; do
    ln -sf ${sysmaster_install_target}/system/${unit} /etc/sysmaster/system/sysinit.target.wants/${unit}
done

# Install compatible rules for lvm
install -Dm444 -t /usr/lib/udev/rules.d ${rules_dir}/99-sysmaster.rules || exit 1

# Install configurations of sysmaster.
install -Dm0640 -t ${conf_install_target} ${pwd}/factory/etc/sysmaster/system.conf || exit 1

# Create the symbolic linkage '/init' to sysmaster-init.
ln -sf ${sysmaster_install_target}/init /init

# Install syslog.target
ln -sf ${sysmaster_install_target}/system/syslog.socket /etc/sysmaster/system/syslog.target.wants/syslog.socket

ln -sf  /usr/bin/sctl  /usr/sbin/halt
ln -sf  /usr/bin/sctl  /usr/sbin/reboot
ln -sf  /usr/bin/sctl  /usr/sbin/poweroff
ln -sf  /usr/bin/sctl  /usr/sbin/shutdown

sync
