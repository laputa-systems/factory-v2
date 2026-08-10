//! Daemon-private deterministic evaluator process driver.
//!
//! There is deliberately no local protocol command, scheduler call site, Pi
//! session, or semantic evidence mutation here. A later resident bridge may
//! construct [`DeterministicEvaluatorAdmission`] only from the kernel's exact
//! native-child admission/experiment binding. This driver then owns native
//! process physics and allows physical output sealing only after direct reaping
//! and owned-group containment have completed.

use std::collections::BTreeMap;
use std::io::Read;

use society_kernel::{
    Blake3Digest, ChildStreamKind, DeterministicExperimentId, EvaluatorRevisionId, InputManifestId,
    NativeChildSpawnAdmissionId,
};
use thiserror::Error;

use crate::{
    content::{
        ContentObjectRegistration, ContentSealOperationId, ContentSealOperationIdError,
        ContentSealingAuthority, ContentSealingError,
    },
    native_child::{
        NativeChildDeadline, NativeChildEnvironment, NativeChildExecution, NativeChildReceipt,
        NativeChildSpawnOutcome, NativeChildSpawnRequest, NativeChildSupervisor,
    },
    supervision::{
        MonotonicTick, NativeWorkspace, SupervisedChildId, SupervisionError, TransientRetention,
        VerifiedArtifact,
    },
};

/// The daemon-local projection of one already durable kernel admission. The
/// identity fields are retained together so a future bridge cannot splice an
/// evaluator executable or input treatment from another experiment. The
/// fields stay private: arbitrary paths, program identities, and argv do not
/// cross a public daemon boundary.
#[derive(Clone, Debug)]
pub(crate) struct DeterministicEvaluatorAdmission {
    native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
    deterministic_experiment_id: DeterministicExperimentId,
    evaluator_revision_id: EvaluatorRevisionId,
    input_manifest_id: InputManifestId,
    child_process_id: SupervisedChildId,
    workspace: NativeWorkspace,
    evaluator_artifact: VerifiedArtifact,
    input_manifest_artifact: VerifiedArtifact,
}

/// No construction API is exposed while no resident scheduler entry point
/// carries the kernel admission. This internal constructor gives the future
/// bridge one narrow, auditable join point rather than a generic executor.
impl DeterministicEvaluatorAdmission {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_kernel_admission(
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
        deterministic_experiment_id: DeterministicExperimentId,
        evaluator_revision_id: EvaluatorRevisionId,
        input_manifest_id: InputManifestId,
        child_process_id: SupervisedChildId,
        workspace: NativeWorkspace,
        evaluator_artifact: VerifiedArtifact,
        input_manifest_artifact: VerifiedArtifact,
    ) -> Self {
        Self {
            native_child_spawn_admission_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            child_process_id,
            workspace,
            evaluator_artifact,
            input_manifest_artifact,
        }
    }
}

#[derive(Default)]
pub(crate) struct DeterministicEvaluatorDriver {
    native_children: NativeChildSupervisor,
    admitted_children: BTreeMap<SupervisedChildId, NativeChildSpawnAdmissionId>,
}

