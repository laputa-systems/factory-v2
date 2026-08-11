//! The approved CL-001 application/daemon composition seam.
//!
//! The application owns its sealed world plan; `societyd` owns PostgreSQL,
//! immutable-content custody, native children, and every mutation.  The
//! public [`societyd::StudyAdmissionAuthority`] is the narrow composition
//! route: it can seal this application's opaque plan and accept closed generic
//! study transitions, but it grants no database handle, content writer, native
//! executable choice, or TaskAttempt driver.  A later canonical runner must
//! still own the admitted actor-task lifecycle and paired barriers.

use std::path::{Path, PathBuf};

use society_kernel::{Blake3Digest, StudyRunPairCount};
use societyd::{
    Daemon, DaemonConfig, SealedStudyContent, StudyAdmissionContentSlot,
    StudyAdmissionError, StudyAdmissionOperationId,
};

use crate::LiveRunPlan;

/// Revision of the narrow application/daemon composition contract.
pub const DAEMON_COMPOSITION_REVISION: &str = "cl-001-daemon-composition-v1";

/// Application-owned location input for one resident daemon.
///
/// The runtime root is an operator/supervisor composition input.  It is not an
/// actor workspace and it contains no CL-001 private view, plan, seed, or
/// provider credential.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonComposition {
    runtime_root: PathBuf,
}

/// Application-owned admission material before it enters daemon custody.
///
/// The exact bytes remain inside this value until [`Self::seal_into`] uses the
/// resident content authority.  This is intentionally not a content object
/// constructor: application code cannot name an object ID without the daemon
/// recording the physical seal and its kernel receipts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedLiveRunAdmission {
    operation: StudyAdmissionOperationId,
    plan_bytes: Vec<u8>,
    plan_digest: Blake3Digest,
    pair_count: StudyRunPairCount,
}

/// The daemon-custody identity which a later coordinator must use in its
/// `StudyCommand::AdmitStudyRun` transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SealedLiveRunAdmission {
    plan_content: SealedStudyContent,
    plan_digest: Blake3Digest,
    pair_count: StudyRunPairCount,
}

#[derive(Debug)]
pub enum DaemonCompositionError {
    InvalidPairCount,
    StudyAdmission(StudyAdmissionError),
    AdmissionBytesDigestMismatch,
    PlanDigestMismatch,
}

impl std::fmt::Display for DaemonCompositionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPairCount => {
                formatter.write_str("the CL-001 plan has an invalid generic pair count")
            }
            Self::StudyAdmission(error) => write!(formatter, "daemon study admission failed: {error}"),
            Self::AdmissionBytesDigestMismatch => formatter.write_str(
                "the CL-001 admission bytes do not match the plan's sealed digest",
            ),
            Self::PlanDigestMismatch => {
                formatter.write_str("the resident seal did not preserve the CL-001 plan digest")
            }
        }
    }
}

impl std::error::Error for DaemonCompositionError {}

impl From<StudyAdmissionError> for DaemonCompositionError {
    fn from(value: StudyAdmissionError) -> Self {
        Self::StudyAdmission(value)
    }
}

impl PreparedLiveRunAdmission {
    /// Bind one exact pre-registered CL-001 plan to a unique daemon operation
    /// identity.  The operation is a custody/retry namespace, not a treatment
    /// label and not a source of randomization.
    pub fn new(
        operation: StudyAdmissionOperationId,
        plan: &LiveRunPlan,
    ) -> Result<Self, DaemonCompositionError> {
        let pair_count = u16::try_from(plan.pairs().len())
            .ok()
            .and_then(StudyRunPairCount::new)
            .ok_or(DaemonCompositionError::InvalidPairCount)?;
        let plan_bytes = plan.admission_bytes();
        let plan_digest = plan.sealed_digest();
        if Blake3Digest::of_bytes(&plan_bytes) != plan_digest {
            return Err(DaemonCompositionError::AdmissionBytesDigestMismatch);
        }
        Ok(Self {
            operation,
            plan_bytes,
            plan_digest,
            pair_count,
        })
    }

