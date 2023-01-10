//! the special name of the system unit

/// default startup target
pub const DEFAULT_TARGET: &str = "default.target";
/// the shutdown target
pub const SHUTDOWN_TARGET: &str = "shutdown.target";
/// the socketc target
pub const SOCKETS_TARGET: &str = "sockets.target";

/// early boot targets
pub const SYSINIT_TARGET: &str = "sysinit.target";
/// the basic start target
pub const BASIC_TARGET: &str = "basic.target";

/// Special user boot targets */
pub const MULTI_USER_TARGET: &str = "multi-user.target";

/// the init scope
pub const INIT_SCOPE: &str = "init.scope";
/// sysmaster service slice
pub const SYSMASTER_SLICE: &str = "system.slice";

/// the unit store sysmaster itself
pub const CGROUP_SYSMASTER: &str = "sysmaster";
