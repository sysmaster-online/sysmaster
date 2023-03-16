# [Service] 配置

## ExecStart

`ExecStart`是Service唯一必须配置的选项，用于配置Service需要执行的命令。

### ExecStart配置的限制

1. 配置的命令必须为绝对路径
2. 除非服务的类型配置为`OneShot`，否则只允许配置一条命令
3. 命令的绝对路径前支持添加前缀：`-`（暂不支持其他systemd支持的前缀，如`@`，`:`)。前缀`-`的含义是，即使后面列出的命令执行失败也当作成功处理。

## WatchdogSec

配置软件狗的定时时间，时间单位为秒， 当值大于0时，启用软件狗，应用通过发送notify消息喂狗， 定时时间内收到"WATCHDOG=1"消息代表应用正常， 收到“WATCHDOG=trigger"消息停止应用，收到“WATCHDOG_USEC=15”消息表示将定时时间修改为15秒。

## Restart

配置在服务退出或终止时，是否重新启动服务，可以配置为`no`。`on-success`，`on-failure`，`on-watchdog`, `on-abnormal`, `on-abort`, `always`, 默认值为`no`。
    `no`: 代表服务启动时， 不重新启动服务。
    `on-success`: 当服务正常退出时重新启动服务。 1、退出码为0。 2、其他符合预期的退出码或信号。
    `on-failure`: 当服务非正常退出时重新启动服务。 1、退出码非0。 2、非预期的信号或超时导致服务退出等。
    `on-watchdog`: 当watchdog超时导致进程退出时重新启动服务。
    `on-abnormal`: 当服务超时或接收到异常的退出信号时候，重新启动服务。
    `on-abort`: 当服务因未捕获的异常退出时重新拉起服务。
    `always`: 无论服务因何原因退出都重新拉起服务。

## RestartSec

当服务退出时， 间隔多长时间重新拉起服务， 配置为正整数， 单位为秒。

## RestartPreventExitStatus

配置进程的退出码或信号， 当服务进程的退出码或信号符合此选项时不重新拉起服务， 此时忽略Restart的配置。 可以配置为整数或信号名， 中间以空格分开默， 默认为空字符串。
如RestartPreventExitStatus=“1 2 SIGKILL”， 当前信号仅支持以SIG开头的信号。

## ExecReload

主要用于服务重新加载配置文件等操作， 配置格式如`ExecStart`, 可以配置为空， 当服务active状态时， 才会生效。

## EnvironmentFile

设置环境变量的文件读取路径， 只支持绝对路径， 配置多个路径时以`;`隔开， 如果路径以`-`开头， 则忽略该文件， 文件中的内容格式为`key=value`， 若为空行或以#开头则忽略该行。

## KillSignal

设置杀死进程的第一步使用的信号, 配置类型为字符串。默认值为`SIGTERM`信号。
