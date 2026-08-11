//! Resident-owned native execution-profile qualification.
//!
//! The qualification path is deliberately not an actor TaskAttempt or a Root
//! Authority Office session.  Its durable child owner is
//! `NativeExecutionProfileQualification`; the `TaskAttempt` session kind used
//! on the Pi wire is only the currently closed adapter envelope.  This module
//! keeps that distinction visible and keeps all spawn configuration, native
//! paths, content custody, and process receipts inside `societyd`.

use society_kernel::ExecutionProfileQualificationId;
use thiserror::Error;

use crate::{
    Daemon,
    pi_execution::{
        NativeExecutionProfileQualificationChild, NativeExecutionProfileQualificationEvidence,
        NativeExecutionProfileQualificationSpawnRegistration,
        NativeExecutionProfileQualificationStart, PiExecutionError, PiExecutionOperationId,
        UnregisteredPiChild,
    },
    supervision::{ControlWriteDeadline, ControlWriteProgress, HandshakeDeadline, MonotonicTick},
};

const OPERATION_MAX_BYTES: usize = 36;

/// Stable identity for one daemon qualification lifecycle.  It is an
/// operation label, not a process path, provider/model selector, or arbitrary
/// command namespace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeProfileQualificationRunnerOperationId(String);

impl NativeProfileQualificationRunnerOperationId {
    pub fn parse(value: impl Into<String>) -> Result<Self, NativeProfileQualificationError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > OPERATION_MAX_BYTES
            || !value.as_bytes()[0].is_ascii_alphanumeric()
            || !value
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric)
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(NativeProfileQualificationError::InvalidOperationIdentity);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Observable lifecycle state.  The runner never returns a PID, workspace,
/// transcript, content-store path, or mutable process handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeProfileQualificationState {
    NotStarted,
    Spawned,
    ContainmentRequired,
    AdapterReady,
    CreateAuthorized,
    CreateDelivered,
    SessionReady,
    ControlledExitRequested,
    Reconciled,
    Qualified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum NativeProfileQualificationError {
    #[error("qualification operation identity is not canonical")]
    InvalidOperationIdentity,
    #[error("qualification lifecycle is not in the required phase")]
    InvalidLifecycle,
    #[error("daemon recovery fencing prevents native qualification")]
    RecoveryFenced,
    #[error("native qualification requires containment or reconciliation")]
    ContainmentRequired,
    #[error("native qualification kernel transition failed")]
    Kernel,
    #[error("native qualification process supervision failed")]
    Process,
    #[error("native qualification Pi protocol failed")]
    Protocol,
    #[error("native qualification content custody failed")]
    Content,
    #[error("no active kernel qualification launch claim exists")]
    LaunchClaimMissing,
    #[error("resident pinned Pi launch profile rejected the qualification claim")]
    Profile,
}

impl From<PiExecutionError> for NativeProfileQualificationError {
    fn from(error: PiExecutionError) -> Self {
        match error {
            PiExecutionError::RecoveryFenced => Self::RecoveryFenced,
            PiExecutionError::InvalidLifecycle => Self::InvalidLifecycle,
            PiExecutionError::Kernel(_)
            | PiExecutionError::KernelServiceCapabilityMissing { .. }
            | PiExecutionError::KernelCommandRejected { .. }
            | PiExecutionError::UnexpectedKernelEvent => Self::Kernel,
            PiExecutionError::Supervision(_) => Self::Process,
            PiExecutionError::Content(_) => Self::Content,
            _ => Self::Protocol,
        }
    }
}

/// A daemon-owned qualification runner. Construction from an active durable
/// launch claim is public; the claim is projected by the kernel and the
/// resident then supplies all native host/session/workspace material.
pub struct NativeProfileQualificationRunner<'daemon> {
    daemon: &'daemon mut Daemon,
    operation: NativeProfileQualificationRunnerOperationId,
    start: Option<NativeExecutionProfileQualificationStart>,
    child: Option<NativeExecutionProfileQualificationChild>,
    unregistered_child: Option<UnregisteredPiChild>,
    evidence: Option<NativeExecutionProfileQualificationEvidence>,
    state: NativeProfileQualificationState,
}

