//! Trusted, replayable domain and PostgreSQL storage for a Society.
//!
//! This crate intentionally has no JSON serialization boundary.  Commands are
//! closed Rust values, and each accepted command/event has one named PostgreSQL
//! body table.  JSON is reserved for the separately-owned Pi SDK-host adapter.

mod domain;
mod postgres;
#[doc(hidden)]
pub mod postgres_db;
mod store;
mod study;

pub use domain::*;
pub use postgres::{
    KernelDatabaseUrl, POSTGRES_SCHEMA_REVISION, PostgresAdvisoryLockGuard,
    PostgresAdvisoryLockLease, PostgresCatalogSnapshot, PostgresKernelStore, PostgresStoreError,
};
pub use store::{
    ContentIdentityState, DeterministicEvaluatorNativeChildAdmission,
    DeterministicEvaluatorScheduleClaim, DeterministicEvaluatorScheduleClaimRequest,
    InstallFoundingMissionPreflight, KernelStore, NativeExecutionProfileQualificationLaunchClaim,
    StoreError, StudyActorTaskAttemptLaunchClaim, StudyActorWorkAllocation,
};
pub use study::*;
