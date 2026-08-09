//! Resident, single-writer authority for the current bounded kernel tranche.
//!
//! The daemon owns the SQLite connection and accepts only the closed binary
//! local protocol in [`protocol`]. It deliberately does not yet own content
//! objects, an outbox, actor supervision, or a recovery transition: those need
//! kernel-owned types which do not exist in this narrow Milestone-2 tranche.

mod daemon;
mod observability;
pub mod protocol;

pub use daemon::{Daemon, DaemonConfig, DaemonError, FaultInjection, ShutdownHandle, StartupMode};
pub use observability::{MonitorInstallError, install_mandatory_monitor};