impl<'daemon> NativeProfileQualificationRunner<'daemon> {
    pub(crate) fn new(
        daemon: &'daemon mut Daemon,
        operation: NativeProfileQualificationRunnerOperationId,
        start: NativeExecutionProfileQualificationStart,
    ) -> Self {
        Self {
            daemon,
            operation,
            start: Some(start),
            child: None,
            unregistered_child: None,
            evidence: None,
            state: NativeProfileQualificationState::NotStarted,
        }
    }

    pub fn state(&self) -> NativeProfileQualificationState {
        self.state
    }

    /// Crosses the native boundary exactly once.  The start value is consumed
    /// and never returned, even on a registration failure.
    pub fn spawn(
        &mut self,
    ) -> Result<NativeProfileQualificationState, NativeProfileQualificationError> {
        if self.state != NativeProfileQualificationState::NotStarted {
            return Err(NativeProfileQualificationError::InvalidLifecycle);
        }
        let mut start = self
            .start
            .take()
            .ok_or(NativeProfileQualificationError::InvalidLifecycle)?;
        let operation = PiExecutionOperationId::parse(self.operation.as_str().to_owned())
            .map_err(NativeProfileQualificationError::from)?;
        start.operation = operation;
        let registration = self
            .daemon
            .admit_native_profile_qualification_child(start)
            .map_err(NativeProfileQualificationError::from)?;
        match registration {
            NativeExecutionProfileQualificationSpawnRegistration::Ready(child) => {
                self.child = Some(child);
                self.state = NativeProfileQualificationState::Spawned;
                Ok(self.state)
            }
            NativeExecutionProfileQualificationSpawnRegistration::PostSpawnSetupContained {
                child,
                ..
            }
            | NativeExecutionProfileQualificationSpawnRegistration::RegisteredBoundaryContained {
                child,
                ..
            } => {
                self.child = Some(child);
                self.state = NativeProfileQualificationState::ContainmentRequired;
                Err(NativeProfileQualificationError::ContainmentRequired)
            }
            NativeExecutionProfileQualificationSpawnRegistration::RegistrationUnresolved {
                child,
                ..
            } => {
                self.unregistered_child = Some(*child);
                self.state = NativeProfileQualificationState::ContainmentRequired;
                Err(NativeProfileQualificationError::ContainmentRequired)
            }
        }
    }

    pub fn drive_containment(
        &mut self,
        now: MonotonicTick,
    ) -> Result<bool, NativeProfileQualificationError> {
        if let Some(child) = self.unregistered_child.as_mut() {
            let done = self
                .daemon
                .drive_unregistered_native_profile_qualification_containment(child, now)
                .map_err(NativeProfileQualificationError::from)?;
            if done {
                self.unregistered_child = None;
            }
            return Ok(done);
        }
        let child = self
            .child
            .as_ref()
            .ok_or(NativeProfileQualificationError::InvalidLifecycle)?;
        self.daemon
            .drive_native_profile_qualification_boundary_containment(child, now)
            .map_err(NativeProfileQualificationError::from)?;
        Ok(false)
    }

