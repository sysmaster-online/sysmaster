# devctl使用手册

## 1. 简介

`devctl`是`devmaster`的客户端管理工具，用来控制`devmaster`的行为、模拟设备事件、调试规则等等。

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
    info            Query sysfs or the devmaster database
    monitor         Monitor device events from kernel and userspace
    kill            Kill all devmaster workers
    trigger         Trigger a fake device action, then the kernel will report an uevent
    test-builtin    Test builtin command on a device
    control         Control the devmaster daemon
    hwdb            The sub-command 'hwdb' is deprecated, and is left for backwards
                        compatibility. Please use sysmaster-hwdb instead
    help            Print this message or the help of the given subcommand(s)
```

## 2. 选项
### devctl info [OPTIONS] [DEVICES]
从sysfs路径或devmaster数据库中查询设备信息。DEVICES参数用于指定一个或多个设备，它可以是 一个设备名(必须以 /dev/ 开头)、 一个 sys 路径(必须以 /sys/ 开头)
```shell
OPTIONS:
    -a, --attribute-walk
        按照devmaster中匹配规则的格式，显示所有可用于匹配该设备的sysfs属性： 从该设备自身开始，沿着设备树向上回溯(一直到树根)， 显示沿途每个设备的sysfs属性。

    -c, --cleanup-db
        清除devmaster数据库。

    -d, --device-id-of-file <DEVICE_ID_OF_FILE>
        显示文件所在底层设备的主/次设备号。如果使用了此选项，那么将忽略所有位置参数。

    -e, --export-db
        导出devmaster数据库的全部内容。

    -h, --help
        显示简短的帮助信息并退出。

    -q, --query <QUERY>
        提取特定类型的设备信息。TYPE 可以是下列值之一：name, symlink, path, property, all(默认值)
        name                     设备节点名称
        symlink                  指向设备节点的软链接
        path                     设备路径
        property or env          设备属性
        all                      所有信息

    -r, --root
        以绝对路径显示 --query=name 与 --query=symlink 的查询结果。

    -x, --export
        以 key='value' 的格式输出此设备的属性(注意，值两边有单引号界定)。 仅在指定了 --query=property --query=env或 --device-id-of-file=FILE 的情况下才有效。
```

### devctl monitor [OPTIONS]
监听内核上报的`uevent`事件和`devmaster`处理完设备后发出的事件，分别以`KERNEL`和`USERSPACE`作为前缀进行区分。

```shell
OPTIONS:
    -e, --environment
        显示事件的各属性 (与-p功能一致)。

    -h, --help
        显示简短的帮助信息并退出。

    -k, --kernel
        显示"KERNEL"事件。

    -p, --property
        显示事件的各属性。

    -s, --subsystem-match <SUBSYSTEM_MATCH>
        根据 subsystem[/devtype] 对事件进行过滤，仅显示与"subsystem[/devtype]"匹配的事件。 如果多次使用此选项，那么表示以 OR 逻辑连接每个匹配规则，即任一匹配的设备都会被监视。

    -t, --tag-match <TAG_MATCH>
        根据设备标签对事件进行过滤，仅显示与"标签"匹配的事件。 如果多次使用此选项，那么表示以 OR 逻辑连接每个匹配规则， 即拥有任一指定标签的设备都会被监视。

    -u, --userspace
        显示"devmaster"处理完设备后发出的事件。
