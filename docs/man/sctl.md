# sctl命令

## start

通过`sctl start`命令启动一个或多个unit，如果启动多个unit，它们之间采用空格分隔。

### 命令的返回值：

当命令执行成功时，返回0，否则，返回一个正数表示对应的linux标准错误码。

**注意：**返回值为0，并不意味着单元被成功启动或者单元的启动状态为`active`，只是说明启动该服务的动作已执行成功。

## stop

通过`sctl stop`命令关闭一个或多个unit，如果关闭多个unit，它们之间采用空格分隔。

### 命令的返回值：

当命令执行成功时，返回0，否则，返回一个正数表示对应的linux标准错误码。

**注意：**与`start`命令类似，返回值为0,并不意味着单元被成功关闭或者单元的关闭状态为`inactive`，只是说明关闭服务的动作已经执行成功。

## status

通过`sctl status`命令获取一个或多个unit的当前状态。

## 返回值

当命令执行成功时，返回0，否则，返回一个正数表示对应的linux标准错误码。

**注意：请不要通过命令的返回值判断服务的状态，而是通过sctl status命令。** `systemctl status`的返回值会根据单元状态变化，例如：当服务状态为`failed`时，`systemctl status`命令的返回值为3。sysmaster不支持该特性，在上述场景下`sctl status`命令的返回值为0。原因如下：

1. 无论status命令返回的单元状态是什么，status命令已经执行成功，并且返回了用户需要的结果，那么其返回值应该为0。
2. systemd根据单元状态修改返回值的逻辑不统一，如`systemctl status`的返回值受单元状态变化，但是`systemctl start`却不会。
3. systemd不建议通过命令的返回值判断服务的状态，请参考：<https://www.freedesktop.org/software/systemd/man/systemctl.html#Exit%20status>
