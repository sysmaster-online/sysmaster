# sysMaster （[http://sysmaster.online](http://sysmaster.online)）

## 挑战

1 号进程处于系统关键位置，负责系统初始化和运行时服务管理，当前面临如下挑战：
- **①可靠性差：** 位置关键，自身故障时必须重启操作系统才能恢复
- **②复杂性高：** systemd 成为 1 号进程事实上的标准。它引入了许多新的概念和工具，依赖繁杂，难以针对实际使用场景进行裁剪
- **③兼容性弱：** 对云、边、嵌入式场景的支持差，不满足全场景的诉求

**行业诉求**
- **服务器、嵌入式场景对可靠性有极强诉求：** 内存安全问题降至0：消减极难定位的内存问题，减少宕机影响
- **系统不宕机，业务无影响：** 借助永不宕机特性追求极致可靠
- **高效智能运维成云场景痛点：** 对HOST的运维缺乏高效手段：容器型OS对1号进程提出极简目标
- **国产化替代要求核心软件自主可控：** 聚焦关键开源软件：敏感/竞争力根组件，实现根技术完全自主可控

## 愿景
1. 短期竞争力：极致的可靠性和系统启动速度
- **永不宕机：** 通过状态外置、savepoint等技术实现故障秒级自愈，保障1号进程持续在线，业务不受影响
- **快速启动：** 解耦去重、极简镜像，支撑服务器重启时间从3-5min优化到1min（友商5min）
2. 长期竞争力：全场景应用、云平台运维效率提升
- **全场景应用：** 支持云边端等场景，统一init，支持裸机、虚机、容器
- **容器型OS：** kernel+sysMaster=容器OS底座，通过对接k8s、openstack等，与KubeOS一起支撑云场景下运维效率X倍提升

整体策略：聚焦功能最小系统，采用核心自研与非核心社区方案替代并行的策略，独立组件抽屉式替换的策略，实现systemd自研替代
1. 分场景：结合不同的使用场景，将systemd功能拆分多层，按照场景由易到难的方式，逐步完成功能开发验证及替代。
2. 多方案：无社区替代方案的核心功能和组件，采用自研重写的替代，其他的采用无供应风险的社区软件替代。
3. 有节奏：一年一个核心竞争力，至24年完成3个核心竞争力的发掘，按照开发-落地-替代三条线开展工作。

## sysMaster：全新1号进程实现方案，秒级故障自愈，保障系统全天在线
sysMaster 旨在改进传统的init守护进程，1+1+N架构。
- init：新的1号进程，功能极简，代码千行，极致可靠
- core：承担systemd原有核心功能，引入可靠性框架、插件机制，使其具备崩溃快速自愈、热升级、灵活组装能力
- exts：使原本耦合的各组件功能独立，支持抽屉式替代systemd的对应组件，支持有节奏的分场景替换

**极力构筑三大竞争力：**
1. 极致可靠 – **永不宕机**
- 极简架构：1+1+N，简化init；非核心功能组件化提供
- 极致可靠：故障感知+秒级恢复， 根进程持续在线
- 内存安全：内存问题降至0，故障后自愈
2. 极度轻量 – **快速启动**
- 更少的资源：内存占用降低10%
- 更快的速度：启动速度提升15%
3. 极优体验 – **极简镜像**
- 易于运维：热升级、按需裁剪，方便的部署/运维
- 兼容生态：提供systemd生态兼容、转换工具
- 插件机制：支持灵活扩展多种服务类型

## 构建

首先，下载仓库代码，并执行命令来预装项目依赖，构建开发环境， 项目主要基于rust 1.57构建，使用``pre-commit`做git commit检查。
```
sh ./build.sh
```
其次，可以通过提供的脚本来构建程序。也可以参考`.pre-commit-config.yaml`中的动作构建。
```
sh ci/01-pre-commit.sh

# 格式检查
cargo clippy -vvv --all-targets --features "default" --all -- -Dwarnings

# 构建
cargo build --all --features "default" -v

# 测试
RUST_BACKTRACE=full cargo test --all-targets --all -v -- --nocapture --show-output --test-threads=1
```
## 使用

在各场景下的使用，可以参考`tools`目录下。
```
ls tools
```
也可访问，[sysmaster.online 官网案例](http://sysmaster.online)

## 代码目录结构说明

源码仓库以workspaces方式管理，每一个目录是一个package，每个package包含一个crate（lib或bin形式），
公共lib crate的目录带lib前缀，使用cargo new --lib libtests创建,
daemon类型的bin crate的目录以d结尾。

```text
/ (init)
|...libs (对外接口)
|     |...libtests (test lib crate)
|     |...cgroup (cgroup lib crate)
|     |...cmdproto(cmd proto lib crate)
|...exts (sysmaster-extends组件)
|     |...devmaster (daemon)
|     |...random-seed (bin)
|...core (sysmaster-core核心组件)
|     |...sysmaster (bin)
|     |...libcore (internal lib)
|     |...sctl (sysmaster cli)
|     |...coms (插件)
|          |...service (unit type crate)
|          |...socket  (unit type crate)
|          |...target  (unit type crate)
|...tools
|     |...musl_build
|     |...run_with_sd
|...docs (sysmaster.online)
|...build.sh (准备环境)
```

如：

```text
  - lib crate: libs/event, libs/basic
  - bin crate: extends/init, sysmaster
  - daemon crate: extends/udevd, extends/logind
```