    pub fn observe_adapter_ready(
        &mut self,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<bool, NativeProfileQualificationError> {
        let child = self
            .child
            .as_mut()
            .ok_or(NativeProfileQualificationError::InvalidLifecycle)?;
        let ready = self
            .daemon
            .observe_native_profile_qualification_adapter_ready(child, now, deadline)
            .map_err(NativeProfileQualificationError::from)?;
        if ready {
            self.state = NativeProfileQualificationState::AdapterReady;
        }
        Ok(ready)
    }

    pub fn begin_create(
        &mut self,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, NativeProfileQualificationError> {
        let child = self
            .child
            .as_mut()
            .ok_or(NativeProfileQualificationError::InvalidLifecycle)?;
        let progress = self
            .daemon
            .authorize_and_begin_native_profile_qualification_create(child, now, deadline)
            .map_err(NativeProfileQualificationError::from)?;
        self.state = NativeProfileQualificationState::CreateAuthorized;
        if progress == ControlWriteProgress::Delivered {
            self.state = NativeProfileQualificationState::CreateDelivered;
        }
        Ok(progress)
    }

    pub fn drive_create(
        &mut self,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, NativeProfileQualificationError> {
        let child = self
            .child
            .as_mut()
            .ok_or(NativeProfileQualificationError::InvalidLifecycle)?;
        let progress = self
            .daemon
            .drive_native_profile_qualification_create_delivery(child, now)
            .map_err(NativeProfileQualificationError::from)?;
        if progress == ControlWriteProgress::Delivered {
            self.state = NativeProfileQualificationState::CreateDelivered;
        }
        Ok(progress)
    }

    pub fn observe_session_ready(
        &mut self,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<bool, NativeProfileQualificationError> {
        let child = self
            .child
            .as_mut()
            .ok_or(NativeProfileQualificationError::InvalidLifecycle)?;
        let ready = self
            .daemon
            .observe_native_profile_qualification_session_ready(child, now, deadline)
            .map_err(NativeProfileQualificationError::from)?;
        if ready {
            self.state = NativeProfileQualificationState::SessionReady;
        }
        Ok(ready)
    }

    /// Requests the physical controlled-exit suffix.  There is intentionally
    /// no Office Dispose or TaskAttempt terminal semantic in qualification.
    pub fn request_controlled_exit(
        &mut self,
        now: MonotonicTick,
    ) -> Result<(), NativeProfileQualificationError> {
        let child = self
            .child
            .as_mut()
            .ok_or(NativeProfileQualificationError::InvalidLifecycle)?;
        self.daemon
            .request_native_profile_qualification_exit(child, now)
            .map_err(NativeProfileQualificationError::from)?;
        self.state = NativeProfileQualificationState::ControlledExitRequested;
        Ok(())
    }

    pub fn drive_controlled_exit(
        &mut self,
        now: MonotonicTick,
    ) -> Result<(), NativeProfileQualificationError> {
        let child = self
            .child
            .as_ref()
            .ok_or(NativeProfileQualificationError::InvalidLifecycle)?;
        self.daemon
            .drive_native_profile_qualification_exit(child, now)
            .map_err(NativeProfileQualificationError::from)
    }

    pub fn reconcile(
        &mut self,
        now: MonotonicTick,
    ) -> Result<bool, NativeProfileQualificationError> {
        let child = self
            .child
            .as_mut()
            .ok_or(NativeProfileQualificationError::InvalidLifecycle)?;
        let Some(evidence) = self
            .daemon
            .reconcile_native_profile_qualification_child(child, now)
            .map_err(NativeProfileQualificationError::from)?
        else {
            return Ok(false);
        };
        self.evidence = Some(evidence);
        self.state = NativeProfileQualificationState::Reconciled;
        Ok(true)
    }

    pub fn qualify(
        &mut self,
    ) -> Result<ExecutionProfileQualificationId, NativeProfileQualificationError> {
        if self.state != NativeProfileQualificationState::Reconciled {
            return Err(NativeProfileQualificationError::InvalidLifecycle);
        }
        let child = self
            .child
            .as_ref()
            .ok_or(NativeProfileQualificationError::InvalidLifecycle)?;
        let evidence = self
            .evidence
            .as_ref()
            .ok_or(NativeProfileQualificationError::InvalidLifecycle)?;
        let qualification_id = self
            .daemon
            .qualify_native_profile(child, evidence)
            .map_err(NativeProfileQualificationError::from)?;
        self.state = NativeProfileQualificationState::Qualified;
        Ok(qualification_id)
    }
}
