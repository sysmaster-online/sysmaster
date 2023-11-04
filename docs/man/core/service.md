# Service 配置


## Type

service服务的类型，当前支持`simple`、`forking`、`oneshot`、`notify`，默认值为`simple`。

`simple`： 拉起service服务时，当执行fork成功即认为服务启动成功。

`forking`： 代表ExecStart的进程会调用fork系统调用，此时父进程退出之后认为该服务启动成功。此时建议配置PIDFile选项，通过该选项获取主服务进程的pid。

`oneshot`： 主服务进程退出之后即认为服务启动完成，此服务类型需同时设置`RemainAfterExit`，允许配置多条命令，通常用于短时间运行的服务。

`notify`： 此服务需要主进程通过sd_notify发送通知消息，当前支持的notify消息包括`MAINPID=`、`ERRNO=`、`STOPPING=1`、`WATCHDOG=`、`WATCHDOG_USEC`。

    `MAINPID=`：通过此消息接受主服务进程的pid。
    `ERRNO=`：接受主服务进程的错误码。
    `STOPPING=1`：当参数为1时，且当前服务在Running状态则停止当前服务。
    `WATCHDOG=`：当参数为1时，则启动watchdog的定时器，当参数为trigger时，则服务进程StopWatchdog状态。


## ExecCondition、ExecStartPre、ExecStart、ExecStop、ExecStartPost

* 类型：字符串

服务在不同的启动阶段执行的命令，多条命令使用`;`分隔。`ExecStart`是Service唯一必须配置的选项，用于配置Service需要执行的命令。

**注意：** 为了避免`;`解析为命令参数，`;`作为分隔符使用时需要在前后添加空格。例如：

|配置的命令|解析结果|
|-|-|
|ExecStart="/bin/echo foo; /bin/echo bar"|命令路径：/bin/echo，命令参数："foo;", "/bin/echo", "bar"，执行结果：打印"foo; /bin/echo bar"|
|ExecStart="/bin/echo foo;/bin/echo bar"|命令路径：/bin/echo，命令参数："foo;/bin/echo", "bar"，执行结果：打印"foo;/bin/echo bar"|
|ExecStart="/bin/echo foo ; /bin/echo bar"|解析为两条命令。</br> 命令1路径：/bin/echo，命令1参数："foo"，命令1执行结果：打印"foo"。</br> 命令2路径：/bin/echo，命令2参数："bar"，命令2执行结果：打印"bar"|

### ExecStart配置的限制

1. 配置的命令必须为绝对路径
2. 除非服务的类型配置为`OneShot`，否则只允许配置一条命令
3. 命令的绝对路径前支持添加前缀：`-`（暂不支持其他systemd支持的前缀，如`@`，`:`)。前缀`-`的含义是，即使后面列出的命令执行失败也当作成功处理。
4. 当前不支持命令行中的环境变量展开，即与systemd添加`:`前缀的行为一致。即同时配置`Environment={name="test"}, ExecStart="/bin/echo $name"`，sysMaster输出为"$name"，而systemd输出为"test"。如果systemd的服务配置文件修改为"ExecStart=:/bin/echo $name"，则输出也为"$name"。

## PIDFile

当Type类型为`forking`时使用，用于获取主服务进程的pid。

## RemainAfterExit

支持的值为true或false, 当配置为true时，当主服务进程退出时，服务状态仍然为active状态。

## NonBlocking

* 类型：布尔值

该配置仅对socket激活的服务有效，设置从socket继承的文件描述符的O_NONBLOCK标志位。默认值为`false`。

## NotifyAccess

配置类型为字符串，支持`none`、`main`，当Type为Notify时默认值为`main`。当前功能未实现。

## Sockets

配置类型为字符串，当需要配置多个时以；号隔开，表示该服务需要从socket继承套接子。

## KillMode

当需要停止服务进程时，杀死服务进程的方法，取值范围如下： `control-group`、`process`、`mixed`，默认值为`control-group`。

    `control-group`: 表示杀死该服务的cgroup内的所有进程。
    `process`: 仅杀死主进程。
    `mixed`:  表示只向该服务的cgroup内的进程发送SigKill信号。

## WatchdogSec