```

### [deprecated] devctl kill
使`devmaster`终止所有`worker`，正在运行中的`worker`会等待执行完当前任务再终止，期间无法再接收新的任务。

参数：无

选项：
    -h, --help  显示帮助信息

### devctl trigger [OPTIONS] [DEVICES]...
模拟一个设备动作，使内核上报对应的uevent事件，用于重放内核初始化过程中的冷插(coldplug)设备事件。

```shell
OPTIONS:
    -a, --attr-match <ATTR_MATCH>
        仅触发匹配 ATTR 属性的设备事件。如果同时还指定了"=VALUE"，那么表示仅触发 ATTR 属性匹配 VALUE 的设备事件。 注意，可以在 VALUE 中使用shell风格的通配符。 如果多次使用此选项，那么表示以 AND 逻辑连接每个匹配规则， 即只有匹配所有指定属性的设备才会被触发。

    -A, --attr-nomatch <ATTR_NOMATCH>
        不触发匹配 ATTR 属性的设备事件。如果同时还指定了"=VALUE"，那么表示不触发 ATTR 属性匹配 VALUE 的设备事件。 注意，可以在 VALUE 中使用shell风格的通配符。 如果多次使用此选项，那么表示以 AND 逻辑连接每个匹配规则， 即只有不匹配指定属性的设备才会被触发。

    -b, --parent-match <PARENT_MATCH>
        仅触发给定设备及其所有子设备的事件。PARENT_MATCH 是该设备在 /sys 目录下的路径。 如果多次使用此选项，那么仅以最后一个为准。

    -c, --action <ACTION>
        指定触发哪种类型的设备事件，ACTION 可以是下列值之一： add, remove, change(默认值), move, online, offline, bind, unbind。

    -g, --tag-match <TAG_MATCH>
        仅触发匹配 TAG_MATCH 标签的设备事件。如果多次使用此选项， 那么表示以 AND 逻辑连接每个匹配规则，即只有匹配所有指定标签的设备才会被触发。

    -h, --help
        显示简短的帮助信息并退出。

    -n, --dry-run
        并不真正触发设备事件。

    --name-match <NAME_MATCH>
        仅触发/dev name匹配NAME_MATCH的设备事件。如果多次使用此选项，那么表示以 OR 逻辑连接每个匹配规则，即任意匹配/dev name的设备都会被触发。

    -p, --property-match <PROPERTY_MATCH>
        仅触发那些设备的 PROPERTY 属性值匹配 VALUE 的设备事件。注意，可以在 VALUE 中使用shell风格的通配符。 如果多次使用此选项，那么表示以 OR 逻辑连接每个匹配规则，即匹配任意一个属性值的设备都会被触发。

    -s, --subsystem-match <SUBSYSTEM_MATCH>
        仅触发匹配 SUBSYSTEM 子系统的设备事件。 可以在 SUBSYSTEM 中使用shell风格的通配符。 如果多次使用此选项，那么表示以 OR 逻辑连接每个匹配规则， 即所有匹配的子系统中的设备都会被触发。

    -S, --subsystem-nomatch <SUBSYSTEM_NOMATCH>
        不触发匹配 SUBSYSTEM 子系统的设备事件。 可以在 SUBSYSTEM 中使用shell风格的通配符。 如果多次使用此选项，那么表示以 AND 逻辑连接每个匹配规则， 即只有不匹配所有指定子系统中的设备才会被触发。

    -t, --type <TYPE>
        仅触发特定类型的设备， TYPE 可以是下列值之一：
        devices                     sysfs下的设备对象 (默认值)
        subsystems                  sysfs下的总线或驱动对象
        all                         所有类型

    --uuid
        显示uevent事件的uuid。

    -v, --verbose
        显示被触发的设备列表。

    -w, --settle
        除了触发设备事件之外，还要等待这些事件完成。注意，此选项仅等待该命令自身触发的事件完成， 而 devctl settle 则要一直等到所有设备事件全部完成。

    -y, --sysname-match <SYSNAME_MATCH>
        仅触发设备/sys name(也就是该设备在 /sys 路径下最末端的文件名)匹配 SYSNAME_MATCH 的设备事件。注意，可以在 SYSNAME 中使用shell风格的通配符。 如果多次使用此选项，那么表示以 OR 逻辑连接每个匹配规则，即匹配任意一个sys名称的设备都会被触发。
```

### devctl settle [OPTIONS]
监视devmaster事件队列，并且在所有事件全部处理完成之后退出。

```shell
OPTIONS:
    -E, --exit-if-exists <EXIT_IF_EXISTS>
        如果 FILE 文件存在，则停止等待。
    -h, --help
        显示简短的帮助信息并退出。
    -t, --timeout <TIMEOUT>
        最多允许花多少秒等候事件队列清空。 默认值是120秒。设为 0 表示仅检查事件队列是否为空，并且立即返回。
```


### devctl test-builtin [OPTIONS] <BUILTIN\> <SYSPATH\>
测试内置命令在某个设备上的执行效果。

```shell
ARGS:
    <BUILTIN>
        builtin 子命令，目前支持的内置命令包括：
        blkid           文件系统和分区探测
        hwdb            设备硬件数据库属性
        input_id        input设备属性
        keyboard        更改键位设置，使能对应硬件的特殊键位。
        kmod            内核模块加载
        net_id          显示网络设备属性
        net_setup_link  配置网络链接
        path_id         显示设备的PATH_ID属性
        usb_id          显示USB设备属性
    <SYSPATH>
        设备 /sys 路径

OPTIONS:
    -a, --action <ACTION>
        指定设备事件的动作类型，包括：add、change、remove、move、online、offline、bind 和 unbind
    -h, --help
        显示简短的帮助信息并退出。
```

参数：

​	`<BUILTIN>`	目前支持的内置命令包括：`blkid`、`input_id`、`kmod`、`net_id`、`net_setup_link`、`path_id`、`usb_id`

​	`<SYSPATH>`	设备的`sysfs`路径

选项：

​	-a, --action `<ACTION>`	指定设备事件的动作类型，包括：`add`、`change`、`remove`、`move`、`online`、`offline`、`bind`和`unbind`

​	-h, --help		显示帮助信息

### devctl control [OPTIONS]
控制devmaster守护进程的内部状态。

```shell
OPTIONS：
    -e, --exit
        向 devmaster 发送"退出"信号并等待其退出。
    -h, --help
        显示简短的帮助信息并退出。
```

### devctl hwdb [OPTIONS]
hwdb子命令已弃用，保留是为了向后兼容性。请使用sysmaster-hwdb替代devctl hwdb

```shell
OPTIONS：
    -h, --help
        显示简短的帮助信息并退出。
    --path <PATH>
        自定义.hwdb文件的路径。
    -r, --root <ROOT>
        指定根文件系统的路径。
    -s, --strict <STRICT>
        在更新时，如果遇到任何解析错误，那么就返回非零退出码表示出错。
    -t, --test <TEST>
        查询二进制格式的硬件数据库，并显示查询结果
    -u, --update
        更新二进制格式的硬件数据库。
    --usr
        输出到 /usr/lib/devmaster 目录中(而不是默认的 /etc/devmaster 目录)。
```
