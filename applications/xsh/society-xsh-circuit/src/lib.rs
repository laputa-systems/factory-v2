//! Closed XSH adapters for deterministic VS-001 circuit observations.
//!
//! This application-owned crate accepts TSV only at a sealed evaluator
//! boundary. Parsing returns exhaustive Rust values; neither the TSV nor an
//! opaque row is durable state. A parsed observation still is not admitted
//! evidence until a later authority binds its evaluator and separately sealed
//! artifact identities.

use society_content::ContentDigest;
use thiserror::Error;

mod vs001;

pub use vs001::*;

const MAX_BEHAVIOR_OBSERVATION_BYTES: usize = 64 * 1024;
const BEHAVIOR_SCHEMA: &str = "# schema: BehaviorObservationV1/tsv-v1";
const BEHAVIOR_HEADER: &str = "case_id\tconsumer\tinput_manifest\texpected_contract_source\tdisposition\texit_shape\tparent_stdout_blake3\tparent_stderr_blake3\tstdout_evidence_kind\tstdout_evidence_blake3\tstderr_evidence_kind\tstderr_evidence_blake3\tlifecycle";
const BEHAVIOR_FIELD_COUNT: usize = 13;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BehaviorCaseId {
    B01,
    B02,
    B03,
    B04,
    B05,
    B06,
    B07,
    B08,
    B09,
    B10,
    B11,
}

impl BehaviorCaseId {
    const ORDERED: [Self; 11] = [
        Self::B01,
        Self::B02,
        Self::B03,
        Self::B04,
        Self::B05,
        Self::B06,
        Self::B07,
        Self::B08,
        Self::B09,
        Self::B10,
        Self::B11,
    ];

    fn parse(value: &str) -> Option<Self> {
        match value {
            "B01" => Some(Self::B01),
            "B02" => Some(Self::B02),
            "B03" => Some(Self::B03),
            "B04" => Some(Self::B04),
            "B05" => Some(Self::B05),
            "B06" => Some(Self::B06),
            "B07" => Some(Self::B07),
            "B08" => Some(Self::B08),
            "B09" => Some(Self::B09),
            "B10" => Some(Self::B10),
            "B11" => Some(Self::B11),
            _ => None,
        }
    }

