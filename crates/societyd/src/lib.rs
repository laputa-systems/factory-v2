//! Resident, single-writer authority and native Pi process physics.
//!
//! The daemon owns the SQLite connection and accepts only the closed binary
//! local protocol in [`protocol`]. [`supervision`] is deliberately narrower:
//! it owns an inert Pi-host child/process group and returns transient typed
//! receipts for the later kernel transaction. It does not itself persist
//! content, charge budgets, admit work, or recover a successor.

// The resident content writer has no public/supervisor command yet. Keep its
// complete typed recovery seam crate-private until the control loop can carry
// an authorized operation into it; tests exercise the physical/kernel chain.
#[allow(dead_code)]
mod content;
mod daemon;
// This private driver has no supervisor mutation wire. It accepts only the
// already-admitted deterministic evaluator treatment and seals reaped bytes
// before a later kernel evidence command may observe them.
#[allow(dead_code)]
mod deterministic_evaluator;
mod observability;
// The daemon-private M5 bridge is intentionally not yet reachable from the
// public/supervisor wire. Its provider-free integration tests exercise the
// real kernel/process path while the next resident control-loop tranche wires
// in its typed scheduler call site.
#[allow(dead_code)]
mod native_child;
#[allow(dead_code)]
mod pi_execution;
pub mod protocol;
pub mod supervision;

pub use daemon::{Daemon, DaemonConfig, DaemonError, FaultInjection, ShutdownHandle, StartupMode};
pub use observability::{MonitorInstallError, install_mandatory_monitor};