impl DeterministicEvaluatorDriver {
    /// Rechecks direct executable/evaluator digests and spawns only the exact
    /// empty-environment direct-executable treatment. The caller records the
    /// returned native facts using the admission ID before any deadline tick.
    pub(crate) fn spawn_admitted(
        &mut self,
        kernel: &society_kernel::KernelStore,
        admission: DeterministicEvaluatorAdmission,
    ) -> Result<NativeChildSpawnOutcome, SupervisionError> {
        let kernel_admission = kernel
            .deterministic_evaluator_native_child_admission(
                admission.native_child_spawn_admission_id,
            )
            .map_err(|_| SupervisionError::InvalidNativeChildRequest)?
            .ok_or(SupervisionError::InvalidNativeChildRequest)?;
        if kernel_admission.deterministic_experiment_id() != admission.deterministic_experiment_id
            || kernel_admission.evaluator_revision_id() != admission.evaluator_revision_id
            || kernel_admission.input_manifest_id() != admission.input_manifest_id
        {
            return Err(SupervisionError::InvalidNativeChildRequest);
        }
        reject_script_or_digest_drift(
            &admission.evaluator_artifact,
            Some(kernel_admission.evaluator_digest()),
        )?;
        reject_script_or_digest_drift(
            &admission.input_manifest_artifact,
            Some(kernel_admission.input_manifest_digest()),
        )?;
        let _binding = (
            admission.native_child_spawn_admission_id,
            admission.deterministic_experiment_id,
            admission.evaluator_revision_id,
            admission.input_manifest_id,
        );
        let child_process_id = admission.child_process_id.clone();
        let native_child_spawn_admission_id = admission.native_child_spawn_admission_id;
        let execution = NativeChildExecution::direct_evaluator(
            admission.evaluator_artifact,
            admission.input_manifest_artifact,
        )?;
        let outcome = self.native_children.spawn(NativeChildSpawnRequest {
            child_process_id: admission.child_process_id,
            workspace: admission.workspace,
            execution,
            environment: NativeChildEnvironment::EmptyV1,
        })?;
        // A post-spawn setup failure is still an admitted containment subject.
        // Retain the binding so its later reaped streams cannot be spliced
        // into another evaluator admission.
        self.admitted_children
            .insert(child_process_id, native_child_spawn_admission_id);
        Ok(outcome)
    }

    pub(crate) fn drive_at(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
        deadline: NativeChildDeadline,
    ) -> Result<(), SupervisionError> {
        self.native_children
            .drive_at(child_process_id, now, deadline)
    }

    /// The content writer sees only complete retained stdout after the generic
    /// core has reaped the direct child and found the owned group absent. No
    /// evaluator evidence command is emitted here: a later bridge must use
    /// the existing closed deterministic evidence path after this physical
    /// seal/register result exists.
    pub(crate) fn seal_reaped_streams(
        &self,
        authority: &ContentSealingAuthority,
        kernel: &mut society_kernel::KernelStore,
        receipt: &NativeChildReceipt,
    ) -> Result<DeterministicEvaluatorSealedStreams, DeterministicEvaluatorError> {
        let native_child_spawn_admission_id = self
            .admitted_children
            .get(&receipt.child_process_id)
            .copied()
            .ok_or(DeterministicEvaluatorError::ReceiptNotAdmitted)?;
        if receipt.stdout.retention != TransientRetention::Complete
            || receipt.stderr.retention != TransientRetention::Complete
        {
            return Err(DeterministicEvaluatorError::OutputNotComplete);
        }
        let stdout_operation = ContentSealOperationId::native_child_stream(
            native_child_spawn_admission_id,
            ChildStreamKind::Stdout,
            kernel_digest(receipt.stdout.blake3())?,
        )?;
        let stderr_operation = ContentSealOperationId::native_child_stream(
            native_child_spawn_admission_id,
            ChildStreamKind::Stderr,
            kernel_digest(receipt.stderr.blake3())?,
        )?;
        let stdout = authority
            .seal_and_register(kernel, &stdout_operation, receipt.stdout.retained_bytes())
            .map_err(DeterministicEvaluatorError::Content)?;
        let stderr = authority
            .seal_and_register(kernel, &stderr_operation, receipt.stderr.retained_bytes())
            .map_err(DeterministicEvaluatorError::Content)?;
        Ok(DeterministicEvaluatorSealedStreams { stdout, stderr })
    }
}

fn kernel_digest(digest: &society_pi::Blake3Digest) -> Result<Blake3Digest, SupervisionError> {
    Ok(Blake3Digest::from_bytes(protocol_digest_bytes(digest)?))
}

