# 使用说明

本章介绍 `devmaster`的使用方法，包括常驻进程配置、客户端工具、规则使用说明和网卡配置。

## 常驻进程配置

常驻进程 `devmaster`启动后会读取配置文件，并根据配置文件内容，调整日志级别、设置规则加载路径等等。`devmaster`拥有唯一的配置文件，路径为 `/etc/devmaster/config.toml`，文件内容采用 `toml`格式。

### 配置选项

目前 `devmaster`配置文件中支持的配置选项如下：

- `rules_d`: 指定规则加载路径，默认规则中设置为 `["/etc/devmaster/rules.d"]`，未指定时无默认加载路径。`devmaster`当前不支持规则加载优先级，不同规则路径下的同名规则文件不会发生覆盖。规则文件的加载顺序按照 `rules_d`配置项中指定的目录顺序，相同目录下按照规则文件的字典序进行加载。
- `max_workers`: 指定最大 `worker`线程并发数，未指定时默认为3。
- `log_level`: 指定日志级别，支持 `debug`和 `info`级别，未指定时默认为 `"info"`。
- `network_d`: 指定网卡配置加载路径，默认规则中设置为 `["/etc/devmaster/network.d"]`，未指定时无默认加载路径。网卡配置用于控制 `devmaster`的内置命令 `net_setup_link`的行为，具体可参考[网卡配置说明](#网卡配置)。

## 客户端工具

`devctl`是常驻进程 `devmaster`的客户端工具，用来控制 `devmaster`的行为、模拟设备事件、调试规则等等。

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

选项说明：

  `-h, --help`：  显示帮助信息。

  `-V, --version`：  显示版本信息。

  `<SUBCOMMAND>`： 选择执行的子命令，包括`monitor`、`trigger`、`test-builtin`等。

接下来介绍三种常用的子命令，分别用于监听设备事件、触发设备事件以及测试内置命令。

### 监听设备事件

监听内核上报的 `uevent`事件和 `devmaster`处理完设备后发出的事件，分别以 `KERNEL`和 `USERSPACE`作为前缀进行区分，执行的命令如下：

  ```shell
  # devctl monitor [OPTIONS]
  ```

选项说明：

  `-h, --help`：  显示帮助信息。

### 触发设备事件

模拟一个设备动作，使内核上报对应的uevent事件，用于重放内核初始化过程中的冷插(coldplug)设备事件，执行的命令如下：

  ```shell
  # devctl trigger [OPTIONS] [DEVICES...]
  ```

选项说明：

  `-h, --help`：    显示帮助信息。

  `-a, --action <ACTION>`：   指定设备事件的动作类型。

  `-t, --type <TYPE>`：    指定搜索的设备类型，可以是`devices`（设备）或者`subsystems`（子系统）。

  `-v, --verbose`：    打印搜索到的设备。

  `-n, --dry-run`：    不会实际触发设备事件，配合`--verbose`选项使用时，可以查看系统中的设备清单。

  `[DEVICES...]`：    指定若干个需要触发事件的设备，如果为空，则触发系统中所有设备的事件。

### 测试内置命令

测试内置命令在某个设备上的执行效果，执行的命令如下：

  ```shell
  # devctl test-builtin [OPTIONS] <BUILTIN> <SYSPATH>
  ```

选项说明：

  `-a, --action <ACTION>`：    指定设备事件的动作类型，包括：`add`、`change`、`remove`、`move`、`online`、`offline`、`bind`和 `unbind`。

  `-h, --help`：    显示帮助信息。

  `<BUILTIN>`： 选择执行的内置命令，目前支持`blkid`、`input_id`、`kmod`、`net_id`、`net_setup_link`、`path_id`、`usb_id`。

  `<SYSPATH>`： 指定设备的 `sysfs`路径。

## 规则使用说明

`devmaster`的规则由一组规则文件组成，`devmaster`常驻进程启动后会根据配置文件中指定的规则加载目录，按字典序依次加载各个规则文件。

> [!NOTE]说明
> 增加、删除、修改规则后，均需要重启 `devmaster`使之生效。

### 常用规则案例

以下介绍几种常见的规则应用案例，规则语法详见官方文档中的[devmaster手册](http://sysmaster.online/man/exts/devmaster/devmaster/)。

#### 示例1: 创建块设备软链接

通过 `blkid`内置命令，读取块设备的 `uuid`，并基于 `uuid`创建块设备的软链接。

触发拥有文件系统的某块设备的事件后，在 `/dev/test`目录下生成该设备对应的软链接。

以 `sda1`分区块设备为例，测试规则效果：

1. 创建规则文件 `/etc/devmaster/rules.d/00-persist-storage.rules`，内容如下：

    ```shell
    SUBSYSTEM!="block", GOTO="end"

    IMPORT{builtin}=="blkid"

    ENV{ID_FS_UUID_ENC}=="?*", SYMLINK+="test/$env{ID_FS_UUID_ENC}"

    LABEL="end"
    ```

2. 触发 `sda1`设备的事件：

    ```shell
    # devctl trigger /dev/sda1
    ```

3. 查看 `/dev/test/`目录下存在指向 `sda1`的软链接，表示规则生效：

    ```shell
    # ll /dev/test/
    total 0
    lrwxrwxrwx 1 root root 7 Sep  6 15:35 06771fe1-39da-42d7-ad3c-236a10d08a7d -> ../sda1
    ```

#### 示例2: 网卡重命名

使用 `net_id`内置命令，获取网卡设备的硬件属性，再使用 `net_setup_link`内置命令，基于网卡配置选择某个硬件属性作为网卡名，最后通过 `NAME`规则重命名网卡。

以 `ens33`网卡为例，测试网卡重命名规则的效果：

1. 创建规则文件 `/etc/devmaster/rules.d/01-netif-rename.rules`，内容如下：

    ```shell
    SUBSYSTEM!="net", GOTO="end"

    IMPORT{builtin}=="net_id"

    IMPORT{builtin}=="net_setup_link"

    ENV{ID_NET_NAME}=="?*", NAME="$env{ID_NET_NAME}"

    LABEL="end"
    ```

2. 创建网卡配置`/etc/devmaster/network.d/99-default.link`，内容如下：

    ```shell
    [Match]
    OriginalName = "*"

    [Link]
    NamePolicy = ["database", "onboard", "slot", "path"]
    ```

3. 先将网卡设备下线：

    ```shell
    # ip link set ens33 down
    ```

4. 将网卡名临时命名为 `tmp`：

    ```shell
    # ip link set ens33 name tmp
    ```

5. 触发网卡设备的 `add`事件：

    ```shell
    # devctl trigger /sys/class/net/tmp --action add
    ```

6. 查看网卡名称，发现重新命名为 `ens33`，表示规则生效：

    ```shell
    # ll /sys/class/net/| grep ens33
    lrwxrwxrwx 1 root root 0 Sep  6 11:57 ens33 -> ../../devices/pci0000:00/0000:00:11.0/0000:02:01.0/net/ens33
    ```

7. 激活网卡后恢复网络连接：

    ```shell
    # ip link set ens33 up
    ```

> [!NOTE]说明
> 网卡设备处于激活状态下无法重命名，需要先将其下线。另外 `devmaster`仅在网卡设备的 `add`事件下对网卡重命名才会生效。

#### 示例3: 修改设备节点的用户权限

`OPTIONS+="static_node=<devnode>`规则会使 `devmaster`启动后，立即将本规则行中的用户权限应用在 `/dev/<devnode>`设备节点上。重启 `devmaster`后立即生效，无需设备事件触发。

1. 创建规则文件`/etc/devmaster/rules.d/02-devnode-privilege.rules`，内容如下：

    ```shell
    OWNER="root", GROUP="root", MODE="777", OPTIONS+="static_node=tty5"
    ```

2. 重启 `devmaster`后，观察 `/dev/tty5`的用户、用户组和权限，变更为 `root`、`root`和 `rwxrwxrwx`，表示规则生效：

    ```shell
    # ll /dev/tty5
    crwxrwxrwx 1 root root 4, 5 Feb  3  2978748 /dev/tty5
    ```

## 网卡配置

`devmaster`的网卡重命名功能由内置命令 `net_id`、`net_setup_link`和网卡配置文件配合完成。在规则文件中，通过 `net_id`获取网卡的硬件属性，再使用 `net_setup_link`选择某个网卡属性作为新的网卡名。`net_setup_link`命令基于网卡配置，针对特定网卡设备，控制网卡命名的风格。本章主要介绍网卡配置文件的使用方法，网卡重命名的实施方法可参考[网卡重命名规则案例](#示例2-网卡重命名)。

### 默认网卡配置

`devmaster`提供了如下默认网卡配置：

  ```toml
  [Match]
  OriginalName = "*"

  [Link]
  NamePolicy = ["onboard", "slot", "path"]
  ```

网卡配置文件中包含 `[Match]`匹配节和 `[Link]`控制节，每节中包含若干配置项。匹配节的配置项用于匹配网卡设备，当网卡满足所有匹配条件时，将控制节中的所有配置项作用在网卡上，比如设置网卡名选取策略、调整网卡参数等等。

以上列举的默认网卡配置表示将该配置作用在所有网卡设备上，并依次检查 `onboard`、`slot`和 `path`风格的网卡命名风格，如果找到一个可用的风格，就以该风格对网卡进行命名。

网卡配置的详细说明可以参考 `sysMaster`官方手册中的[devmaster手册](http://sysmaster.online/man/exts/devmaster/netif_config/#1)。
