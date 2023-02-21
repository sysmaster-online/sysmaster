# sysmaster [Unit] 配置

## Action

sysmaster兼容systemd的SuccessAction、FailureAction、StartLimitAction等配置。这些配置允许某个unit的状态发生变更时修改操作系统的状态，例如重启、关机或退出。

### SuccessAction/FailureAction

配置当unit结束（SuccessAction）或进入失败状态（FailureAction）时采取的动作。可以配置的值包括： `none`，`reboot`，`reboot-force`，`reboot-immediate`，`poweroff`，`poweroff-force`，`poweroff-immediate`，`exit`和`exit-force`。

当配置为`none`时，不触发任何动作，所有unit的默认值为`none`。`reboot`，`poweroff`，`exit`会分别触发`reboot.target`，`poweroff.target`，`exit.target`，与正常的系统重启、关机、退出流程一致。`reboot-force`，`poweroff-force`，`exit-force`会分别触发sysmaster以相应的状态退出，强行杀死服务及相关进程。`reboot-immediate`，`poweroff-immediate`会分别触发系统立即重启、关机，直接调用`reboot(2)`。

### StartLimitAction

配置当unit触发启动限制时采取的动作。配置的值与采取的动作与`SuccessAction`、`FailureAction`一致。

触发启动限制指：sysmaster最多允许单个unit在`StartLimitInterval`时间内启动`StartLimitBurst`次。

### StartLimitInterval

限制启动的时间区间, 单位为秒， 默认值为10秒。

### StartLimitBurst

单位时间内最多的启动次数， 默认值为5。 只要`StartLimitInterval`与`StartLimitBurst`其中一项配置为0时不启动限速。


### WatchdogSec

配置软件狗的定时时间， 当值大于0时，启用软件狗，应用通过发送notify消息喂狗， 定时时间内收到"WATCHDOG=1"消息代表应用正常， 收到“WATCHDOG=trigger"消息停止应用，收到
“WATCHDOG_USEC=15”消息表示将定时时间修改为15秒。

## 顺序和依赖
