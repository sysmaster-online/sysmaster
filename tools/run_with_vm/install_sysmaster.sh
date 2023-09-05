#!/usr/bin/env bash

mode="${1:-debug}"
pwd=$(pwd)
target_dir=${pwd}/target/${mode}
units_dir=${pwd}/units
run_with_vm_dir=${pwd}/tools/run_with_vm
sysmaster_install_target=/usr/lib/sysmaster
conf_install_target=/etc/sysmaster

multi_user_target=(dbus.service fstab.service getty-tty1.service hostname-setup.service \
lvm-activate-openeuler.service NetworkManager.service sshd-keygen@ecdsa.service \
sshd-keygen@ed25519.service sshd-keygen@rsa.service sshd.service)
sysinit_target=(udevd.service udev-trigger.service)

# Install binaries of sysmaster.
install -Dm0550 -t /usr/bin ${target_dir}/sctl || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/init || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/sysmaster || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/fstab || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/sysmonitor || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/random_seed || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/rc-local-generator || exit 1
install -Dm0550 -t ${sysmaster_install_target} ${target_dir}/hostname_setup || exit 1

# Install '.service', '.socket', and '.target' units.
install -Dm0640 -t ${sysmaster_install_target}/system ${units_dir}/* || exit 1

for service in ${multi_user_target[@]} ${sysinit_target[@]}; do
    install -Dm0640 -t ${sysmaster_install_target}/system ${run_with_vm_dir}/${service} || exit 1
done

install -Dm0640 -t ${sysmaster_install_target}/system ${run_with_vm_dir}/*.socket || exit 1

# Simulate `sctl enable` command to automatically start services after bootup.
mkdir -p /etc/sysmaster/system/multi-user.target.wants
mkdir -p /etc/sysmaster/system/sysinit.target.wants

for unit in ${multi_user_target[@]}; do
    ln -sf ${sysmaster_install_target}/system/${unit} /etc/sysmaster/system/multi-user.target.wants/${unit}
done

for unit in ${sysinit_target[@]}; do
    ln -sf ${sysmaster_install_target}/system/${unit} /etc/sysmaster/system/sysinit.target.wants/${unit}
done

strip ${target_dir}/lib*.so

# Install the dynamic libraries of unit plugins.
install -Dm0550 -t ${sysmaster_install_target}/plugin ${target_dir}/libmount.so || exit 1
install -Dm0550 -t ${sysmaster_install_target}/plugin ${target_dir}/libservice.so || exit 1
install -Dm0550 -t ${sysmaster_install_target}/plugin ${target_dir}/libsocket.so || exit 1
install -Dm0550 -t ${sysmaster_install_target}/plugin ${target_dir}/libtarget.so || exit 1

# Install configurations of sysmaster.
install -Dm0550 -t ${sysmaster_install_target}/plugin ${pwd}/config/conf/plugin.conf || exit 1
install -Dm0640 -t ${conf_install_target} ${pwd}/config/conf/system.conf || exit 1

# Create the symbolic linkage '/init' to sysmaster-init.
ln -sf ${sysmaster_install_target}/init /init

sync