配置软件狗的定时时间，时间单位为秒，当值大于0时，启用软件狗，应用通过发送notify消息喂狗，定时时间内收到"WATCHDOG=1"消息代表应用正常，收到“WATCHDOG=trigger"消息停止应用，收到“WATCHDOG_USEC=15”消息表示将定时时间修改为15秒。

## Restart

配置在服务退出或终止时，是否重新启动服务，可以配置为`no`,`on-success`，`on-failure`，`on-watchdog`, `on-abnormal`, `on-abort`, `always`, 默认值为`no`。

    `no`: 代表服务启动失败时，不重新启动服务。
    `on-success`: 当服务正常退出时重新启动服务。正常退出码有两种情况: 1、退出码为0。 2、其他符合预期的退出码或信号。
    `on-failure`: 当服务非正常退出时重新启动服务。非正常退出码 1、退出码非0。 2、非预期的信号或超时导致服务退出等。
    `on-watchdog`: 当watchdog超时导致进程退出时重新启动服务。
    `on-abnormal`: 当服务超时或接收到异常的退出信号时候，重新启动服务。
    `on-abort`: 当服务因未捕获的异常退出时重新拉起服务。
    `always`: 无论服务因何原因退出都重新拉起服务。

## RestartSec

* 类型：数值

当服务退出时，间隔多长时间重新拉起服务，配置为正整数，单位为微秒。

## RestartPreventExitStatus

配置进程的退出码或信号，当服务进程的退出码或信号符合此选项时不重新拉起服务，此时忽略Restart的配置。可以配置为整数或信号名，中间以空格分开默，默认为空字符串。
如RestartPreventExitStatus=“1 2 SIGKILL”，当前信号仅支持以SIG开头的信号。

## ExecReload

主要用于服务重新加载配置文件等操作，配置格式如`ExecStart`，可以配置为空，当服务active状态时，才会生效。

## Environment

配置进程的环境变量，每个环境变量采用`Key=Value`键值对的格式，多个环境变量通过空格分隔，例如：`Environment=FOO=foo BAR=bar`。

支持使用双引号包裹`Value`，这常用于`Value`内包含空格的场景，例如：`Environment=FOO="foo foo" BAR="bar bar"`。如果`Value`包含`"`，可以使用反斜杠转义，例如：`Environment=FOO=\"foo`。

合法的`Key`仅支持大小写字母、阿拉伯数字及下划线`_`。

### 注意：

1. 环境变量不会默认在命令行中展开，和systemd在ExecXXX字段显式配置`:`的行为一致。
2. 当解析到非法的键值对时，sysmaster将停止解析，使用当前已解析到的合法值。

## EnvironmentFile

设置环境变量的文件读取路径，只支持绝对路径，配置多个路径时以`;`隔开，如果路径以`-`开头，则忽略该文件，文件中的内容格式为`key=value`，若为空行或以#开头则忽略该行。

## KillSignal

设置杀死进程的第一步使用的信号, 配置类型为字符串。默认值为`SIGTERM`信号。

## TimeoutSec

服务启动或停止时的超时时间，单位为秒，取值范围为0~u64::MAX, 当值为0或u64::Max时，不启动定时器。当`TimeoutSec`的值不为0且`TimeoutStartSec`或`TimeoutStopSec`值为0时，则将`TimeoutStartSec`或`TimeoutStopSec`的值更新为`TimeoutSec`选项的

## TimeoutStartSec

服务启动时的超时时间，单位为秒，取值范围为0~u64::MAX, 当值为0或u64::Max时，不启动定时器。当执行`Condition`、`StartPre`、`Start`、`StartPost`、`Reload`命令时的超时时间。

## TimeoutStopSec

服务停止时的超时时间，单位为秒，取值范围为0~u64::MAX, 当值为0或u64::Max时，不启动定时器。当执行`Stop`、`StopPost`命令时的超时时间。

## LimitCORE、LimitNOFILE、LimitNPROC

设置进程的资源限制，参考[setrlimit(2)](https://www.man7.org/linux/man-pages/man2/setrlimit.2.html)，配置类型为字符串。

当前支持两种格式，(1) 不包括`':'`的当个字符串，此时soft、hard设置为同一个值，(2) 使用`':'`分割的两个字符串，soft为`:`前的值， hard为`:`后的值。

单个值只支持配置为数值型或“infinity”字符串，配置为“infinity”时资源限制设置为`ulimit`。
