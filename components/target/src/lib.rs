//! #target 是process1的控制启动模式的入口，启动模式最早的概念来源Linux系统sysvint的概念，在sysvinit中，启动模式包含0-6 6个模式
//! process1 参考systemd，使用target作为启动模式的入口，是process1在启动过程中默认加载的单元，target没有实际的要执行的动作，
//! target可以理解成在系统启动过程中，需要启动的unit的逻辑分组
//! target配置文件没有自己的私有配置项，只包含Unit/Install
//! # Example:
//! ```toml
//! [Unit]
//! Description = ""
//!
//! [Install]
//! WantedBy =
//! ```
//! ## 自动依赖
//!
//! ### 隐含依赖
//! 没有隐含依赖
//!
//! ### 默认依赖
//! 如果设置了DefaultDependencies = no ，否则会默认增加如下依赖关系：
//! + 对通过XXX
//! + Conflicts = shutdown.target 与 Before = shutdown.target的依赖

mod target_comm;
mod target_mng;
mod target_unit;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
