//! # 主要负责系统启动过程中系统目录的创建和文件系统的挂载
//!
//! # 创建cgroupv2子系统
//!
//! 若/proc/cmdline文件中设置sysmaster.unified_cgroup_hierarchy=y或cgroup_no_v1=all, 则表示系统使用cgroupv2.
//! 系统挂载/sys/fs/cgroup目录的文件系统类型为cgroup2.
//!
//! # 创建cgorupv1子系统
//!
//! 否则挂载/sys/fs/cgroup目录的文件系统类型为tmpfs.
//!
//! 若/proc/cmdline文件中设置sysmaster.unified_v1_controller=y. 则挂载/sys/fs/cgroup/unified为cgroup2.
//!
//! 在/sys/fs/cgroup/目录下创建不属于任何cgroup子系统的目录sysmaster, 并挂载为文件系统类型为cgroup.
//!
//! 读取/proc/cgroups目录，查询当前系统支持的子系统，并在/sys/fs/cgroup目录下挂载对应的子系统类型。

pub mod mount_setup;
