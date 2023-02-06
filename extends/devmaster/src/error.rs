//! utils of libdevmaster
//!

/// Error kinds of devmaster
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error kind for worker manager
    #[error("Worker Manager: {}", msg)]
    WorkerManagerError {
        ///
        msg: &'static str,
    },

    /// Error kind for job queue
    #[error("Job Queue: {}", msg)]
    JobQueueError {
        ///
        msg: &'static str,
    },

    /// Error kind for control manager
    #[error("Control Manager: {}", msg)]
    ControlManagerError {
        ///
        msg: &'static str,
    },
}
