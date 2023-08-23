# README

devmaster测试场景说明手册

版本：v1.0

编辑日期：20230822

## 配置文件

### 说明

devmaster启动需准备配置文件，默认配置可以参考exts/devmaster/config/config.toml

### 配置文件的系统路径：

```
/etc/devmaster/config.toml
```

### 配置项：

```toml
# Default configuration of devmaster daemon.

# Support 'info' and 'debug'.
log_level = "info"

# Support multiple rules directories.
rules_d = ["/etc/devmaster/rules.d", "/lib/devmaster/rules.d"]

# Support multiple network interface configuration directories.
network_d = ["/etc/devmaster/network.d"]

# The default value is 3.
max_workers = 3
```

配置文件采用`toml`格式。其中`rules_d`为规则加载目录的配置项，network_d为网卡配置加载目录的配置项，两者需要显示设置。`log_level`为`devmaster`和`devctl`的日志级别配置项，支持`info`和`debug`，默认为`info`。`max_workers`为`devmaster`服务端的最大`worker`线程数，默认为3。

### 网卡配置

```toml
[Match]
OriginalName = "*"

[Link]
NamePolicy = ["database", "onboard", "slot", "path"]
```

网卡配置文件采用`toml`格式，以`.link`作为文件后缀。其中分为匹配`Match`节和配置`Link`节。网卡设备事件上报后，会经过匹配节过滤，满足匹配的网卡设备再通过配置节进行设置。匹配节目前支持`OriginalName`配置项，该配置项用于匹配网卡的原始名字，支持正则匹配或者`shell`模式匹配。配置节目前支持`NamePolicy`配置项，用于配制网卡命名策略的优先级排序，当前支持的网卡命名策略包括`database`、`onboard`、`slot`和`path`，`database`采用硬件数据库`hwdb`中的记录进行命名，`onboard`采用网卡的板载号进行命名，`slot`采用网卡的插槽号命名，`path`采用网卡的路径信息命名。网卡命名策略的优先级从左到右依次降低，如果根据高优先级策略可以获取可用的网卡名，则采用该网卡名对网卡进行重命名。**网卡配置需结合`net_id`、`net_setup_link`和规则配套使用，目前建议使用默认配置。**

### 规则文件

```
TAG+="devmaster"
```

`devmaster`的规则语法兼容`udev`，以`.rules`作为文件后缀。`devmaster`启动后，按规则加载目录的配置顺序，从各个目录中按字典序依次加载该目录下的所有配置文件。**目前`devmaster`不支持高优先级目录下的规则文件覆盖低优先级目录下的同名规则文件，和udev`的行为不同，不同目录下的同名规则文件会视为不同的规则文件导入到内存中。**每个规则文件包含多个规则行，一般情况下，单条规则行中包含匹配规则和赋值规则，匹配规则会检查事件是否满足匹配条件，如果不满足则跳过该行中后续的规则执行。赋值规则拥有创建软链接、添加标签等功能。另外，如果某条规则行中不包含赋值规则时，则规则执行过程会跳过该行。

## 测试场景说明

### 说明

- 根据`devmaster`的功能范畴划分了24类特性，每种特性包含若干使用场景，总计61种子场景。目前`devmaster`尚未编写集成测试用例，针对每种子场景设计了手工测试方法。
- 部分子场景的功能验证重叠，如果某个该子场景的备注中说明 **同子场景a.b** ，表示该子场景的功能验证参考特性`a`的子场景`b`。
- 所有测试场景仅在`openEuler 22.03 x86_64`上验证，理论上支持其他`OS`发行版和`aarch64`架构。

### 约束

- `devmaster`的功能测试依赖特定的运行环境，某些测试场景需要满足环境中存在特定硬件设备，约束条件在各个子场景中说明。

- 所有测试场景需要满足串行执行。

- 环境中无法同时运行多个`devmaster`。

- `devmaster`运行前，需要先关闭后台的`udevd`进程。

   如果是采用`systemd`启动的环境，需执行以下命令：

   ````shell
   # systemctl stop systemd-udevd.service systemd-udevd-control.socket systemd-udevd-kernel.socket
   ````

   如果是采用`sysmaster`的虚拟机启动方案的环境，需执行以下命令：

   ```shell
   # sctl stop udevd.service udevd-control.socket udevd-kernel.socket
   ```

- 每次重新配置规则后，大部分子场景下需清零数据残留`rm -rf /run/devmaster/*`，并重启`devmaster`。少部分特殊场景已在备注中说明。
