#!/bin/bash

check() {
    require_binaries devctl || return 1
    return 255
}

# called by dracut
install() {
    rm -f "${initdir}${systemdutildir}"/systemd-udevd \
        "${initdir}"/bin/udevadm \
        "${initdir}"/sbin/udevd \
        "${initdir}"/"${udevdir}"/udevd

    inst_script "$moddir/init.sh" "/init"

    inst_multiple devctl

    inst_dir /etc/devmaster
    inst_dir /lib/devmaster
    inst_dir /etc/devmaster/network.d
    inst_multiple -o /etc/devmaster/config.toml \
	    /etc/devmaster/network.d/99-default.link \
	    /lib/devmaster/simulate_udev.sh
    ln -sf /bin/devctl "${initdir}"/bin/udevadm
    ln -sf /bin/devctl "${initdir}"/lib/devmaster/devmaster
}
