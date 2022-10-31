//! Mount is the entry for the mount point monitoring of sysmaster. sysmaster refers to systemd, but it is not the same.
//! sysmaster mainly provides the monitoring function and does not actively mount/unmount the mount point, which is implemented by other processes.
//! Mount does not support configuration files.
//!
//! ## Automatic dependency
//! NA
//! ### Implicit dependency
//! NA
//! ### Default Dependency
//! NA

// dependency: mount_base -> mount_rentry -> mount_comm -> {mount_mng -> mount_unit} -> mount_manager
mod mount_base;
mod mount_comm;
mod mount_manager;
mod mount_mng;
mod mount_rentry;
mod mount_unit;
