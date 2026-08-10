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
// This private coordinator has no supervisor mutation wire. It accepts only
// a kernel-registered deterministic experiment, then claims, materializes,
// contains, seals, and projects exact generic custody receipts without
// admitting evaluator evidence.
#[allow(dead_code)]
mod deterministic_evaluator;
mod observability;
// The daemon-private native-child foundation is intentionally unreachable from
// the public/supervisor wire. Its provider-free integration tests exercise the
// real kernel/process path without making a generic execution request public.
#[allow(dead_code)]
mod native_child;
#[allow(dead_code)]
mod pi_execution;
pub mod protocol;
pub mod supervision;

pub use daemon::{Daemon, DaemonConfig, DaemonError, FaultInjection, ShutdownHandle, StartupMode};
pub use observability::{MonitorInstallError, install_mandatory_monitor};