    const fn manifest_contract(
        self,
    ) -> (
        CommandConsumer,
        BehaviorInputManifest,
        ExpectedContractSource,
    ) {
        match self {
            Self::B01 => (
                CommandConsumer::ProcessRun,
                BehaviorInputManifest::CommandPathRedirection,
                ExpectedContractSource::SpecCommandPlanStdio,
            ),
            Self::B02 => (
                CommandConsumer::SpawnCommand,
                BehaviorInputManifest::OwnedCommandPathRedirection,
                ExpectedContractSource::SpecSpawnCommandRedirection,
            ),
            Self::B03 => (
                CommandConsumer::SpawnRun,
                BehaviorInputManifest::DirectSpawnRedirection,
                ExpectedContractSource::SpecSpawnRunRedirection,
            ),
            Self::B04 => (
                CommandConsumer::ProcessSpawn,
                BehaviorInputManifest::DetachedCommandPathRedirection,
                ExpectedContractSource::SpecProcessSpawnDetached,
            ),
            Self::B05 => (
                CommandConsumer::SpawnCommand,
                BehaviorInputManifest::OwnedCommandDefaultStdio,
                ExpectedContractSource::SpecSpawnDefaultInherit,
            ),
            Self::B06 => (
                CommandConsumer::ProcessRun,
                BehaviorInputManifest::CommandStderrTruncate,
                ExpectedContractSource::SpecCommandStderrTruncate,
            ),
            Self::B07 => (
                CommandConsumer::ProcessRun,
                BehaviorInputManifest::CommandStderrAppend,
                ExpectedContractSource::SpecCommandStderrAppend,
            ),
            Self::B08 => (
                CommandConsumer::SpawnCommand,
                BehaviorInputManifest::OwnedInvalidStderrDestination,
                ExpectedContractSource::SpecProcessSetupError,
            ),
            Self::B09 => (
                CommandConsumer::SpawnCommand,
                BehaviorInputManifest::OwnedNonzeroStatus,
                ExpectedContractSource::SpecSpawnStatusData,
            ),
            Self::B10 => (
                CommandConsumer::SpawnCommand,
                BehaviorInputManifest::OwnedCancelledSleeper,
                ExpectedContractSource::SpecOsOwnedCancellation,
            ),
            Self::B11 => (
                CommandConsumer::ProcessRun,
                BehaviorInputManifest::CommandStderrDevNull,
                ExpectedContractSource::SpecCommandPathSink,
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandConsumer {
    ProcessRun,
    SpawnCommand,
    SpawnRun,
    ProcessSpawn,
}

impl CommandConsumer {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "process_run" => Some(Self::ProcessRun),
            "spawn_command" => Some(Self::SpawnCommand),
            "spawn_run" => Some(Self::SpawnRun),
            "process_spawn" => Some(Self::ProcessSpawn),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BehaviorInputManifest {
    CommandPathRedirection,
    OwnedCommandPathRedirection,
    DirectSpawnRedirection,
    DetachedCommandPathRedirection,
    OwnedCommandDefaultStdio,
    CommandStderrTruncate,
    CommandStderrAppend,
    OwnedInvalidStderrDestination,
    OwnedNonzeroStatus,
    OwnedCancelledSleeper,
    CommandStderrDevNull,
}

impl BehaviorInputManifest {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "command_path_redirection" => Some(Self::CommandPathRedirection),
            "owned_command_path_redirection" => Some(Self::OwnedCommandPathRedirection),
            "direct_spawn_redirection" => Some(Self::DirectSpawnRedirection),
            "detached_command_path_redirection" => Some(Self::DetachedCommandPathRedirection),
            "owned_command_default_stdio" => Some(Self::OwnedCommandDefaultStdio),
            "command_stderr_truncate" => Some(Self::CommandStderrTruncate),
            "command_stderr_append" => Some(Self::CommandStderrAppend),
            "owned_invalid_stderr_destination" => Some(Self::OwnedInvalidStderrDestination),
            "owned_nonzero_status" => Some(Self::OwnedNonzeroStatus),
            "owned_cancelled_sleeper" => Some(Self::OwnedCancelledSleeper),
            "command_stderr_dev_null" => Some(Self::CommandStderrDevNull),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpectedContractSource {
    SpecCommandPlanStdio,
    SpecSpawnCommandRedirection,
    SpecSpawnRunRedirection,
    SpecProcessSpawnDetached,
    SpecSpawnDefaultInherit,
    SpecCommandStderrTruncate,
    SpecCommandStderrAppend,
    SpecProcessSetupError,
    SpecSpawnStatusData,
    SpecOsOwnedCancellation,
    SpecCommandPathSink,
}

impl ExpectedContractSource {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "spec_command_plan_stdio" => Some(Self::SpecCommandPlanStdio),
            "spec_spawn_command_redirection" => Some(Self::SpecSpawnCommandRedirection),
            "spec_spawn_run_redirection" => Some(Self::SpecSpawnRunRedirection),
            "spec_process_spawn_detached" => Some(Self::SpecProcessSpawnDetached),
            "spec_spawn_default_inherit" => Some(Self::SpecSpawnDefaultInherit),
            "spec_command_stderr_truncate" => Some(Self::SpecCommandStderrTruncate),
            "spec_command_stderr_append" => Some(Self::SpecCommandStderrAppend),
            "spec_process_setup_error" => Some(Self::SpecProcessSetupError),
            "spec_spawn_status_data" => Some(Self::SpecSpawnStatusData),
            "spec_os_owned_cancellation" => Some(Self::SpecOsOwnedCancellation),
            "spec_command_path_sink" => Some(Self::SpecCommandPathSink),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BehaviorDisposition {
    Pass,
    NotApplicable,
}

impl BehaviorDisposition {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "pass" => Some(Self::Pass),
            "not_applicable" => Some(Self::NotApplicable),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessExitStatus(u8);

impl ProcessExitStatus {
    pub const fn value(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitOrErrorShape {
    Exited(ProcessExitStatus),
    DetachedStarted,
    SetupError,
    Cancelled,
}

impl ExitOrErrorShape {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "detached_started" => Some(Self::DetachedStarted),
            "setup_error" => Some(Self::SetupError),
            "cancelled" => Some(Self::Cancelled),
            _ => {
                let status = value.strip_prefix("exited_")?;
                let parsed = status.parse::<u8>().ok()?;
                if status != parsed.to_string() {
                    return None;
                }
                Some(Self::Exited(ProcessExitStatus(parsed)))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParentStream {
    Stdout,
    Stderr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalStreamSink {
    DevNull,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamEvidence {
    Redirected { destination: ContentDigest },
    Inherited { parent_stream: ParentStream },
    NotProduced,
    RedirectionIgnored { destination: ContentDigest },
    RedirectedExternalSink { sink: ExternalStreamSink },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Evaluator-observed task lifecycle, not a daemon process-group receipt.
/// In particular, `OwnedWaited` does not claim OS-level descendant reaping.
pub enum ProcessLifecycleReceipt {
    CompletedStatus,
    OwnedWaited,
    DetachedRecordNoWait,
    DefaultInheritWaited,
    SetupFailedBeforeHandle,
    OwnedWaitedNonzeroStatus,
    CancelReturnedNoDelayedEffect,
}

impl ProcessLifecycleReceipt {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "completed_status" => Some(Self::CompletedStatus),
            "owned_waited" => Some(Self::OwnedWaited),
            "detached_record_no_wait" => Some(Self::DetachedRecordNoWait),
            "default_inherit_waited" => Some(Self::DefaultInheritWaited),
            "setup_failed_before_handle" => Some(Self::SetupFailedBeforeHandle),
            "owned_waited_nonzero_status" => Some(Self::OwnedWaitedNonzeroStatus),
            "cancel_returned_no_delayed_effect" => Some(Self::CancelReturnedNoDelayedEffect),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BehaviorObservationV1 {
    pub case_id: BehaviorCaseId,
    pub consumer: CommandConsumer,
    pub input_manifest: BehaviorInputManifest,
    pub expected_contract_source: ExpectedContractSource,
    pub disposition: BehaviorDisposition,
    pub exit_shape: ExitOrErrorShape,
    pub parent_stdout: ContentDigest,
    pub parent_stderr: ContentDigest,
    pub stdout_evidence: StreamEvidence,
    pub stderr_evidence: StreamEvidence,
    pub lifecycle: ProcessLifecycleReceipt,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BehaviorObservationSetV1 {
    observations: [BehaviorObservationV1; 11],
}

impl BehaviorObservationSetV1 {
    pub fn parse(bytes: &[u8]) -> Result<Self, BehaviorParseError> {
        if bytes.len() > MAX_BEHAVIOR_OBSERVATION_BYTES {
            return Err(BehaviorParseError::FrameTooLarge);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| BehaviorParseError::InvalidUtf8)?;
        if text.contains('\r') {
            return Err(BehaviorParseError::NonCanonicalLineEnding);
        }
        let Some(canonical_text) = text.strip_suffix('\n') else {
            return Err(BehaviorParseError::MissingTerminalLf);
        };
        let mut lines = canonical_text.split('\n');
        if lines.next() != Some(BEHAVIOR_SCHEMA) {
            return Err(BehaviorParseError::WrongSchema);
        }
        if lines.next() != Some(BEHAVIOR_HEADER) {
            return Err(BehaviorParseError::WrongHeader);
        }

        let mut parsed = Vec::with_capacity(BehaviorCaseId::ORDERED.len());
        for (index, expected_case) in BehaviorCaseId::ORDERED.into_iter().enumerate() {
            let line_number = index + 3;
            let line = lines.next().ok_or(BehaviorParseError::MissingCase {
                case: expected_case,
            })?;
            let observation = parse_behavior_row(line, line_number)?;
            if observation.case_id != expected_case {
                return Err(BehaviorParseError::CaseOutOfOrder {
                    line: line_number,
                    expected: expected_case,
                    observed: observation.case_id,
                });
            }
            parsed.push(observation);
        }
        if lines.next().is_some() {
            return Err(BehaviorParseError::ExtraRow);
        }
        let observations = parsed
            .try_into()
            .map_err(|_| BehaviorParseError::InternalCardinality)?;
        Ok(Self { observations })
    }

    pub const fn observations(&self) -> &[BehaviorObservationV1; 11] {
        &self.observations
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BehaviorColumn {
    CaseId,
    Consumer,
    InputManifest,
    ExpectedContractSource,
    Disposition,
    ExitShape,
    ParentStdout,
    ParentStderr,
    StdoutEvidence,
    StderrEvidence,
    Lifecycle,
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum BehaviorParseError {
    #[error("behavior observation file exceeds its fixed byte bound")]
    FrameTooLarge,
    #[error("behavior observation file is not UTF-8")]
    InvalidUtf8,
    #[error("behavior observation file must use LF line endings")]
    NonCanonicalLineEnding,
    #[error("behavior observation file must end in exactly one LF-terminated record")]
    MissingTerminalLf,
    #[error("behavior observation schema line is not exact")]
    WrongSchema,
    #[error("behavior observation header is not exact")]
    WrongHeader,
    #[error("behavior case {case:?} is missing")]
    MissingCase { case: BehaviorCaseId },
    #[error("behavior row {line} has {observed} fields instead of 13")]
    WrongFieldCount { line: usize, observed: usize },
    #[error("behavior row {line} has an unknown value in {column:?}")]
    UnknownClosedValue { line: usize, column: BehaviorColumn },
    #[error("behavior row {line} has a noncanonical digest in {column:?}")]
    InvalidDigest { line: usize, column: BehaviorColumn },
    #[error("behavior row {line} has an invalid evidence-kind/digest pair in {column:?}")]
    InvalidStreamEvidence { line: usize, column: BehaviorColumn },
    #[error("behavior row {line} does not match the sealed manifest for its case")]
    CaseManifestMismatch { line: usize },
    #[error("behavior row {line} is out of order: expected {expected:?}, got {observed:?}")]
    CaseOutOfOrder {
        line: usize,
        expected: BehaviorCaseId,
        observed: BehaviorCaseId,
    },
    #[error("behavior observation file has an extra row")]
    ExtraRow,
    #[error("closed behavior observation cardinality could not be constructed")]
    InternalCardinality,
}

fn parse_behavior_row(
    line: &str,
    line_number: usize,
) -> Result<BehaviorObservationV1, BehaviorParseError> {
    let fields: Vec<_> = line.split('\t').collect();
    if fields.len() != BEHAVIOR_FIELD_COUNT {
        return Err(BehaviorParseError::WrongFieldCount {
            line: line_number,
            observed: fields.len(),
        });
    }
    let unknown = |column| BehaviorParseError::UnknownClosedValue {
        line: line_number,
        column,
    };
    let case_id =
        BehaviorCaseId::parse(fields[0]).ok_or_else(|| unknown(BehaviorColumn::CaseId))?;
    let consumer =
        CommandConsumer::parse(fields[1]).ok_or_else(|| unknown(BehaviorColumn::Consumer))?;
    let input_manifest = BehaviorInputManifest::parse(fields[2])
        .ok_or_else(|| unknown(BehaviorColumn::InputManifest))?;
    let expected_contract_source = ExpectedContractSource::parse(fields[3])
        .ok_or_else(|| unknown(BehaviorColumn::ExpectedContractSource))?;
    if (consumer, input_manifest, expected_contract_source) != case_id.manifest_contract() {
        return Err(BehaviorParseError::CaseManifestMismatch { line: line_number });
    }
    let disposition = BehaviorDisposition::parse(fields[4])
        .ok_or_else(|| unknown(BehaviorColumn::Disposition))?;
    let exit_shape =
        ExitOrErrorShape::parse(fields[5]).ok_or_else(|| unknown(BehaviorColumn::ExitShape))?;
    let parent_stdout = parse_digest(fields[6], line_number, BehaviorColumn::ParentStdout)?;
    let parent_stderr = parse_digest(fields[7], line_number, BehaviorColumn::ParentStderr)?;
    let stdout_evidence = parse_stream_evidence(
        fields[8],
        fields[9],
        line_number,
        BehaviorColumn::StdoutEvidence,
        ParentStream::Stdout,
    )?;
    let stderr_evidence = parse_stream_evidence(
        fields[10],
        fields[11],
        line_number,
        BehaviorColumn::StderrEvidence,
        ParentStream::Stderr,
    )?;
    let lifecycle = ProcessLifecycleReceipt::parse(fields[12])
        .ok_or_else(|| unknown(BehaviorColumn::Lifecycle))?;
    Ok(BehaviorObservationV1 {
        case_id,
        consumer,
        input_manifest,
        expected_contract_source,
        disposition,
        exit_shape,
        parent_stdout,
        parent_stderr,
        stdout_evidence,
        stderr_evidence,
        lifecycle,
    })
}

fn parse_digest(
    value: &str,
    line: usize,
    column: BehaviorColumn,
) -> Result<ContentDigest, BehaviorParseError> {
    ContentDigest::parse(value).map_err(|_| BehaviorParseError::InvalidDigest { line, column })
}

fn parse_stream_evidence(
    kind: &str,
    digest: &str,
    line: usize,
    column: BehaviorColumn,
    expected_parent_stream: ParentStream,
) -> Result<StreamEvidence, BehaviorParseError> {
    let digest_value = || parse_digest(digest, line, column);
    match (kind, digest) {
        ("redirected", "-") | ("redirection_ignored", "-") => {
            Err(BehaviorParseError::InvalidStreamEvidence { line, column })
        }
        ("redirected", _) => Ok(StreamEvidence::Redirected {
            destination: digest_value()?,
        }),
        ("redirection_ignored", _) => Ok(StreamEvidence::RedirectionIgnored {
            destination: digest_value()?,
        }),
        ("inherited_parent_stdout", "-") if expected_parent_stream == ParentStream::Stdout => {
            Ok(StreamEvidence::Inherited {
                parent_stream: ParentStream::Stdout,
            })
        }
        ("inherited_parent_stderr", "-") if expected_parent_stream == ParentStream::Stderr => {
            Ok(StreamEvidence::Inherited {
                parent_stream: ParentStream::Stderr,
            })
        }
        ("not_produced", "-") => Ok(StreamEvidence::NotProduced),
        ("redirected_dev_null", "-") => Ok(StreamEvidence::RedirectedExternalSink {
            sink: ExternalStreamSink::DevNull,
        }),
        (
            "inherited_parent_stdout"
            | "inherited_parent_stderr"
            | "not_produced"
            | "redirected_dev_null",
            _,
        ) => Err(BehaviorParseError::InvalidStreamEvidence { line, column }),
        _ => Err(BehaviorParseError::UnknownClosedValue { line, column }),
    }
}
