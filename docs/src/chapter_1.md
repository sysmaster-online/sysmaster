# 整体描述

## 名词解释

| 术语名称| 说明  |
|   --   | --    |
|  Unit  | SysMaster管理的服务实例|
|  SubUnit | 具体被管理的对象类型，如Service，Socket等等 |
|  Job | 每个Unit执行时产生的对象，具备事务性，是Unit状态迁移的最小实例 |

## 设计目标

sysmaster是标现有systemd的提供的一号进程对系统进行管理的功能，当前主要设计目标为：

1. 兼容性：兼容systemd并为容器，虚拟化，服务器场景提供1号进程以及系统服务管理能力。
2. 按需加载，极速启动：现有systemd包含50+服务，在实际使用过程中，有些服务实际并没有用处，但是带来了架构的复杂性，代码结构臃肿，无法修改，启动过程冗余，启动速度有进一步提升的空间
3. 极致可靠性：通过rust语言减少内存类bug，并支持快速恢复能力，1号进程故障，系统不重启。

## 整体架构

Sysmaster核心架构包含UnitManager，DataStore，JobEngine，EventEngine，ProtoServer，Cli命令行。以及基于核心架构扩展出来的SubUnit，以及相关的服务，整体架构如下图：

![avatar](../res/architecture.jpg)

sysmaster-init：1号进程，保持简单，只盲等待，并定期监听sysmaster-core的心跳，巡检core的状态。
unitManager: 管理Unit，从配置文件到Unit对象的加载，以及subUnit类的加载
dataStore：保存系统中所有加载的unit对象，实时更新状态
eventEngine：事件驱动引擎，接收外部事件，驱动unit状态机
jobEngine：工作任务引擎，unit状态的变化，通过job驱动，job支持事务的概念，支持会滚机制，保障unit状态迁移的原子性。
potoServer：监听接收 pctrl的命令，执行对应的unit动作。
