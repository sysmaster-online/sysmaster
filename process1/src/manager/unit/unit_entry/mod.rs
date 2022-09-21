//! # 总体描述
//! unit 是process1 管理对象的一个抽象，所有的对象都可以映射为一个unit，unit在process1中分为两个大的阶段
//! 1.load阶段：从配置文件转换为具体的unit对象，load到process1内部。
//! 2.执行阶段：创建unit实例，执行unit定义的具体动作。
//! # 整体抽象
//! Unit为process1管理的基本单元抽象，systemd原来包含了9中类型，process1支持Unit扩展成多种类型，整体架构如下：
//! ![avatar][../../../../doc/img/unit_c_diagram.jpg]
//! 包含两个核心对象：UnitObj，Unit以及一个子Unit的实现。
//! UnitObj是子类的接口抽象，包含子类必须实现的接口，在rust中使用trait表示,具体定义见['UnitObj']
//! # 配置项说明
//! unit配置包含三个部分，具体描述如下
//! ```toml
//! [Unit]：所有Unit都可以配置的配置项，具体见uu_config::UeConfigUnit
//! [SelfDefSection]
//! [Install] Unit安装时（安装概念，见后续备注）的配置项
//! ```
//! # load阶段设计
//!    load阶段将unit从配置文件中加载到process1内部，包括配置unit对象创建，配置文件解析，unit对象属性填充。
//! ## unit对象创建
//!    process1参考systemd，初步规划包含9种类型的unit，每种类型的配置文件命名规则为*.XXX,XXX指具体的unit类型，如service，slice，target等。
//!    
//! 包含以下模块
//! u_entry: unit的接口抽象实体，是所有unit—的父类，子类可以实现unitObj trait对象
//! uf_interface是内部管理的实体对象，对Unit进行封装，在process1内部只看到UnitX对象，看不到Unit，对Unit进行隔离
//! uu_load 对 unitload状态的封装
//! uu_child对 unit关联的父子进程的维护，unit关联的子服务可能会启动子进程，因此这里需要维护unit关联的进程有哪些。
//! uu_cgroup cgroup相关配置
//! uu_config 是unit的配置
//!
pub use u_entry::{Unit, UnitObj, UnitRef};
pub(in crate::manager) use uf_interface::UnitX;
// pub(super) use uu_config::UnitConfigItem;

// dependency: {uu_config | uu_cgroup} -> {uu_load | uu_child} -> u_entry -> uf_interface
mod u_entry;
mod uf_interface;
mod uu_cgroup;
mod uu_child;
mod uu_condition;
mod uu_config;
mod uu_load;
