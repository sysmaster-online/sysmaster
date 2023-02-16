//! # Mainly responsible for the creation of the system directory and the mounting of the file system during the system startup process
//!
//! # Create cgroupv2 subsystem
//!
//! If sysmaster.unified_cgroup_hierarchy=y or cgroup_no_v1=all is set in the /proc/cmdline file, it means that the system uses cgroupv2.
//! The file system type of the /sys/fs/cgroup directory mounted by the system is cgroup2.
//!
//! # Create cgorupv1 subsystem
//!
//! Otherwise, the file system type of the mounted /sys/fs/cgroup directory is tmpfs.
//!
//! If sysmaster.unified_v1_controller=y is set in the /proc/cmdline file, mount /sys/fs/cgroup/unified as cgroup2.
//!
//! Create a directory sysmaster in the /sys/fs/cgroup/ directory that does not belong to any cgroup subsystem, and mount it as a file system type cgroup.
//!
//! Read the /proc/cgroups directory, query the subsystems supported by the current system, and mount the corresponding subsystem type in the /sys/fs/cgroup directory.

pub mod setup;
