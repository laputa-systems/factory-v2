//! Resident-owned projection from a sealed study lifetime to generic M3 work.
//!
//! A sealed application plan names an ordered population, but it does not
//! name an actor attempt, work item, budget reservation, workspace, or native
//! child.  This module is the narrow resident hand-off which joins those
//! planes.  The application-facing key is deliberately closed and contains
//! only generic study identities; the M3 identities live in the durable,
//! root-owned allocation projection and never cross the daemon boundary.
//!
//! Allocation is explicit. The resident never selects an arbitrary claimed
//! work item by role order or by whatever row happens to be available.

use society_kernel::{
    Blake3Digest, ContentObjectId, StudyPopulationPhase, StudyRoleOrdinal, StudyRunId,
    StudyRunPairCount, StudyRunPairOrdinal, StudyTreatment,
};
use thiserror::Error;

/// Generic key for one actor lifetime in a sealed finite study.
///
/// The key is intentionally independent of an application role enum.  An
/// application may derive it from its sealed topology, while the resident
/// uses it only to look up an already trusted generic allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StudyPlanLifetimeKey {
    study_run_id: StudyRunId,
    plan_content_object_id: ContentObjectId,
    plan_digest: Blake3Digest,
    pair_count: StudyRunPairCount,
    pair_ordinal: StudyRunPairOrdinal,
    treatment: StudyTreatment,
    phase: StudyPopulationPhase,
    role: StudyRoleOrdinal,
}

impl StudyPlanLifetimeKey {
    /// Constructs the only application-visible launch selector.  The sealed
    /// plan identity is retained alongside the ordinal so a plan substitution
    /// cannot reuse a resident allocation.
    #[cfg_attr(not(test), allow(dead_code))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        study_run_id: StudyRunId,
        plan_content_object_id: ContentObjectId,
        plan_digest: Blake3Digest,
        pair_count: StudyRunPairCount,
        pair_ordinal: StudyRunPairOrdinal,
        treatment: StudyTreatment,
        phase: StudyPopulationPhase,
        role: StudyRoleOrdinal,
    ) -> Self {
        Self {
            study_run_id,
            plan_content_object_id,
            plan_digest,
            pair_count,
            pair_ordinal,
            treatment,
            phase,
            role,
        }
    }

    pub const fn study_run_id(self) -> StudyRunId {
        self.study_run_id
    }

    pub const fn plan_content_object_id(self) -> ContentObjectId {
        self.plan_content_object_id
    }

    pub const fn plan_digest(self) -> Blake3Digest {
        self.plan_digest
    }

    pub const fn pair_count(self) -> StudyRunPairCount {
        self.pair_count
    }

    pub const fn pair_ordinal(self) -> StudyRunPairOrdinal {
        self.pair_ordinal
    }

    pub const fn treatment(self) -> StudyTreatment {
        self.treatment
    }

    pub const fn phase(self) -> StudyPopulationPhase {
        self.phase
    }

    pub const fn role(self) -> StudyRoleOrdinal {
        self.role
    }

    /// Stable command identity for the durable launch claim associated with
    /// this selector. The command ledger is the restart-safe projection once
    /// the resident has accepted the private M3 registration.
    pub(crate) fn launch_command_id(self) -> society_kernel::CommandId {
        society_kernel::CommandId::parse(format!(
            "study-plan-lifetime-launch/{}/{}/{}/{}/{}",
            self.study_run_id.value(),
            self.pair_ordinal.value(),
            self.treatment as i64,
            self.phase as i64,
            self.role.value(),
        ))
        .expect("study plan lifetime command identity is bounded by typed IDs")
    }
}

/// Errors at the explicit generic plan-lifetime projection boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum StudyPlanLifetimeError {
    #[error("no root-owned durable M3 allocation exists for this sealed lifetime")]
    AllocationMissing,
    #[error("the selector does not match the daemon's sealed study-run projection")]
    PlanMismatch,
    #[error("the selected study run is not running")]
    RunNotRunning,
    #[error("the selected pair ordinal is not registered in the study run")]
    PairNotRegistered,
    #[error("the private M3 allocation does not belong to the sealed study lifetime")]
    ObligationMismatch,
    #[error("the daemon could not materialize its canonical workspace for this lifetime")]
    WorkspaceMismatch,
    #[error("the daemon is recovery-fenced")]
    RecoveryFenced,
    #[error("the kernel rejected the plan-lifetime launch claim")]
    Kernel,
    #[error("the durable launch claim was not projected after acceptance")]
    LaunchClaimMissing,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> StudyPlanLifetimeKey {
        StudyPlanLifetimeKey::new(
            StudyRunId::new(1).expect("positive fixed run identity"),
            ContentObjectId::new(2).expect("positive fixed content identity"),
            Blake3Digest::of_bytes(b"plan"),
            StudyRunPairCount::new(2).expect("bounded fixed pair count"),
            StudyRunPairOrdinal::new(1).expect("bounded fixed pair ordinal"),
            StudyTreatment::Retained,
            StudyPopulationPhase::Source,
            StudyRoleOrdinal::new(1).expect("bounded fixed role ordinal"),
        )
    }

    #[test]
    fn lifetime_selector_has_one_retry_stable_resident_command_identity() {
        let first = key().launch_command_id();
        let second = key().launch_command_id();
        assert_eq!(first, second);
        assert_eq!(first.as_str(), "study-plan-lifetime-launch/1/1/1/1/1");
    }
}