    pub fn operation(&self) -> &StudyAdmissionOperationId {
        &self.operation
    }

    pub const fn plan_digest(&self) -> Blake3Digest {
        self.plan_digest
    }

    pub const fn pair_count(&self) -> StudyRunPairCount {
        self.pair_count
    }

    /// Place the complete canonical plan under resident immutable-content
    /// custody.  This is the first live mutation; the subsequent generic run
    /// admission remains a separate typed transition and must be driven by the
    /// canonical scheduler.
    pub fn seal_into(
        &self,
        daemon: &mut Daemon,
    ) -> Result<SealedLiveRunAdmission, DaemonCompositionError> {
        let mut admission = daemon.open_study_admission(self.operation.clone())?;
        let plan_content = admission.seal_content(StudyAdmissionContentSlot::parse("plan")?, &self.plan_bytes)?;
        if plan_content.digest() != self.plan_digest {
            return Err(DaemonCompositionError::PlanDigestMismatch);
        }
        Ok(SealedLiveRunAdmission {
            plan_content,
            plan_digest: self.plan_digest,
            pair_count: self.pair_count,
        })
    }
}

impl SealedLiveRunAdmission {
    pub const fn plan_content(&self) -> SealedStudyContent {
        self.plan_content
    }

    pub const fn plan_digest(&self) -> Blake3Digest {
        self.plan_digest
    }

    pub const fn pair_count(&self) -> StudyRunPairCount {
        self.pair_count
    }
}

impl DaemonComposition {
    /// Bind CL-001's future canonical runner to this one daemon runtime root.
    pub fn new(runtime_root: impl AsRef<Path>) -> Self {
        Self {
            runtime_root: runtime_root.as_ref().to_path_buf(),
        }
    }

    /// The only resident-authority value the application may construct.
    ///
    /// The caller may give this configuration to the trusted process
    /// supervisor.  It cannot inspect or mutate the daemon's store, content
    /// custody, or native-child bridge.
    pub fn daemon_config(&self) -> DaemonConfig {
        DaemonConfig::new(&self.runtime_root)
    }

    /// The operator-selected root passed to [`Self::daemon_config`].
    pub fn runtime_root(&self) -> &Path {
        &self.runtime_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActorPolicyIdentity, LiveRunDescriptor, PairSeed, PrecisionTarget};

    fn prepared_admission() -> PreparedLiveRunAdmission {
        let descriptor = LiveRunDescriptor::canonical(
            ActorPolicyIdentity::new(
                Blake3Digest::of_bytes(b"policy"),
                Blake3Digest::of_bytes(b"runtime"),
                Blake3Digest::of_bytes(b"sampling"),
            )
            .unwrap(),
        )
        .unwrap();
        let plan = LiveRunPlan::new(
            descriptor,
            vec![
                PairSeed::new("pair-01", Blake3Digest::of_bytes(b"seed-01")).unwrap(),
                PairSeed::new("pair-02", Blake3Digest::of_bytes(b"seed-02")).unwrap(),
            ],
            [PrecisionTarget::new(10).unwrap(); crate::Cl001Metric::ALL.len()],
        )
        .unwrap();
        PreparedLiveRunAdmission::new(
            StudyAdmissionOperationId::parse("cl001-pilot-01").unwrap(),
            &plan,
        )
        .unwrap()
    }

    #[test]
    fn composition_passes_only_the_runtime_root_to_the_daemon() {
        let composition = DaemonComposition::new("/private/var/society/cl001");
        assert_eq!(
            composition.runtime_root(),
            Path::new("/private/var/society/cl001")
        );
        assert_eq!(
            composition.daemon_config().runtime_root(),
            Path::new("/private/var/society/cl001")
        );
    }

    #[test]
    fn preparation_binds_the_complete_plan_to_one_daemon_operation() {
        let prepared = prepared_admission();
        assert_eq!(prepared.operation().as_str(), "cl001-pilot-01");
        assert_eq!(prepared.pair_count().value(), 2);
        assert_ne!(prepared.plan_digest(), Blake3Digest::from_bytes([0; 32]));
    }
}
