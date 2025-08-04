# Installation and Deployment

Currently, devmaster can be used in the VM environment. This section describes the requirements and procedure of devmaster installation and deployment.

## Software

* OS: openEuler 23.09

## Hardware

* x86_64 or AArch64 architecture

## Installation and Deployment

1. Run the following `yum` command to install the sysmaster-devmaster package:

    ```shell
    # yum install sysmaster-devmaster
    ```

2. Run the following commands to create the default rule file **/etc/devmaster/rules.d/99-default.rules** and the daemon configuration file **/etc/devmaster/config.toml**:

    ```shell
    # mkdir -p /etc/devmaster/rules.d
    # mkdir -p /etc/devmaster/network.d
    # echo "TAG+=\"devmaster\"" > /etc/devmaster/rules.d/99-default.rules
    # cat << EOF > /etc/devmaster/config.toml
    log_level = "info"
    rules_d = ["/etc/devmaster/rules.d"]
    network_d = ["/etc/devmaster/network.d"]
    max_workers = 1
    log_targets = ["console"]
    EOF
    ```

3. Run the following commands to start the devmaster daemon and export logs to the **/tmp/devmaster.log** file:

    ```shell
    # /lib/devmaster/devmaster &>> /tmp/devmaster.log &
    ```

    > ![Note](./public_sys-resources/icon-note.gif)**Note:**
    >
    > devmaster must be started with the root privilege and cannot be running with udev at the same time. Before starting devmaster, stop the udev service.
    >
    > If udev is started by sysMaster, run the following command:

    ```shell
    # sctl stop udevd.service udevd-control.socket udevd-kernel.socket
    ```

    > If udev is started by systemd, run the following command:

    ```shell
    # systemctl stop systemd-udevd.service systemd-udevd systemd-udevd-kernel.socket systemd-udevd-control.socket
    ```

4. Run the following command to use the `devctl` tool to trigger a device event:

    ```shell
    # devctl trigger
    ```

5. Check the **/run/devmaster/data/** directory. If the device database is generated, the deployment is successful.

    ```shell
    # ll /run/devmaster/data/
    ```
