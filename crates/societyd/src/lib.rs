//! Resident, single-writer authority and native Pi process physics.
//!
//! The daemon owns the PostgreSQL connection and accepts only the closed binary
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
mod launch_profile;
#[allow(dead_code)]
mod native_child;
#[allow(dead_code)]
mod pi_execution;
// Qualification is opened only from a durable kernel claim and a
// supervisor-installed pinned host/session profile. The raw spawn seam remains
// crate-private and is absent from the local protocol.
#[allow(dead_code)]
mod qualification_runner;
// Root-owned M3 provisioning is a separate composition boundary. It can
// construct only the closed project/ticket/actor/work/allocation sequence
// required by a sealed study, never arbitrary kernel commands.
mod root_study_provisioner;
mod study_admission;
// The plan-lifetime bridge accepts only generic sealed-run selectors. It
// consumes a durable root-owned M3 allocation projection; allocation authority
// is never part of the local protocol or application composition surface.
mod study_plan;
// The live-study TaskAttempt coordinator is deliberately separate from the
// Office-only Pi bridge. Its public surface is an in-process composition API;
// the local socket remains query-only and cannot acquire this authority.
pub mod protocol;
pub mod supervision;
#[allow(dead_code)]
pub mod task_attempt_scheduler;

pub use daemon::{Daemon, DaemonConfig, DaemonError, FaultInjection, ShutdownHandle, StartupMode};
pub use launch_profile::{PinnedPiLaunchProfile, PinnedPiProfileError, PinnedPiSessionProfile};
pub use observability::{MonitorInstallError, install_mandatory_monitor};
pub use qualification_runner::{
    NativeProfileQualificationError, NativeProfileQualificationRunner,
    NativeProfileQualificationRunnerOperationId, NativeProfileQualificationState,
};
pub use root_study_provisioner::{
    RootStudyM3Project, RootStudyM3ProvisioningAuthority, RootStudyM3ProvisioningError,
    RootStudyM3ProvisioningOperationId, RootStudyM3ProvisioningPlan,
    RootStudyM3ProvisioningReceipt, RootStudyM3Seat,
};
pub use study_admission::{
    SealedStudyContent, StudyAdmissionAuthority, StudyAdmissionContentSlot, StudyAdmissionError,
    StudyAdmissionOperationId,
};
pub use study_plan::{StudyPlanLifetimeError, StudyPlanLifetimeKey};
pub use supervision::{
    ControlWriteDeadline, ControlWriteProgress, HandshakeDeadline, MonotonicTick,
};
pub use task_attempt_scheduler::{
    TaskAttemptDisposeEvent, TaskAttemptDisposeRequest, TaskAttemptPromptEvent,
    TaskAttemptPromptRequest, TaskAttemptRunner, TaskAttemptRunnerError,
    TaskAttemptRunnerOpenError, TaskAttemptRunnerOperationId, TaskAttemptScheduleState,
};
