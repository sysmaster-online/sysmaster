# Usage Instructions

This section describes how to use devmaster, covering daemon configuration, client tool, rule usage, and NIC configuration.

## Daemon Configuration

After being started, the devmaster daemon reads the configuration file, adjusts the log level, and sets the rule path based on the configuration file. devmaster has a unique configuration file **/etc/devmaster/config.toml**, which is in TOML format.

### Configuration Items

The devmaster configuration file supports the following configuration items:

- **rules_d**: Rule path. The default value is **\["/etc/devmaster/rules.d"]**. If this item is not specified, there is no default path. Currently, devmaster does not support rule loading priorities. Rule files with the same name in different rule paths will not conflict with each other. Rule files are loaded in the sequence specified by **rules_d**. Rule files in the same directory are loaded in the lexicographical sequence.
- **max_workers**: Maximum number of concurrent worker threads. If this item is not specified, the default value **3** is used.
- **log_level**: Log level. The value can be **debug** or **info**. If this parameter is not specified, the default value **info** is used.
- **network_d**: NIC configuration path. The default value is **\["/etc/devmaster/network.d"]**. If this parameter is not specified, there is no default path. NIC configurations control the behavior of the `net_setup_link` command of devmaster. For details, see [NIC Configuration](#nic-configuration).

## Client Tool

`devctl` is the client tool of the devmaster daemon. It is used to control devmaster behaviors, simulate device events, and debug rules.

  ```shell
  # devctl --help
  devmaster 0.5.0
  parse program arguments

  USAGE:
      devctl <SUBCOMMAND>

  OPTIONS:
      -h, --help       Print help information
      -V, --version    Print version information

  SUBCOMMANDS:
      monitor         Monitor device events from kernel and userspace
      kill            Kill all devmaster workers
      test            Send a fake device to devmaster
      trigger         Trigger a fake device action, then the kernel will report an uevent
      test-builtin    Test builtin command on a device
      help            Print this message or the help of the given subcommand(s)
  ```

Command options:

  `-h, --help`: Displays help information.

  `-V, --version`: Displays version information.

  `<SUBCOMMAND>`: Subcommand to be executed, including `monitor`, `trigger`, and `test-builtin`.

The following sections describe the three frequently used subcommands, which are used to monitor device events, trigger device events, and test built-in commands.

### Monitoring Device Events

Monitor uevent events reported by the kernel and events sent after devmaster processes devices, which are prefixed with **KERNEL** and **USERSPACE**, respectively. The command is as follows:

  ```shell
  # devctl monitor [OPTIONS]
  ```

Command options:

  `-h, --help`: Displays help information.

### Triggering Device Events

Simulate a device action to trigger a kernel uevent event. This operation is used to replay coldplug device events during kernel initialization. The command is as follows:

  ```shell
  # devctl trigger [OPTIONS] [DEVICES...]
  ```

Command options:

  `-h, --help`: Displays help information.

  `-a, --action <ACTION>`: Action type of a device event.

  `-t, --type <TYPE>`: Type of the device to be searched for. The value can be **devices** or **subsystems**.

  `-v, --verbose`: Prints the found devices.

  `-n, --dry-run`: Does not trigger device events. This option can be used together with `--verbose` to view the list of devices in the system.

  `[DEVICES...]`: Devices for which events are triggered. If this item is left blank, events of all devices in the system are triggered.

### Testing Built-in Commands

Test the effect of a built-in command on a device. The command is as follows:

  ```shell
  # devctl test-builtin [OPTIONS] <BUILTIN> <SYSPATH>
  ```

Command options:

  `-a, --action <ACTION>`: Action type of a device event. The value can be **add**, **change**, **remove**, **move**, **online**, **offline**, **bind**, or **unbind**.

  `-h, --help`: Displays help information.

  `<BUILTIN>`: Built-in command to be executed. The value can be **blkid**, **input_id**, **kmod**, **net_id**, **net_setup_link**, **path_id**, or **usb_id**.

  `<SYSPATH>`: Sysfs path of the device.

## Rule Usage

devmaster rules consist of a group of rule files. After the devmaster daemon is started, it loads the rule files in lexicographic order based on the rule path specified in the configuration file.

> [!Note]Note
>
> After adding, deleting, or modifying a rule, you need to restart devmaster for the rule to take effect.

### Rule Examples

The following describes several common rule examples.

#### Example 1: Creating a Soft Link for a Block Device

Use the `blkid` built-in command to read the UUID of a block device and create a soft link for the block device based on the UUID.

After an event of a device that has a file system is triggered, a soft link corresponding to the device is generated in the **/dev/test** directory.

The following uses the block device of the **sda1** partition as an example.

1. Create the rule file **/etc/devmaster/rules.d/00-persist-storage.rules**. The file content is as follows:

    ```shell
    SUBSYSTEM!="block", GOTO="end"

    IMPORT{builtin}=="blkid"

    ENV{ID_FS_UUID_ENC}=="?*", SYMLINK+="test/$env{ID_FS_UUID_ENC}"

    LABEL="end"
    ```

2. Trigger the **sda1** device event:

    ```shell
    # devctl trigger /dev/sda1
    ```

3. Check if a soft link pointing to **sda1** exists in the **/dev/test/** directory. If yes, the rule takes effect.

    ```shell
    # ll /dev/test/
    total 0
    lrwxrwxrwx 1 root root 7 Sep  6 15:35 06771fe1-39da-42d7-ad3c-236a10d08a7d -> ../sda1
    ```

#### Example 2: Renaming a NIC

Use the `net_id` built-in command to obtain the hardware attributes of the NIC, then run the `net_setup_link` built-in command to select a hardware attribute based on the NIC configuration as the NIC name, and rename the NIC through the **NAME** rule.

The following uses the **ens33** NIC as an example to test the effect of the NIC renaming rule:

1. Create the rule file **/etc/devmaster/rules.d/01-netif-rename.rules**. The file content is as follows:

    ```shell
    SUBSYSTEM!="net", GOTO="end"

    IMPORT{builtin}=="net_id"

    IMPORT{builtin}=="net_setup_link"

    ENV{ID_NET_NAME}=="?*", NAME="$env{ID_NET_NAME}"

    LABEL="end"
    ```

2. Create the NIC configuration file **/etc/devmaster/network.d/99-default.link**. The content is as follows:

    ```shell
    [Match]
    OriginalName = "*"

    [Link]
    NamePolicy = ["database", "onboard", "slot", "path"]
    ```

3. Bring the NIC offline.

    ```shell
    # ip link set ens33 down
    ```

4. Temporarily name the NIC **tmp**:

    ```shell
    # ip link set ens33 name tmp
    ```

5. Trigger the **add** event of the NIC.

    ```shell
    # devctl trigger /sys/class/net/tmp --action add
    ```

6. Check the NIC name. If the NIC name is changed to **ens33**, the rule takes effect.

    ```shell
    # ll /sys/class/net/| grep ens33
    lrwxrwxrwx 1 root root 0 Sep  6 11:57 ens33 -> ../../devices/pci0000:00/0000:00:11.0/0000:02:01.0/net/ens33
    ```

7. Restore the network connection after activating the NIC.

    ```shell
    # ip link set ens33 up
    ```

> [!Note]Note
>
> An activated NIC cannot be renamed. You need to bring it offline first. In addition, the renaming rule of devmaster takes effect only in the **add** event of the NIC.

#### Example 3: Modifying the User Permissions on a Device Node

The **OPTIONS+="static_node=\<devnode>** rule enables devmaster to immediately apply the user permissions in this rule to **/dev/\<devnode>** after devmaster is started. The configuration takes effect immediately after devmaster is restarted. No device event is required.

1. Create the rule file **/etc/devmaster/rules.d/02-devnode-privilege.rules**. The file content is as follows:

    ```shell
    OWNER="root", GROUP="root", MODE="777", OPTIONS+="static_node=tty5"
    ```

2. After devmaster is restarted, check the user, user group, and permissions of **/dev/tty5**. If the user, user group, and permissions are changed to **root**, **root**, and **rwxrwxrwx**, the rule takes effect.

    ```shell
    # ll /dev/tty5
    crwxrwxrwx 1 root root 4, 5 Feb  3  2978748 /dev/tty5
    ```

## NIC Configuration

The NIC renaming function of devmaster is implemented by the built-in commands `net_id` and `net_setup_link` and the NIC configuration file. In the rule file, use `net_id` to obtain the hardware attributes of a NIC, and then use `net_setup_link` to select a NIC attribute as the new NIC name. The `net_setup_link` command controls the NIC naming style for a specific NIC based on the NIC configuration file. This section describes how to use the NIC configuration file. For details about how to rename a NIC, see [Renaming a NIC](#example-2-renaming-a-nic).

### Default NIC Configurations

devmaster provides the following default NIC configurations:

  ```toml
  [Match]
  OriginalName = "*"

  [Link]
  NamePolicy = ["onboard", "slot", "path"]
  ```

The NIC configuration file contains the **\[Match]** matching section and **\[Link]** control section. Each section contains several configuration items. The configuration items in the **\[Match]** section are used to match NICs. When a NIC meets all matching conditions, all configuration items in the **\[Link]** section are applied to the NIC, for example, setting the NIC naming style and adjusting NIC parameters.

The preceding default NIC configuration indicates that the configuration takes effect on all NICs and checks the NIC naming styles of the **onboard**, **slot**, and **path** styles in sequence. If an available style is found, the NIC is named in this style.
