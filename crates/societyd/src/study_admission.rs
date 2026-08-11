//! Public in-process admission boundary for an application-owned live study.
//!
//! An experimental application may compose the resident daemon, but it must
//! not acquire its PostgreSQL connection, content writer, child paths, or Pi
//! process handles.  This narrow boundary is the one exception: it lets the
//! application submit the already-closed generic [`StudyCommand`] family and
//! lets the daemon seal opaque application bytes before those commands refer
//! to them.  It is intentionally an in-process composition API, not a local
//! socket capability and not a general [`society_kernel::CommandBody`] API.
//!
//! In particular, this is an *admission* boundary.  It does not claim to be a
//! TaskAttempt scheduler: native-child start, prompt delivery, disposal, and
//! recovery remain daemon-owned work and require a later concrete runner.

use society_kernel::{
    Blake3Digest, CommandId, ContentObjectId, StoreError, StudyCommand, StudyTransitionReceipt,
};
use thiserror::Error;

use crate::{
    Daemon, StartupMode,
    content::{ContentSealOperationId, ContentSealingError},
};

const COMMAND_PREFIX: &str = "study-admission-v1";
const CONTENT_PREFIX: &str = "study-admission";
const MAX_OPERATION_BYTES: usize = 36;
const MAX_CONTENT_SLOT_BYTES: usize = 24;

/// Stable, application-selected identity for one resident live-study
/// admission sequence.
///
/// The value is deliberately constrained to a compact ASCII component.  The
/// daemon derives every kernel command and content operation identity from it
/// and from a monotonic application sequence; callers cannot splice their own
/// service command IDs into resident custody.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StudyAdmissionOperationId(String);

