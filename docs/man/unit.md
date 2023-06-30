# Unit 配置

## Unit描述

### Description

* 类型：字符串

允许配置为一个字符串，说明该unit的主要功能。

### Documentation

* 类型：字符串

允许配置为一个字符串，说明该unit的文档链接。

## 条件动作

sysmaster兼容systemd的SuccessAction、FailureAction、StartLimitAction等配置。这些配置允许某个unit的状态发生变更时修改操作系统的状态，例如重启、关机或退出。

### SuccessAction/FailureAction

* 类型：字符串

配置当unit结束（SuccessAction）或进入失败状态（FailureAction）时采取的动作。可以配置的值包括： `"none"`，`"reboot"`，`"reboot-force"`，`"reboot-immediate"`，`"poweroff"`，`"poweroff-force"`，`"poweroff-immediate"`，`"exit"`和`"exit-force"`。

当配置为`none`时，不触发任何动作，所有unit的默认值为`"none"`。`"reboot"`，`"poweroff"`，`"exit"`会分别触发`reboot.target`，`poweroff.target`，`exit.target`，与正常的系统重启、关机、退出流程一致。`"reboot-force"`，`"poweroff-force"`，`"exit-force"`会分别触发sysmaster以相应的状态退出，强行杀死服务及相关进程。`"reboot-immediate"`，`"poweroff-immediate"`会分别触发系统立即重启、关机，直接调用[reboot(2)](https://man7.org/linux/man-pages/man2/reboot.2.html)。

### StartLimitAction

* 类型：字符串

配置当unit触发启动限制时采取的动作。配置的值与采取的动作与`SuccessAction`、`FailureAction`一致。

触发启动限制指：sysmaster最多允许单个unit在`StartLimitInterval`时间内启动`StartLimitBurst`次。

### StartLimitInterval

* 类型：数值

限制启动的时间区间, 单位为秒， 默认值为10秒。

### StartLimitBurst

* 类型：数值

单位时间内最多的启动次数， 默认值为5。 只要`StartLimitInterval`与`StartLimitBurst`其中一项配置为0时不启动限速。

### JobTimeoutAction，JobTimeoutSec

* 类型：字符串

`JobTimeoutAction`配置unit的job运行超时时采取的动作，配置的值与采取的动作与`SuccessAction`、`FailureAction`一致。

`JobTimeoutSec`配置unit的job运行超时时间，单位是`秒`。systemd禁用`JobTimeoutSec`的配置为`"infinity"`，sysmaster与之不同，禁用`JobTimeoutSec`需要配置为`"0"`。

## 顺序和依赖

sysmaster支持配置单元之间的顺序及依赖关系，可以配置的值为`;`分隔的单元。如`After="foo.service;bar.target"`。需要注意：

1. 单元之间不允许有多余的空格
2. 仅支持通过`;`分隔，与systemd的空格分隔存在差异
3. 不支持systemd同一配置项配置多次的场景，如:
    After="foo.service"
    After="bar.service"

### After/Before

* 类型：字符串

`After`和`Before`能够配置同时启动的单元的先后顺序。如`foo.service`配置了`After="bar.service" Wants="bar.service"`，`foo.service`将在`bar.service`启动完成后启动。需要注意：

1. `After`和`Before`仅确定启动的先后顺序，不自动添加依赖。在上面的例子中如果没有配置`Wants="bar.service"`，`foo.service`也能正常启动，且不会自动拉起`bar.service`。
2. 后序单元不关注前序单元的启动结果，这意味着在上面的例子中，即使`bar.service`启动失败，`foo.service`依旧能正常拉起。

`After`和`Before`的依赖相反。在关机阶段，依赖反转。

### Wants/Requires/Requisite/PartOf/BindsTo/Conflicts

这些配置项均用于配置单元之间的依赖，以`foo.service`配置`Wants/Requisite...="bar.service"`为例详细说明。

`Wants`：配置一个单元对另一个单元的依赖。在启动`foo.service`时，将自动拉起`bar.service`。但是`bar.service`的启动结果不影响`foo.service`。

`Requires`：与`Wants`类似，区别在于如果`bar.service`启动失败，`foo.service`也同时进入失败状态。重启或关闭`bar.service`会传递到`foo.service`，使`foo.service`跟随重启或关闭。

`Requisite`：与`Requires`类似，区别在于启动`foo.service`时，如果`bar.service`还未启动成功，那么`foo.service`会直接失败。**注意：`Requisite`必须与`After`配合使用，否则启动`bar.service`不检查`foo.service`，这与systemd行为一致。**

`PartOf`：与`Requires`类似，区别在于依赖仅影响单元的重启或关闭。

`BindsTo`：与`Requires`类似，但依赖更强，当`bar.service`的状态突然发生变化时，`foo.service`会跟随立即变化。

`Conflicts`：`foo.service`与`bar.service`的状态相反。启动`foo.service`将关闭`bar.service`，启动`bar.service`会关闭`foo.service`。

`Wants`和`Requires`除了支持通过`.service/.target/.socket`等单元配置文件配置，也允许在`/etc/sysmaster/system/`或`/usr/lib/sysmaster/system/`目录下创建`单元名.wants/单元名.requires`目录，并在里面添加指向依赖单元的软链接。例如为了给`foo.service`配置`Wants="bar.service"`，可以创建`/etc/sysmaster/system/foo.service.wants`目录，并在该目录内创建`bar.service -> /etc/sysmaster/system/bar.service`的软链接。

### OnFailure/OnSuccess

* 类型：字符串

`OnFailure`：配置当一个unit启动失败或成功结束后，对其他服务的影响。以`foo.service`配置`OnFailure/OnSuccess="bar.service"`为例，当`foo.service`启动失败或成功结束后，将自动拉起`bar.service`。拉起`bar.service`会生成一个Start类型的job,该job的模式可配置为`fail`， `replace`，`replace-irreversibly`，`isolate`，`flush`，`ignore-dependencies`或`ignore-requirements`。

### DefaultDependencies

* 类型：布尔值

`DefaultDependencies`配置是否为单元添加缺省依赖，默认值为`true`。缺省依赖如下：

1. 所有类型的单元统一添加`Conflict="shutdown.target"`，`Before="shutdown.target"`。
2. 针对不同类型的单元，额外添加以下依赖

> * 对`.service`，添加`Requires="sysinit.target"`，`After="sysinit.target"`，`After="basic.target"`。
> * 对`.socket`，添加`Requires="sysinit.target"`，`After="sysinit.target"`，`After="socket.target"`。
> * 对`.target`，对target配置的`Requires=`、`Wants=`补全`After=`。

## 启动检查

sysmaster支持配置`Condition...`和`Assert...`进行启动检查，当条件不满足时，停止启动流程。所有配置均支持通过在值前面添加`!`反转检查结果。

### ConditionACPower

* 类型：可选的布尔值

检查操作系统是否连接交流电源。可以配置为`false`，`true`。配置为`true`时，当操作系统至少一个接口连接了交流电，或者无法确定是否有连接时，检查通过。配置为`false`时，当成功检查到所有接口都没有连接交流电时，检查通过。如果不配置，跳过该检查。配置为其他的值，会导致解析失败。

### ConditionCapability

* 类型：字符串

检测sysmaster是否支持给定的权能，允许配置引号括起来的权能名称，如`"CAP_CHOWN"`。该配置仅允许配置一个权能。通过读取`/proc/self/status/`的`CapBnd`检查sysmaster的权能组（Capability Set），如果包含指定的权能，那么检查通过；否则，检查失败。

### ConditionDirectoryNotEmpty

* 类型：字符串

检查配置的绝对路径目录是否非空，如果为软链接，则判断软链接指向的目录。如果目录非空，检查通过；否则，检查失败。

### ConditionFileIsExecutable

* 类型：字符串

检查配置的绝对路径文件是否可执行，如果为软链接，则判断软链接指向的文件。如果可执行，检查通过；否则，检查失败。

### ConditionFirstBoot

* 类型：可选的布尔值

检测系统是否首次启动，可配置为`false`、`true`。用于系统出厂后(或者恢复出厂设置之后)，首次开机时执行必要的初始化操作。该选项将会检测`/run/sysmaster/first-boot`文件是否存在。若文件存在，则表明系统首次启动，反之，则表明系统非首次启动。如果在内核命令行上指定了`sysmaster.condition-first-boot=`选项（采用布尔值），它将优先于`/run/sysmaster/first-boot`文件是否存在的检查结果。

### ConditionKernelCommandLine

* 类型：字符串

检查内核命令行是否配置给定的内容。该选项仅允许配置 **一个** 单词或者`=`分隔的键值对，例如：`"ro"`，`"crashkernel=512M"`。配置为单词时，检查内核命令行是否包含
该单词或作为键值对的键。配置为键值对时，将检查是否存在完全一致的键值对。内核命令行仅支持读取`/proc/cmdline`。

### ConditionPathExists

* 类型：字符串

检查文件是否存在。只有`ConditionPathExists=`后配置的绝对路径存在时，检查通过，启动流程正常；如果配置的绝对路径不存在，则检查失败，停止单元的启动流程。

### ConditionPathExistsGlob

* 类型：字符串

与`ConditionPathExists`类似，区别在于`ConditionPathExistsGlob`支持通配符。

### ConditionPathIsDirectory

* 类型：字符串

检查`ConditionPathIsDirectory=`后配置的绝对路径是否为目录，如果是，检查通过；否则检查失败。

### ConditionPathIsMountPoint

* 类型：字符串

检查`ConditionPathIsMountPoint=`后配置的绝对路径是否为挂载点目录，如果是软链接，则检查软链接指向的目录。该配置仅支持`kernel > 4.11`版本。如果是挂载点目录，检查通过；否则，检查失败。

### ConditionPathIsReadWrite

* 类型：字符串

与`ConditionPathExists`类似，检查配置的绝对路径所属的文件系统是否为可读可写。

### ConditionPathIsSymbolicLink

* 类型：字符串

检查`ConditionPathIsSymbolicLink=`后配置的绝对路径是否为软链接，如果是软链接，检查通过；否则检查失败。

### ConditionSecurity

* 类型：字符串

`ConditionSecurity`可以用于检查系统是否支持给定的安全技术。当前支持的值包括：`"selinux"`，`"apparmor"`，`"tomoyo"`，`"ima"`，`"smack"`，`"audit"`，`"uefi-secureboot"`和`"tpm2"`。 **注意：** 检查`"selinux"`需要在编译时通过 `--features "selinux"` 开启 `"selinux"` feature。

### ConditionUser

* 类型：字符串

检测sysmaster是否以给定的用户身份运行。参数可以是数字形式的"UID"、字符串形式的UNIX用户名或者特殊值`"@system"`(表示属于系统用户范围内)。如果不配置，默认跳过该检查。

## 其他配置

### RefuseManualStart/RefuseManualStop

* 类型：布尔值

`RefuseManualStart`/`RefuseManualStop`：配置单元是否拒绝通过`sctl start/stop`的形式手动启动/关闭。默认配置为`false`，即允许手动启动/关闭。该配置不影响通过依赖关系解析启动/关闭服务。
