# Unit 配置

## 条件动作

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

### JobTimeoutAction，JobTimeoutSec

`JobTimeoutAction`配置unit的job运行超时时采取的动作，配置的值与采取的动作与`SuccessAction`、`FailureAction`一致。

`JobTimeoutSec`配置unit的job运行超时时间，单位是`秒`。systemd禁用`JobTimeoutSec`的配置为`infinity`，sysmaster与之不同，禁用`JobTimeoutSec`需要配置为`0`。

## 顺序和依赖

sysmaster支持配置单元之间的依赖关系，可以配置的值为`;`分隔的单元。如`After="foo.service;bar.target"`。需要注意：

1. 单元之间不允许有多余的空格
2. 仅支持通过`;`分隔，与systemd的空格分隔存在差异
3. 不支持systemd同一配置项配置多次的场景，如:
    After="foo.service"
    After="bar.service"

### After/Before

`After`和`Before`能够配置同时启动的单元的先后顺序。如`foo.service`配置了`After="bar.service" Wants="bar.service"`，`foo.service`将在`bar.service`启动完成后启动。需要注意：

1. `After`和`Before`仅确定启动的先后顺序，不自动添加依赖。在上面的例子中如果没有配置`Wants="bar.service"`，`foo.service`也能正常启动，且不会自动拉起`bar.service`。
2. 后序单元不关注前序单元的启动结果，这意味着在上面的例子中，即使`bar.service`启动失败，`foo.service`依旧能正常拉起。

`After`和`Before`的依赖相反。在关机阶段，依赖反转。

### Wants/Requires/Requisite/PartOf/BindsTo/Conflicts

这些配置项均用于配置单元之间的依赖，以`foo.service`配置`Wants/Requisite...="bar.service"`为例详细说明。

`Wants`：配置一个单元对另一个单元的依赖。在启动`foo.service`时，将自动拉起`bar.service`。但是`bar.service`的启动结果不影响`foo.service`。

`Requires`：与`Wants`类似，区别在于如果`bar.service`启动失败，`foo.service`也同时进入失败状态。重启或关闭`bar.service`会传递到`foo.service`，使`foo.service`跟随重启或关闭。

`Requisite`：与`Requires`类似，区别在于启动`foo.service`时，如果`bar.service`还未启动成功，那么`foo.service`会直接失败。**注意：`Requisite`必须与`After`配合使用，否则启动`bar.service`不检查`foo.service`，这与systemd行为一致。**

`PartOf`：与`Requires`类似，区别在于依赖仅影响重启或关闭。

`BindsTo`：与`Requires`类似，但依赖更强，当`bar.service`的状态突然发生变化时，`foo.service`会跟随立即变化。

`Conflicts`：`foo.service`与`bar.service`的状态相反。启动`foo.service`将关闭`bar.service`，关闭`foo.service`会启动`bar.service`。

`Wants`和`Requires`除了支持通过`.service/.target/.socket`等单元配置文件配置，也允许在`/etc/sysmaster/`或`/usr/lib/sysmaster`目录下创建`单元名.wants/单元名.requires`目录，并在里面添加指向依赖单元的软链接。例如为了给`foo.service`配置`Wants="bar.service"`，可以创建`/etc/sysmaster/foo.service.wants`目录，并在该目录内创建`bar.service -> /etc/sysmaster/bar.service`的软链接。

### OnFailure/OnSuccess

`OnFailure`：配置当一个unit启动失败或成功结束后，对其他服务的影响。以`foo.service`配置`OnFailure/OnSuccess="bar.service"`为例，当`foo.service`启动失败或成功结束后，将自动拉起`bar.service`。拉起`bar.service`会生成一个Start类型的job,该job的模式可配置为`fail`， `replace`，`replace-irreversibly`，`isolate`，`flush`，`ignore-dependencies`或`ignore-requirements`。

## 启动检查

sysmaster支持配置`Condition...`和`Assert...`进行启动检查，当条件不满足时，停止启动流程。

### ConditionPathExists

检查文件是否存在。只有`ConditionPathExists=`后配置的绝对路径存在时，检查通过，启动流程正常；如果配置的绝对路径不存在，则检查失败，停止单元的启动流程。支持在配置的绝对路径前添加`!`，反转检查结果。

### ConditionPathIsReadWrite

与`ConditionPathExists`类似，检查配置的绝对路径所属的文件系统是否为可读可写。

## 其他配置

### RefuseManualStart/RefuseManualStop

`RefuseManualStart`/`RefuseManualStop`：配置单元是否拒绝通过`sctl start/stop`的形式手动启动/关闭。默认配置为`false`，即允许手动启动/关闭。该配置不影响通过依>赖关系解析启动/关闭服务。