impl StudyAdmissionOperationId {
    pub fn parse(value: impl Into<String>) -> Result<Self, StudyAdmissionError> {
        let value = value.into();
        if !valid_label(&value, MAX_OPERATION_BYTES) {
            return Err(StudyAdmissionError::InvalidOperationIdentity);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A compact slot identity for opaque bytes sealed under an admission
/// operation.  It names custody only; it is never an application type,
/// provenance claim, or generic JSON discriminator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StudyAdmissionContentSlot(String);

impl StudyAdmissionContentSlot {
    pub fn parse(value: impl Into<String>) -> Result<Self, StudyAdmissionError> {
        let value = value.into();
        if !valid_label(&value, MAX_CONTENT_SLOT_BYTES) {
            return Err(StudyAdmissionError::InvalidContentSlot);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Immutable-content identity returned after the resident has physically
/// sealed and registered application bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SealedStudyContent {
    content_object_id: ContentObjectId,
    digest: Blake3Digest,
}

impl SealedStudyContent {
    pub const fn content_object_id(self) -> ContentObjectId {
        self.content_object_id
    }

    pub const fn digest(self) -> Blake3Digest {
        self.digest
    }
}

/// Errors deliberately exposed by the application/daemon admission boundary.
/// Physical content failures remain daemon implementation detail; applications
/// receive no content-store path or lower-level writer capability from them.
#[derive(Debug, Error)]
pub enum StudyAdmissionError {
    #[error("study admission operation identity must be canonical lowercase ASCII")]
    InvalidOperationIdentity,
    #[error("study admission content slot must be canonical lowercase ASCII")]
    InvalidContentSlot,
    #[error("study admission sequence must be positive")]
    InvalidSequence,
    #[error("the daemon is recovery-fenced and cannot admit a study")]
    RecoveryFenced,
    #[error("the daemon could not derive a stable study admission command identity")]
    CommandIdentity,
    #[error("the daemon could not derive a stable study content operation identity")]
    ContentOperationIdentity,
    #[error("daemon content sealing failed")]
    ContentSealing,
    #[error("kernel study transition failed: {0}")]
    Kernel(#[from] StoreError),
}

/// Borrowed, single-writer access to one application study-admission sequence.
///
/// It accepts only [`StudyCommand`], whose closed alternatives are owned by
/// the generic kernel.  The application cannot submit arbitrary `CommandBody`
/// values, obtain an active capability grant, read unsealed content, select a
/// native executable, or touch a Pi child through this type.
pub struct StudyAdmissionAuthority<'daemon> {
    daemon: &'daemon mut Daemon,
    operation: StudyAdmissionOperationId,
}

impl<'daemon> StudyAdmissionAuthority<'daemon> {
    pub(crate) fn new(
        daemon: &'daemon mut Daemon,
        operation: StudyAdmissionOperationId,
    ) -> Result<Self, StudyAdmissionError> {
        if daemon.startup_mode() == StartupMode::RecoveryFenced {
            return Err(StudyAdmissionError::RecoveryFenced);
        }
        Ok(Self { daemon, operation })
    }

    /// Resident operation identity shared by every derived custody command.
    pub fn operation(&self) -> &StudyAdmissionOperationId {
        &self.operation
    }

    /// Physically seal and globally register exact application bytes.
    ///
    /// The caller supplies bytes and a compact custody slot only.  The daemon
    /// computes their digest and derives both service command IDs itself.
    pub fn seal_content(
        &mut self,
        slot: StudyAdmissionContentSlot,
        bytes: &[u8],
    ) -> Result<SealedStudyContent, StudyAdmissionError> {
        let digest = Blake3Digest::of_bytes(bytes);
        let label = self.content_label(&slot);
        let operation = ContentSealOperationId::parse(label, digest)
            .map_err(|_| StudyAdmissionError::ContentOperationIdentity)?;
        let registration = self
            .daemon
            .seal_study_admission_content(&operation, bytes)
            .map_err(map_content_error)?;
        Ok(SealedStudyContent {
            content_object_id: registration.content_object_id,
            digest: registration.digest,
        })
    }

    /// Submit one closed generic study transition under the next exact
    /// sequence identity.  A retry must reuse the same sequence and command;
    /// the kernel ledger then supplies idempotency.
    pub fn transition(
        &mut self,
        sequence: u32,
        command: StudyCommand,
    ) -> Result<StudyTransitionReceipt, StudyAdmissionError> {
        let command_id = self.command_id(sequence)?;
        self.daemon
            .execute_study_admission_transition(command_id, command)
    }

    fn command_id(&self, sequence: u32) -> Result<CommandId, StudyAdmissionError> {
        if sequence == 0 {
            return Err(StudyAdmissionError::InvalidSequence);
        }
        CommandId::parse(format!(
            "{COMMAND_PREFIX}/{}/{sequence}",
            self.operation.as_str()
        ))
        .map_err(|_| StudyAdmissionError::CommandIdentity)
    }

    fn content_label(&self, slot: &StudyAdmissionContentSlot) -> String {
        format!(
            "{CONTENT_PREFIX}-{}-{}",
            self.operation.as_str(),
            slot.as_str()
        )
    }
}

fn valid_label(value: &str, maximum_bytes: usize) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= maximum_bytes
        && bytes[0].is_ascii_alphanumeric()
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

fn map_content_error(_error: ContentSealingError) -> StudyAdmissionError {
    StudyAdmissionError::ContentSealing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_and_slot_are_bounded_canonical_components() {
        assert_eq!(
            StudyAdmissionOperationId::parse("cl001-pilot-01")
                .expect("test fixture")
                .as_str(),
            "cl001-pilot-01"
        );
        assert!(matches!(
            StudyAdmissionOperationId::parse("CL001"),
            Err(StudyAdmissionError::InvalidOperationIdentity)
        ));
        assert!(matches!(
            StudyAdmissionContentSlot::parse("plan_bytes"),
            Err(StudyAdmissionError::InvalidContentSlot)
        ));
    }

    #[test]
    fn derived_content_label_cannot_escape_daemon_namespace() {
        let operation = StudyAdmissionOperationId::parse("cl001-pilot-01").expect("test fixture");
        let slot = StudyAdmissionContentSlot::parse("plan").expect("test fixture");
        assert_eq!(
            format!("{CONTENT_PREFIX}-{}-{}", operation.as_str(), slot.as_str()),
            "study-admission-cl001-pilot-01-plan"
        );
    }
}