fn reject_script_or_digest_drift(
    artifact: &VerifiedArtifact,
    expected: Option<society_kernel::Blake3Digest>,
) -> Result<(), SupervisionError> {
    if let Some(expected) = expected
        && protocol_digest_bytes(artifact.expected_blake3())? != expected.as_bytes()
    {
        return Err(SupervisionError::InvalidNativeChildRequest);
    }
    let mut file = std::fs::File::open(artifact.path().as_path())?;
    let mut prefix = [0_u8; 2];
    let count = file.read(&mut prefix)?;
    if count == 2 && prefix == *b"#!" {
        return Err(SupervisionError::InvalidNativeChildRequest);
    }
    artifact.verify_current_identity()
}

fn protocol_digest_bytes(digest: &society_pi::Blake3Digest) -> Result<[u8; 32], SupervisionError> {
    let text = digest.as_str().as_bytes();
    if text.len() != 64 {
        return Err(SupervisionError::InvalidNativeChildRequest);
    }
    let mut output = [0_u8; 32];
    let (pairs, []) = text.as_chunks::<2>() else {
        return Err(SupervisionError::InvalidNativeChildRequest);
    };
    for (index, pair) in pairs.iter().enumerate() {
        let nibble = |value: u8| match value {
            b'0'..=b'9' => Ok(value - b'0'),
            b'a'..=b'f' => Ok(value - b'a' + 10),
            _ => Err(SupervisionError::InvalidNativeChildRequest),
        };
        output[index] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Ok(output)
}

/// Both native output directions are physically sealed before a future kernel
/// stream/evidence command can refer to either object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeterministicEvaluatorSealedStreams {
    pub(crate) stdout: ContentObjectRegistration,
    pub(crate) stderr: ContentObjectRegistration,
}

#[derive(Debug, Error)]
pub(crate) enum DeterministicEvaluatorError {
    #[error("reaped evaluator receipt was not produced by this admission")]
    ReceiptNotAdmitted,
    #[error("evaluator stdout was not completely captured after reaping")]
    OutputNotComplete,
    #[error("evaluator stream operation identity could not be derived")]
    OperationIdentity(#[from] ContentSealOperationIdError),
    #[error("evaluator stream digest was not canonical BLAKE3")]
    Digest(#[from] SupervisionError),
    #[error("reaped evaluator output could not be physically sealed")]
    Content(#[source] ContentSealingError),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    static NEXT_ARTIFACT: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn admitted_artifact_check_binds_raw_digest_and_rejects_shebang_programs() {
        let (direct, direct_path) = artifact(b"\x7fELFfixture");
        assert!(
            reject_script_or_digest_drift(&direct, Some(Blake3Digest::of_bytes(b"\x7fELFfixture")))
                .is_ok()
        );
        assert!(matches!(
            reject_script_or_digest_drift(
                &direct,
                Some(Blake3Digest::of_bytes(b"different fixture"))
            ),
            Err(SupervisionError::InvalidNativeChildRequest)
        ));
        fs::remove_file(direct_path).unwrap();

        let (script, script_path) = artifact(b"#!/bin/sh\nexit 0\n");
        assert!(matches!(
            reject_script_or_digest_drift(
                &script,
                Some(Blake3Digest::of_bytes(b"#!/bin/sh\nexit 0\n"))
            ),
            Err(SupervisionError::InvalidNativeChildRequest)
        ));
        fs::remove_file(script_path).unwrap();
    }

    fn artifact(bytes: &[u8]) -> (VerifiedArtifact, std::path::PathBuf) {
        let unique = NEXT_ARTIFACT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("societyd-evaluator-artifact-{unique}"));
        fs::write(&path, bytes).unwrap();
        let digest = society_pi::Blake3Digest::parse(blake3::hash(bytes).to_string()).unwrap();
        let artifact = VerifiedArtifact::inspect(&path, digest).unwrap();
        (artifact, path)
    }
}
