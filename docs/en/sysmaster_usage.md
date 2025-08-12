# sysmaster Usage Instructions

This section provides examples on how to use sysmaster, including:

* service unit configuration file creation
* unit service management operations, such as starting, stopping, and viewing services

For more, see the [sysMaster official manual](http://sysmaster.online/man/all/).

## Unit Configuration File Creation

You can create unit configuration files in the **/usr/lib/sysmaster/system/** directory.

### Types of Unit Configuration Files

Currently, sysmaster supports unit configuration files of the **target**, **socket**, and **service** types.

* **target**: Encapsulated startup target managed by sysmaster, which is used for grouping units as a synchronization point. sysmaster provides targets for different states. For example, **multi-user.target** indicates that the system has been started. You can use this target to configure services to run in this state.
* **socket**: Encapsulated socket for inter-process communication to support socket-based startup. For example, you can configure a service unit to depend on a socket. When data is written to the socket, sysmaster starts the corresponding service unit.
* **service**: Encapsulated process monitored and controlled by sysmaster.

### Composition of Unit Configuration Files

A unit configuration file consists of three sections:

* **Unit**: common configuration description of the unit, such as the service name, description, and dependencies
* **Install**: description of how the service is installed and started
* **Service** and **Socket**: configurations of different unit types

### Creating a service Unit

The **sshd** service is used to remotely log in to the server and run commands and perform operations on the remote terminal.
The following configuration items are used to create an **sshd.service** service unit:

```bash
[Unit]
Description="OpenSSH server daemon"
Documentation="man:sshd(8) man:sshd_config(5)"
After="sshd-keygen.target"
Wants="sshd-keygen.target"

[Service]
Type="notify"
EnvironmentFile="-/etc/sysconfig/sshd"
ExecStart="/usr/sbin/sshd -D $OPTIONS"
ExecReload="/bin/kill -HUP $MAINPID"
KillMode="process"
Restart="on-failure"
RestartSec=42

[Install]
WantedBy="multi-user.target"
```

The configuration items in the example are described as follows:

* **Description**: Main functions of the unit.
* **Documentation**: Document link of the unit.
* **After**: Unit startup sequence. In the example, **sshd.service** is started after **sshd-keygen.target**.
* **Wants**: Dependency on another unit. In the example, **sshd-keygen.target** is automatically started with **sshd.service**.
* **Type**: How sysmaster starts the service. **notify** indicates that a notification will be sent after the main process is started.
* **EnvironmentFile**: Path of file that stores environment variables to be loaded.
* **ExecStart**: Command executed when the service is started. In the example, `sshd` is executed when **sshd.service** is started.
* **ExecReload**: Command executed to reload the **sshd.service** configurations.
* **KillMode**: How the process is killed when the service process needs to be stopped. **process** indicates that only the main process is killed.
* **Restart**: Whether to restart the service when the service exits or stops in different situations. **on-failure** indicates that the service is restarted when the service exits abnormally.
* **RestartSec**: Amount of time to wait before the service is restarted after the service exits.
* **WantedBy**: Units that depend on **sshd.service**.

## Unit Service Management

`sctl` is a CLI tool of sysmaster. It is used to check and control the behavior of the sysmaster server and the status of each service. It can start, stop, restart, and check system services.

### Starting a Service

Run the following command to start the **sshd** service and run the commands specified by **ExecStart**:

```bash
# sctl start sshd.service
```

### Stopping a Service

Run the following command to stop the **sshd** service and kill the process started by **ExecStart**:

```bash
# sctl stop sshd.service
```

### Restarting a Service

Run the following command to restart the **sshd** service. After the command is executed, the **sshd** service is stopped and then started.

```bash
# sctl restart sshd.service
```

### Checking Service Status

Run the following command to check the status of the **sshd** service. You can check whether the service is running properly by viewing the service status.

```bash
# sctl status sshd.service
```
