//! utils of libdevmaster
//!
use snafu::prelude::*;

/// devmaster error
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum Error {
    /// Error in worker manager
    #[snafu(display("Worker Manager: {}", msg))]
    WorkerManagerError {
        /// message
        msg: &'static str,
    },

    /// Error in job queue
    #[snafu(display("Job Queue: {}", msg))]
    JobQueueError {
        /// message
        msg: &'static str,
    },

    /// Error in control manager
    #[snafu(display("Control Manager: {}", msg))]
    ControlManagerError {
        /// message
        msg: &'static str,
    },
}
