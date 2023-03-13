# devctl命令行工具

## 1.简介

devctl是devmaster的管理工具，其功能包括控制devmaster行为、监听设备事件、调试等等。

### 选项

-h, --help
    显示帮助信息

-V, --version
    显示版本信息



## 2.子命令

### devctl monitor
监听内核上报的uevent事件和devmaster处理完设备后发出的事件，分别以`KERNEL`和`USERSPACE`作为前缀进行区分。

参数：无

选项：
    -h, --help  显示帮助信息

### [deprecated] devctl kill
使devmaster终止所有worker，正在运行中的worker会等待执行完当前任务再终止，期间无法再接收新的任务。

参数：无

选项：
    -h, --help  显示帮助信息

### [deprecated] devctl test <DEVNAME>
向devmaster发送模拟设备，调试devmaster的框架功能。

参数：
    <DEVNAME>   模拟设备的名称

选项：
    -h, --help  显示帮助信息

### devctl trigger [OPTIONS] [DEVICES]...
模拟一个设备动作，使内核上报对应的uevent事件，用于重放内核初始化过程中的冷插(coldplug)设备事件。

参数：
    <DEVICES>...    以/sys或/dev开头的设备路径

选项：
    -h, --help              显示帮助信息
    -a, --action <ACTION>   指定设备事件的动作类型
    -t, --type <TYPE>       指定enumerator的类型，可以是devices（默认）或者subsystems
    -v, --verbose           打印enumerator搜索到的设备
    -n, --dry-run           不会实际触发设备事件，配合--verbose选项使用时，可以查看系统中的设备列表
