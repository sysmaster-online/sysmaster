//! mount 是process1的挂载点监控的入口，process1参考systemd，但不完全相同。
//! process1主要提供监控功能，不主动挂载/卸载挂载点，这由其他进程实现。
//! mount不支持配置文件。
//!
//! ## 自动依赖
//! 无。
//! ### 隐含依赖
//! 无。
//! ### 默认依赖
//! 无。

mod mount_comm;
mod mount_mng;
mod mount_unit;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
