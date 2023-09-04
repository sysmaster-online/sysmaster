# devctl使用手册

## 1.简介

`devctl`是`devmaster`的客户端管理工具，用来控制`demaster`的行为、模拟设备事件、调试规则等等。

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

## 2.选项

### devctl monitor [options]
监听内核上报的`uevent`事件和`devmaster`处理完设备后发出的事件，分别以`KERNEL`和`USERSPACE`作为前缀进行区分。

```shell
# devctl monitor --help
devctl-monitor
Monitor device events from kernel and userspace

USAGE:
    devctl monitor

OPTIONS:
    -h, --help    Print help information
```

参数：无

选项：
    -h, --help  显示帮助信息

### [deprecated] devctl kill
使`devmaster`终止所有`worker`，正在运行中的`worker`会等待执行完当前任务再终止，期间无法再接收新的任务。

参数：无

选项：
    -h, --help  显示帮助信息

### [deprecated] devctl test <DEVNAME\>
向`devmaster`发送模拟设备，调试`devmaster`的框架功能。

参数：
    `<DEVNAME>`   模拟设备的名称

选项：
    -h, --help  显示帮助信息

### devctl trigger [OPTIONS] [DEVICES]...

模拟一个设备动作，使内核上报对应的uevent事件，用于重放内核初始化过程中的冷插(coldplug)设备事件。

```shell
# devctl trigger --help
devctl-trigger
Trigger a fake device action, then the kernel will report an uevent

USAGE:
    devctl trigger [OPTIONS] [DEVICES]...

ARGS:
    <DEVICES>...    the devices to be triggered

OPTIONS:
    -a, --action <ACTION>    the kind of device action to trigger
    -h, --help               Print help information
    -n, --dry-run            do not actually trigger the device events
    -t, --type <TYPE>        the enumerator type, can be devices (default) or subsystems
    -v, --verbose            print searched devices by enumerator
```

参数：
    `<DEVICES>...`    以`/sys`或`/dev`开头的设备路径

选项：
    -h, --help              显示帮助信息

    -a, --action `<ACTION>`   指定设备事件的动作类型

    -t, --type `<TYPE>`       指定搜索的设备类型，可以是devices（设备）或者subsystems（子系统）

    -v, --verbose           打印搜索到的设备

    -n, --dry-run           不会实际触发设备事件，配合--verbose选项使用时，可以查看系统中的设备清单

### devctl test-builtin [OPTIONS] <BUILTIN\> <SYSPATH\>

测试内置命令在某个设备上的执行效果。

```shell
# devctl test-builtin --help
devctl-test-builtin
Test builtin command on a device

USAGE:
    devctl test-builtin [OPTIONS] <BUILTIN> <SYSPATH>

ARGS:
    <BUILTIN>    builtin command
    <SYSPATH>    device syspath

OPTIONS:
    -a, --action <ACTION>    device action
    -h, --help               Print help information
```

参数：

​	`<BUILTIN>`	目前支持的内置命令包括：`blkid`、`input_id`、`kmod`、`net_id`、`net_setup_link`、`path_id`、`usb_id`

​	`<SYSPATH>`	设备的`sysfs`路径

选项：

​	-a, --action `<ACTION>`	指定设备事件的动作类型，包括：`add`、`change`、`remove`、`move`、`online`、`offline`、`bind`和`unbind`

​	-h, --help		显示帮助信息
