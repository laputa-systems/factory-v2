//! Closed, versioned Unix-domain wire values for the resident authority.
//!
//! The outer frame is `u32-be length || payload`. Every payload starts with a
//! fixed protocol version, a closed request/response tag, and a nonzero typed
//! correlation id. There is intentionally no JSON, map, generic payload, or
//! extension tag. Adding a command changes this module exhaustively.

use std::io::{self, Read, Write};

use society_kernel::{
    AdmissionGeneration, ApplicationIdentity, ApplicationMissionInput, ApplicationName,
    ApplicationRevisionOrdinal, Blake3Digest, CancellationMode, Capability, CapabilityGrantId,
    CommandBody, CommandDisposition, CommandId, CommandReceipt, CommandRequest, CostPostmortemId,
    CostPostmortemResolution, ExpectedGeneration, MissionPrinciple, MissionPrincipleKind,
    MissionPrincipleText, MissionPrinciples, MissionSourceRendering, MissionStatement,
    NorthStarBoundaryCommitmentQuestion, NorthStarChangeQuestion,
    NorthStarImprovementEvidenceQuestion, NorthStarQuestionSet, NorthStarRevisitQuestion,
    OfficeTurnPurpose, OperatingCycleId, OperatingCycleTreatment, PrincipalDisplayName,
    PrincipalId, Rejection, RootAuthorityOfficeSessionId, SocietyName, StudyEpisodeId,
    StudyEpisodeObservation, StudyEpisodeState, StudyInstitutionRevisionId,
    StudyMeasurementObservation, StudyMeasurementRevisionId, StudyMeasurementSlot,
    StudyMeasurementSlotCount, StudyMeasurementStatus, StudyPairId, StudyPairObservation,
    StudyPopulationSnapshotId, StudyProtocolRevisionId, StudyRunId, StudyRunLifecycleState,
    StudyRunObservation, StudyRunPairCount, StudyRunPairOrdinal,
    StudyRunPairRegistrationObservation, StudyRunRegisteredPairCount, StudyTreatment,
    StudyWorldRevisionId, UsdMicros,
};
use thiserror::Error;

pub const PROTOCOL_VERSION: u16 = 8;
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

// Request discriminants are intentionally partitioned by transport. A raw
// supervisor `Execute` tag is unknown to the named monitor socket before any
// command-shaped body could be considered.
const PUBLIC_RECEIPT_TAG: u8 = 0x21;
const PUBLIC_STATUS_TAG: u8 = 0x22;
const PUBLIC_STUDY_PAIR_OBSERVATION_TAG: u8 = 0x23;
const PUBLIC_STUDY_RUN_OBSERVATION_TAG: u8 = 0x24;
const PUBLIC_STUDY_RUN_PAIR_REGISTRATION_TAG: u8 = 0x25;
const SUPERVISOR_EXECUTE_TAG: u8 = 0x41;
const SUPERVISOR_RECEIPT_TAG: u8 = 0x42;
const SUPERVISOR_STATUS_TAG: u8 = 0x43;
const SUPERVISOR_CAPABILITY_GRANT_TAG: u8 = 0x44;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CorrelationId(u64);

impl CorrelationId {
    pub const fn new(value: u64) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

/// The protocol only represents supervisor-authorized command families.
/// Kernel lifecycle facts such as `RecordCycleDrained` are deliberately
/// absent: they are constructed by the resident control loop, never by a peer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClientCommandBody {
    CreateSocietyIdentity {
        name: SocietyName,
    },
    InstallRootAuthorityOffice,
    InstallFoundingMission {
        mission: Box<ApplicationMissionInput>,
        source_rendering: MissionSourceRendering,
    },
    AppointInitialRootAuthority {
        actor_display_name: PrincipalDisplayName,
    },
    SetR0HardCeiling {
        ceiling: UsdMicros,
    },
    BootstrapSociety,
    ProposeOperatingCycle {
        treatment: OperatingCycleTreatment,
        budget_ceiling: UsdMicros,
    },
    AdmitOperatingCycle {
        cycle_id: OperatingCycleId,
    },
    StartRootAuthorityOfficeSession {
        cycle_id: OperatingCycleId,
    },
    OpenOfficeTurn {
        session_id: RootAuthorityOfficeSessionId,
        purpose: OfficeTurnPurpose,
    },
    QuiesceOperatingCycle {
        cycle_id: OperatingCycleId,
    },
    ResumeOperatingCycle {
        cycle_id: OperatingCycleId,
    },
    ReconcileOperatingCycle {
        cycle_id: OperatingCycleId,
    },
    CloseOperatingCycle {
        cycle_id: OperatingCycleId,
    },
    ReserveBudget {
        cycle_id: OperatingCycleId,
        amount: UsdMicros,
    },
    RequestCancellation {
        cycle_id: OperatingCycleId,
        mode: CancellationMode,
    },
    CloseCostPostmortem {
        postmortem_id: CostPostmortemId,
        resolution: CostPostmortemResolution,
    },
}

impl ClientCommandBody {
    fn tag(&self) -> u8 {
        match self {
            Self::CreateSocietyIdentity { .. } => 1,
            Self::InstallRootAuthorityOffice => 2,
            Self::InstallFoundingMission { .. } => 3,
            Self::AppointInitialRootAuthority { .. } => 4,
            Self::SetR0HardCeiling { .. } => 5,
            Self::BootstrapSociety => 6,
            Self::ProposeOperatingCycle { .. } => 7,
            Self::AdmitOperatingCycle { .. } => 8,
            Self::StartRootAuthorityOfficeSession { .. } => 9,
            Self::OpenOfficeTurn { .. } => 11,
            Self::QuiesceOperatingCycle { .. } => 13,
            Self::ResumeOperatingCycle { .. } => 15,
            Self::ReconcileOperatingCycle { .. } => 16,
            Self::CloseOperatingCycle { .. } => 17,
            Self::ReserveBudget { .. } => 18,
            Self::RequestCancellation { .. } => 20,
            Self::CloseCostPostmortem { .. } => 23,
        }
    }

    fn into_kernel(self) -> CommandBody {
        match self {
            Self::CreateSocietyIdentity { name } => CommandBody::CreateSocietyIdentity { name },
            Self::InstallRootAuthorityOffice => CommandBody::InstallRootAuthorityOffice,
            Self::InstallFoundingMission { mission, .. } => {
                CommandBody::InstallFoundingMission { mission: *mission }
            }
            Self::AppointInitialRootAuthority { actor_display_name } => {
                CommandBody::AppointInitialRootAuthority { actor_display_name }
            }
            Self::SetR0HardCeiling { ceiling } => CommandBody::SetR0HardCeiling { ceiling },
            Self::BootstrapSociety => CommandBody::BootstrapSociety,
            Self::ProposeOperatingCycle {
                treatment,
                budget_ceiling,
            } => CommandBody::ProposeOperatingCycle {
                treatment,
                budget_ceiling,
            },
            Self::AdmitOperatingCycle { cycle_id } => CommandBody::AdmitOperatingCycle { cycle_id },
            Self::StartRootAuthorityOfficeSession { cycle_id } => {
                CommandBody::StartRootAuthorityOfficeSession { cycle_id }
            }
            Self::OpenOfficeTurn {
                session_id,
                purpose,
            } => CommandBody::OpenOfficeTurn {
                session_id,
                purpose,
            },
            Self::QuiesceOperatingCycle { cycle_id } => {
                CommandBody::QuiesceOperatingCycle { cycle_id }
            }
            Self::ResumeOperatingCycle { cycle_id } => {
                CommandBody::ResumeOperatingCycle { cycle_id }
            }
            Self::ReconcileOperatingCycle { cycle_id } => {
                CommandBody::ReconcileOperatingCycle { cycle_id }
            }
            Self::CloseOperatingCycle { cycle_id } => CommandBody::CloseOperatingCycle { cycle_id },
            Self::ReserveBudget { cycle_id, amount } => {
                CommandBody::ReserveBudget { cycle_id, amount }
            }
            Self::RequestCancellation { cycle_id, mode } => {
                CommandBody::RequestCancellation { cycle_id, mode }
            }
            Self::CloseCostPostmortem {
                postmortem_id,
                resolution,
            } => CommandBody::CloseCostPostmortem {
                postmortem_id,
                resolution,
            },
        }
    }

    fn required_capability(&self) -> Capability {
        match self {
            Self::CreateSocietyIdentity { .. } => Capability::CreateSocietyIdentity,
            Self::InstallRootAuthorityOffice => Capability::InstallRootAuthorityOffice,
            Self::InstallFoundingMission { .. } => Capability::InstallFoundingMission,
            Self::AppointInitialRootAuthority { .. } => Capability::AppointInitialRootAuthority,
            Self::SetR0HardCeiling { .. } => Capability::SetR0HardCeiling,
            Self::BootstrapSociety => Capability::BootstrapSociety,
            Self::ProposeOperatingCycle { .. } => Capability::ProposeOperatingCycle,
            Self::AdmitOperatingCycle { .. } => Capability::AdmitOperatingCycle,
            Self::StartRootAuthorityOfficeSession { .. } => {
                Capability::StartRootAuthorityOfficeSession
            }
            Self::OpenOfficeTurn { .. } => Capability::OpenOfficeTurn,
            Self::QuiesceOperatingCycle { .. } => Capability::QuiesceOperatingCycle,
            Self::ResumeOperatingCycle { .. } => Capability::ResumeOperatingCycle,
            Self::ReconcileOperatingCycle { .. } => Capability::ReconcileOperatingCycle,
            Self::CloseOperatingCycle { .. } => Capability::CloseOperatingCycle,
            Self::ReserveBudget { .. } => Capability::ReserveBudget,
            Self::RequestCancellation { .. } => Capability::RequestCancellation,
            Self::CloseCostPostmortem { .. } => Capability::CloseCostPostmortem,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientCommandRequest {
    pub command_id: CommandId,
    pub principal_id: PrincipalId,
    pub capability_grant_id: CapabilityGrantId,
    pub capability: Capability,
    pub expected_generation: ExpectedGeneration,
    pub body: ClientCommandBody,
}

impl ClientCommandRequest {
    pub(crate) fn into_kernel(self) -> CommandRequest {
        CommandRequest {
            command_id: self.command_id,
            principal_id: self.principal_id,
            capability_grant_id: self.capability_grant_id,
            capability: self.capability,
            expected_generation: self.expected_generation,
            body: self.body.into_kernel(),
        }
    }
}

/// Closed monitor protocol admitted on the named `societyd.sock`. It contains
/// no command tag: a same-UID actor may monitor receipt/status state but cannot
/// even encode a kernel command for this pathname.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublicRequest {
    CommandReceipt {
        correlation: CorrelationId,
        command_id: CommandId,
    },
    Status {
        correlation: CorrelationId,
    },
    StudyPairObservation {
        correlation: CorrelationId,
        pair_id: StudyPairId,
    },
    StudyRunObservation {
        correlation: CorrelationId,
        study_run_id: StudyRunId,
    },
    StudyRunPairRegistration {
        correlation: CorrelationId,
        study_run_id: StudyRunId,
        pair_ordinal: StudyRunPairOrdinal,
    },
}

/// Closed command/query protocol carried only over the anonymous inherited
/// supervisor stream. It is deliberately a distinct codec from
/// [`PublicRequest`], so its execute tag is not public-socket vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SupervisorRequest {
    Execute {
        correlation: CorrelationId,
        command: ClientCommandRequest,
    },
    CommandReceipt {
        correlation: CorrelationId,
        command_id: CommandId,
    },
    Status {
        correlation: CorrelationId,
    },
    ActiveCapabilityGrant {
        correlation: CorrelationId,
        principal_id: PrincipalId,
        capability: Capability,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandReceiptView {
    Accepted {
        event_id: society_kernel::EventId,
        idempotent: bool,
    },
    Rejected {
        rejection: Rejection,
        idempotent: bool,
    },
}

impl From<CommandReceipt> for CommandReceiptView {
    fn from(receipt: CommandReceipt) -> Self {
        match receipt.disposition {
            CommandDisposition::Accepted(event_id) => Self::Accepted {
                event_id,
                idempotent: receipt.idempotent,
            },
            CommandDisposition::Rejected(rejection) => Self::Rejected {
                rejection,
                idempotent: receipt.idempotent,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DaemonStatus {
    FreshServing { command_count: i64 },
    RecoveryFenced { command_count: i64 },
}

/// Bounded run-level facts returned by the named monitor. Pair registrations
/// are deliberately read one ordinal at a time: a valid generic run can hold
/// 10,000 pairs, which must not overflow the fixed 64 KiB wire frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StudyRunSummary {
    pub study_run_id: StudyRunId,
    pub protocol_revision_id: StudyProtocolRevisionId,
    pub plan_content_object_id: society_kernel::ContentObjectId,
    pub plan_digest: Blake3Digest,
    pub pair_count: StudyRunPairCount,
    pub registered_pair_count: StudyRunRegisteredPairCount,
    pub lifecycle_state: StudyRunLifecycleState,
}

impl From<&StudyRunObservation> for StudyRunSummary {
    fn from(study_run: &StudyRunObservation) -> Self {
        Self {
            study_run_id: study_run.study_run_id,
            protocol_revision_id: study_run.protocol_revision_id,
            plan_content_object_id: study_run.plan_content_object_id,
            plan_digest: study_run.plan_digest,
            pair_count: study_run.pair_count,
            registered_pair_count: study_run.registered_pair_count,
            lifecycle_state: study_run.lifecycle_state,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ProtocolErrorCode {
    MalformedFrame = 1,
    UnsupportedVersion = 2,
    UnknownTag = 3,
    PeerNotAuthorized = 4,
    RecoveryFenced = 5,
    IdempotencyConflict = 6,
    KernelFailure = 7,
    DaemonStopping = 8,
    MissionSourceDigestMismatch = 9,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Response {
    CommandReceipt {
        correlation: CorrelationId,
        receipt: CommandReceiptView,
    },
    CommandReceiptLookup {
        correlation: CorrelationId,
        receipt: Option<CommandReceiptView>,
    },
    Status {
        correlation: CorrelationId,
        status: DaemonStatus,
    },
    StudyPairObservation {
        correlation: CorrelationId,
        pair: Option<StudyPairObservation>,
    },
    StudyRunSummary {
        correlation: CorrelationId,
        study_run: Option<StudyRunSummary>,
    },
    StudyRunPairRegistration {
        correlation: CorrelationId,
        registration: Option<StudyRunPairRegistrationObservation>,
    },
    ActiveCapabilityGrant {
        correlation: CorrelationId,
        capability_grant_id: Option<CapabilityGrantId>,
    },
    Error {
        correlation: CorrelationId,
        code: ProtocolErrorCode,
    },
}

#[derive(Debug, Error)]
pub enum WireError {
    #[error("socket I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("the peer closed before a frame began")]
    EndOfStream,
    #[error("frame is shorter than its declared length")]
    ShortFrame,
    #[error("frame exceeds the fixed maximum")]
    FrameTooLarge,
    #[error("missing required field")]
    MissingField,
    #[error("unsupported protocol version")]
    UnsupportedVersion,
    #[error("unknown closed tag")]
    UnknownTag,
    #[error("correlation id must be nonzero")]
    InvalidCorrelation,
    #[error("invalid fixed-width boolean")]
    InvalidBoolean,
    #[error("invalid UTF-8 text")]
    InvalidUtf8,
    #[error("text field exceeds its declared maximum")]
    StringTooLong,
    #[error("byte field exceeds its declared maximum")]
    BytesTooLong,
    #[error("field does not satisfy its closed domain type")]
    InvalidValue,
    #[error("trailing bytes after a complete closed message")]
    TrailingBytes,
}

pub fn write_public_request(
    writer: &mut impl Write,
    request: &PublicRequest,
) -> Result<(), WireError> {
    let mut payload = Vec::new();
    put_u16(&mut payload, PROTOCOL_VERSION);
    match request {
        PublicRequest::CommandReceipt {
            correlation,
            command_id,
        } => {
            put_u8(&mut payload, PUBLIC_RECEIPT_TAG);
            put_correlation(&mut payload, *correlation);
            put_string(&mut payload, command_id.as_str());
        }
        PublicRequest::Status { correlation } => {
            put_u8(&mut payload, PUBLIC_STATUS_TAG);
            put_correlation(&mut payload, *correlation);
        }
        PublicRequest::StudyPairObservation {
            correlation,
            pair_id,
        } => {
            put_u8(&mut payload, PUBLIC_STUDY_PAIR_OBSERVATION_TAG);
            put_correlation(&mut payload, *correlation);
            put_i64(&mut payload, pair_id.value());
        }
        PublicRequest::StudyRunObservation {
            correlation,
            study_run_id,
        } => {
            put_u8(&mut payload, PUBLIC_STUDY_RUN_OBSERVATION_TAG);
            put_correlation(&mut payload, *correlation);
            put_i64(&mut payload, study_run_id.value());
        }
        PublicRequest::StudyRunPairRegistration {
            correlation,
            study_run_id,
            pair_ordinal,
        } => {
            put_u8(&mut payload, PUBLIC_STUDY_RUN_PAIR_REGISTRATION_TAG);
            put_correlation(&mut payload, *correlation);
            put_i64(&mut payload, study_run_id.value());
            put_u16(&mut payload, pair_ordinal.value());
        }
    }
    write_frame(writer, &payload)
}

pub fn read_public_request(reader: &mut impl Read) -> Result<PublicRequest, WireError> {
    let frame = read_frame(reader)?;
    let mut cursor = Cursor::new(&frame);
    let version = cursor.u16()?;
    if version != PROTOCOL_VERSION {
        return Err(WireError::UnsupportedVersion);
    }
    let tag = cursor.u8()?;
    let correlation = cursor.correlation()?;
    let request = match tag {
        PUBLIC_RECEIPT_TAG => PublicRequest::CommandReceipt {
            correlation,
            command_id: parse_command_id(cursor.string(128)?)?,
        },
        PUBLIC_STATUS_TAG => PublicRequest::Status { correlation },
        PUBLIC_STUDY_PAIR_OBSERVATION_TAG => PublicRequest::StudyPairObservation {
            correlation,
            pair_id: StudyPairId::try_from(cursor.i64()?).map_err(|_| WireError::InvalidValue)?,
        },
        PUBLIC_STUDY_RUN_OBSERVATION_TAG => PublicRequest::StudyRunObservation {
            correlation,
            study_run_id: StudyRunId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
        },
        PUBLIC_STUDY_RUN_PAIR_REGISTRATION_TAG => PublicRequest::StudyRunPairRegistration {
            correlation,
            study_run_id: StudyRunId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
            pair_ordinal: StudyRunPairOrdinal::try_from(i64::from(cursor.u16()?))
                .map_err(|_| WireError::InvalidValue)?,
        },
        _ => return Err(WireError::UnknownTag),
    };
    cursor.finish()?;
    Ok(request)
}

pub fn write_supervisor_request(
    writer: &mut impl Write,
    request: &SupervisorRequest,
) -> Result<(), WireError> {
    let mut payload = Vec::new();
    put_u16(&mut payload, PROTOCOL_VERSION);
    match request {
        SupervisorRequest::Execute {
            correlation,
            command,
        } => {
            put_u8(&mut payload, SUPERVISOR_EXECUTE_TAG);
            put_correlation(&mut payload, *correlation);
            encode_command_request(&mut payload, command)?;
        }
        SupervisorRequest::CommandReceipt {
            correlation,
            command_id,
        } => {
            put_u8(&mut payload, SUPERVISOR_RECEIPT_TAG);
            put_correlation(&mut payload, *correlation);
            put_string(&mut payload, command_id.as_str());
        }
        SupervisorRequest::Status { correlation } => {
            put_u8(&mut payload, SUPERVISOR_STATUS_TAG);
            put_correlation(&mut payload, *correlation);
        }
        SupervisorRequest::ActiveCapabilityGrant {
            correlation,
            principal_id,
            capability,
        } => {
            if !supervisor_capability_is_representable(*capability) {
                return Err(WireError::InvalidValue);
            }
            put_u8(&mut payload, SUPERVISOR_CAPABILITY_GRANT_TAG);
            put_correlation(&mut payload, *correlation);
            put_i64(&mut payload, principal_id.value());
            put_u8(&mut payload, *capability as u8);
        }
    }
    write_frame(writer, &payload)
}

pub fn read_supervisor_request(reader: &mut impl Read) -> Result<SupervisorRequest, WireError> {
    let frame = read_frame(reader)?;
    let mut cursor = Cursor::new(&frame);
    let version = cursor.u16()?;
    if version != PROTOCOL_VERSION {
        return Err(WireError::UnsupportedVersion);
    }
    let tag = cursor.u8()?;
    let correlation = cursor.correlation()?;
    let request = match tag {
        SUPERVISOR_EXECUTE_TAG => SupervisorRequest::Execute {
            correlation,
            command: decode_command_request(&mut cursor)?,
        },
        SUPERVISOR_RECEIPT_TAG => SupervisorRequest::CommandReceipt {
            correlation,
            command_id: parse_command_id(cursor.string(128)?)?,
        },
        SUPERVISOR_STATUS_TAG => SupervisorRequest::Status { correlation },
        SUPERVISOR_CAPABILITY_GRANT_TAG => SupervisorRequest::ActiveCapabilityGrant {
            correlation,
            principal_id: PrincipalId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
            capability: capability_from_u8(cursor.u8()?)?,
        },
        _ => return Err(WireError::UnknownTag),
    };
    cursor.finish()?;
    Ok(request)
}

pub fn write_response(writer: &mut impl Write, response: &Response) -> Result<(), WireError> {
    let mut payload = Vec::new();
    put_u16(&mut payload, PROTOCOL_VERSION);
    match response {
        Response::CommandReceipt {
            correlation,
            receipt,
        } => {
            put_u8(&mut payload, 1);
            put_correlation(&mut payload, *correlation);
            encode_receipt(&mut payload, *receipt);
        }
        Response::CommandReceiptLookup {
            correlation,
            receipt,
        } => {
            put_u8(&mut payload, 2);
            put_correlation(&mut payload, *correlation);
            match receipt {
                Some(receipt) => {
                    put_u8(&mut payload, 1);
                    encode_receipt(&mut payload, *receipt);
                }
                None => put_u8(&mut payload, 0),
            }
        }
        Response::Status {
            correlation,
            status,
        } => {
            put_u8(&mut payload, 3);
            put_correlation(&mut payload, *correlation);
            match status {
                DaemonStatus::FreshServing { command_count } => {
                    put_u8(&mut payload, 1);
                    put_i64(&mut payload, *command_count);
                }
                DaemonStatus::RecoveryFenced { command_count } => {
                    put_u8(&mut payload, 2);
                    put_i64(&mut payload, *command_count);
                }
            }
        }
        Response::StudyPairObservation { correlation, pair } => {
            put_u8(&mut payload, 6);
            put_correlation(&mut payload, *correlation);
            match pair {
                Some(pair) => {
                    put_bool(&mut payload, true);
                    encode_study_pair_observation(&mut payload, pair)?;
                }
                None => put_bool(&mut payload, false),
            }
        }
        Response::StudyRunSummary {
            correlation,
            study_run,
        } => {
            put_u8(&mut payload, 7);
            put_correlation(&mut payload, *correlation);
            match study_run {
                Some(study_run) => {
                    put_bool(&mut payload, true);
                    encode_study_run_summary(&mut payload, study_run);
                }
                None => put_bool(&mut payload, false),
            }
        }
        Response::StudyRunPairRegistration {
            correlation,
            registration,
        } => {
            put_u8(&mut payload, 8);
            put_correlation(&mut payload, *correlation);
            match registration {
                Some(registration) => {
                    put_bool(&mut payload, true);
                    encode_study_run_pair_registration(&mut payload, *registration);
                }
                None => put_bool(&mut payload, false),
            }
        }
        Response::Error { correlation, code } => {
            put_u8(&mut payload, 4);
            put_correlation(&mut payload, *correlation);
            put_u8(&mut payload, *code as u8);
        }
        Response::ActiveCapabilityGrant {
            correlation,
            capability_grant_id,
        } => {
            put_u8(&mut payload, 5);
            put_correlation(&mut payload, *correlation);
            match capability_grant_id {
                Some(capability_grant_id) => {
                    put_bool(&mut payload, true);
                    put_i64(&mut payload, capability_grant_id.value());
                }
                None => put_bool(&mut payload, false),
            }
        }
    }
    write_frame(writer, &payload)
}

pub fn read_response(reader: &mut impl Read) -> Result<Response, WireError> {
    let frame = read_frame(reader)?;
    let mut cursor = Cursor::new(&frame);
    let version = cursor.u16()?;
    if version != PROTOCOL_VERSION {
        return Err(WireError::UnsupportedVersion);
    }
    let tag = cursor.u8()?;
    let correlation = cursor.correlation()?;
    let response = match tag {
        1 => Response::CommandReceipt {
            correlation,
            receipt: decode_receipt(&mut cursor)?,
        },
        2 => match cursor.bool()? {
            true => Response::CommandReceiptLookup {
                correlation,
                receipt: Some(decode_receipt(&mut cursor)?),
            },
            false => Response::CommandReceiptLookup {
                correlation,
                receipt: None,
            },
        },
        3 => {
            let state = cursor.u8()?;
            let command_count = cursor.i64()?;
            let status = match state {
                1 => DaemonStatus::FreshServing { command_count },
                2 => DaemonStatus::RecoveryFenced { command_count },
                _ => return Err(WireError::InvalidValue),
            };
            Response::Status {
                correlation,
                status,
            }
        }
        4 => Response::Error {
            correlation,
            code: protocol_error_from_u8(cursor.u8()?)?,
        },
        5 => Response::ActiveCapabilityGrant {
            correlation,
            capability_grant_id: match cursor.bool()? {
                true => Some(
                    CapabilityGrantId::try_from(cursor.i64()?)
                        .map_err(|_| WireError::InvalidValue)?,
                ),
                false => None,
            },
        },
        6 => Response::StudyPairObservation {
            correlation,
            pair: match cursor.bool()? {
                true => Some(decode_study_pair_observation(&mut cursor)?),
                false => None,
            },
        },
        7 => Response::StudyRunSummary {
            correlation,
            study_run: match cursor.bool()? {
                true => Some(decode_study_run_summary(&mut cursor)?),
                false => None,
            },
        },
        8 => Response::StudyRunPairRegistration {
            correlation,
            registration: match cursor.bool()? {
                true => Some(decode_study_run_pair_registration(&mut cursor)?),
                false => None,
            },
        },
        _ => return Err(WireError::UnknownTag),
    };
    cursor.finish()?;
    Ok(response)
}

/// Writes exactly one length-prefixed payload. The length is checked before it
/// crosses the boundary, so a future caller cannot create an oversized frame.
pub fn write_frame(writer: &mut impl Write, payload: &[u8]) -> Result<(), WireError> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err(WireError::FrameTooLarge);
    }
    let length = u32::try_from(payload.len()).map_err(|_| WireError::FrameTooLarge)?;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(payload)?;
    writer.flush()?;
    Ok(())
}

/// Reads one frame without accepting a partial prefix or payload as a message.
pub fn read_frame(reader: &mut impl Read) -> Result<Vec<u8>, WireError> {
    let mut length_bytes = [0_u8; 4];
    match reader.read(&mut length_bytes[..1]) {
        Ok(0) => return Err(WireError::EndOfStream),
        Ok(1) => {}
        Ok(_) => unreachable!("one-byte read cannot exceed its buffer"),
        Err(error) => return Err(WireError::Io(error)),
    }
    if let Err(error) = reader.read_exact(&mut length_bytes[1..]) {
        return if error.kind() == io::ErrorKind::UnexpectedEof {
            Err(WireError::ShortFrame)
        } else {
            Err(WireError::Io(error))
        };
    }
    let length = u32::from_be_bytes(length_bytes) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(WireError::FrameTooLarge);
    }
    let mut payload = vec![0_u8; length];
    if let Err(error) = reader.read_exact(&mut payload) {
        return if error.kind() == io::ErrorKind::UnexpectedEof {
            Err(WireError::ShortFrame)
        } else {
            Err(WireError::Io(error))
        };
    }
    Ok(payload)
}

fn encode_command_request(
    bytes: &mut Vec<u8>,
    command: &ClientCommandRequest,
) -> Result<(), WireError> {
    if !supervisor_capability_is_representable(command.capability)
        || command.capability != command.body.required_capability()
    {
        return Err(WireError::InvalidValue);
    }
    put_string(bytes, command.command_id.as_str());
    put_i64(bytes, command.principal_id.value());
    put_i64(bytes, command.capability_grant_id.value());
    put_u8(bytes, command.capability as u8);
    encode_expected_generation(bytes, command.expected_generation);
    put_u8(bytes, command.body.tag());
    match &command.body {
        ClientCommandBody::CreateSocietyIdentity { name } => put_string(bytes, name.as_str()),
        ClientCommandBody::InstallRootAuthorityOffice | ClientCommandBody::BootstrapSociety => {}
        ClientCommandBody::InstallFoundingMission {
            mission,
            source_rendering,
        } => {
            encode_application_mission_input(bytes, mission);
            put_bytes(bytes, source_rendering.as_bytes());
        }
        ClientCommandBody::AppointInitialRootAuthority { actor_display_name } => {
            put_string(bytes, actor_display_name.as_str());
        }
        ClientCommandBody::SetR0HardCeiling { ceiling } => put_i64(bytes, ceiling.value()),
        ClientCommandBody::ProposeOperatingCycle {
            treatment,
            budget_ceiling,
        } => {
            put_u8(bytes, *treatment as u8);
            put_i64(bytes, budget_ceiling.value());
        }
        ClientCommandBody::AdmitOperatingCycle { cycle_id }
        | ClientCommandBody::StartRootAuthorityOfficeSession { cycle_id }
        | ClientCommandBody::QuiesceOperatingCycle { cycle_id }
        | ClientCommandBody::ResumeOperatingCycle { cycle_id }
        | ClientCommandBody::ReconcileOperatingCycle { cycle_id }
        | ClientCommandBody::CloseOperatingCycle { cycle_id } => put_i64(bytes, cycle_id.value()),
        ClientCommandBody::OpenOfficeTurn {
            session_id,
            purpose,
        } => {
            put_i64(bytes, session_id.value());
            put_u8(bytes, *purpose as u8);
        }
        ClientCommandBody::ReserveBudget { cycle_id, amount } => {
            put_i64(bytes, cycle_id.value());
            put_i64(bytes, amount.value());
        }
        ClientCommandBody::RequestCancellation { cycle_id, mode } => {
            put_i64(bytes, cycle_id.value());
            put_u8(bytes, *mode as u8);
        }
        ClientCommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution,
        } => {
            put_i64(bytes, postmortem_id.value());
            put_u8(bytes, *resolution as u8);
        }
    }
    Ok(())
}

fn decode_command_request(cursor: &mut Cursor<'_>) -> Result<ClientCommandRequest, WireError> {
    let command_id = parse_command_id(cursor.string(128)?)?;
    let principal_id = PrincipalId::try_from(cursor.i64()?).map_err(|_| WireError::InvalidValue)?;
    let capability_grant_id =
        CapabilityGrantId::try_from(cursor.i64()?).map_err(|_| WireError::InvalidValue)?;
    let capability = capability_from_u8(cursor.u8()?)?;
    let expected_generation = decode_expected_generation(cursor)?;
    let body_tag = cursor.u8()?;
    let body = decode_client_command_body(cursor, body_tag)?;
    Ok(ClientCommandRequest {
        command_id,
        principal_id,
        capability_grant_id,
        capability,
        expected_generation,
        body,
    })
}

fn decode_client_command_body(
    cursor: &mut Cursor<'_>,
    tag: u8,
) -> Result<ClientCommandBody, WireError> {
    match tag {
        1 => Ok(ClientCommandBody::CreateSocietyIdentity {
            name: SocietyName::parse(cursor.string(160)?).map_err(|_| WireError::InvalidValue)?,
        }),
        2 => Ok(ClientCommandBody::InstallRootAuthorityOffice),
        3 => Ok(ClientCommandBody::InstallFoundingMission {
            mission: Box::new(decode_application_mission_input(cursor)?),
            source_rendering: MissionSourceRendering::parse(
                cursor.bytes(MissionSourceRendering::MAX_BYTES)?,
            )
            .map_err(|_| WireError::InvalidValue)?,
        }),
        4 => Ok(ClientCommandBody::AppointInitialRootAuthority {
            actor_display_name: PrincipalDisplayName::parse(cursor.string(160)?)
                .map_err(|_| WireError::InvalidValue)?,
        }),
        5 => Ok(ClientCommandBody::SetR0HardCeiling {
            ceiling: UsdMicros::try_from(cursor.i64()?).map_err(|_| WireError::InvalidValue)?,
        }),
        6 => Ok(ClientCommandBody::BootstrapSociety),
        7 => Ok(ClientCommandBody::ProposeOperatingCycle {
            treatment: operating_cycle_treatment_from_u8(cursor.u8()?)?,
            budget_ceiling: UsdMicros::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
        }),
        8 => Ok(ClientCommandBody::AdmitOperatingCycle {
            cycle_id: OperatingCycleId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
        }),
        9 => Ok(ClientCommandBody::StartRootAuthorityOfficeSession {
            cycle_id: OperatingCycleId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
        }),
        11 => Ok(ClientCommandBody::OpenOfficeTurn {
            session_id: RootAuthorityOfficeSessionId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
            purpose: office_turn_purpose_from_u8(cursor.u8()?)?,
        }),
        13 => Ok(ClientCommandBody::QuiesceOperatingCycle {
            cycle_id: OperatingCycleId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
        }),
        15 => Ok(ClientCommandBody::ResumeOperatingCycle {
            cycle_id: OperatingCycleId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
        }),
        16 => Ok(ClientCommandBody::ReconcileOperatingCycle {
            cycle_id: OperatingCycleId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
        }),
        17 => Ok(ClientCommandBody::CloseOperatingCycle {
            cycle_id: OperatingCycleId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
        }),
        18 => Ok(ClientCommandBody::ReserveBudget {
            cycle_id: OperatingCycleId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
            amount: UsdMicros::try_from(cursor.i64()?).map_err(|_| WireError::InvalidValue)?,
        }),
        20 => Ok(ClientCommandBody::RequestCancellation {
            cycle_id: OperatingCycleId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
            mode: cancellation_mode_from_u8(cursor.u8()?)?,
        }),
        23 => Ok(ClientCommandBody::CloseCostPostmortem {
            postmortem_id: CostPostmortemId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
            resolution: cost_postmortem_resolution_from_u8(cursor.u8()?)?,
        }),
        _ => Err(WireError::UnknownTag),
    }
}

// The founding mission is a fixed, typed sequence rather than an opaque
// rendering: identity, revision, mission, ordered principles, North Star
// questions, then the source byte identity.
fn encode_application_mission_input(bytes: &mut Vec<u8>, mission: &ApplicationMissionInput) {
    put_string(bytes, mission.application_identity.as_str());
    put_string(bytes, mission.application_name.as_str());
    put_i64(bytes, mission.revision_ordinal.value());
    put_string(bytes, mission.statement.as_str());
    put_u8(bytes, mission.principles.as_slice().len() as u8);
    for principle in mission.principles.as_slice() {
        put_u8(bytes, principle.kind as u8);
        put_string(bytes, principle.text.as_str());
    }
    put_string(bytes, mission.north_star_questions.change.as_str());
    put_string(
        bytes,
        mission.north_star_questions.improvement_evidence.as_str(),
    );
    put_string(
        bytes,
        mission.north_star_questions.boundary_commitment.as_str(),
    );
    put_string(bytes, mission.north_star_questions.revisit.as_str());
    bytes.extend_from_slice(&mission.source_rendering_digest.as_bytes());
}

fn decode_application_mission_input(
    cursor: &mut Cursor<'_>,
) -> Result<ApplicationMissionInput, WireError> {
    let application_identity =
        ApplicationIdentity::parse(cursor.string(128)?).map_err(|_| WireError::InvalidValue)?;
    let application_name =
        ApplicationName::parse(cursor.string(160)?).map_err(|_| WireError::InvalidValue)?;
    let revision_ordinal =
        ApplicationRevisionOrdinal::try_from(cursor.i64()?).map_err(|_| WireError::InvalidValue)?;
    let statement =
        MissionStatement::parse(cursor.string(4_096)?).map_err(|_| WireError::InvalidValue)?;
    let principle_count = usize::from(cursor.u8()?);
    if principle_count == 0 || principle_count > MissionPrinciples::MAX_COUNT {
        return Err(WireError::InvalidValue);
    }
    let mut principles = Vec::with_capacity(principle_count);
    for _ in 0..principle_count {
        principles.push(MissionPrinciple {
            kind: mission_principle_kind_from_u8(cursor.u8()?)?,
            text: MissionPrincipleText::parse(cursor.string(4_096)?)
                .map_err(|_| WireError::InvalidValue)?,
        });
    }
    let north_star_questions = NorthStarQuestionSet {
        change: NorthStarChangeQuestion::parse(cursor.string(4_096)?)
            .map_err(|_| WireError::InvalidValue)?,
        improvement_evidence: NorthStarImprovementEvidenceQuestion::parse(cursor.string(4_096)?)
            .map_err(|_| WireError::InvalidValue)?,
        boundary_commitment: NorthStarBoundaryCommitmentQuestion::parse(cursor.string(4_096)?)
            .map_err(|_| WireError::InvalidValue)?,
        revisit: NorthStarRevisitQuestion::parse(cursor.string(4_096)?)
            .map_err(|_| WireError::InvalidValue)?,
    };
    Ok(ApplicationMissionInput {
        application_identity,
        application_name,
        revision_ordinal,
        statement,
        principles: MissionPrinciples::new(principles).map_err(|_| WireError::InvalidValue)?,
        north_star_questions,
        source_rendering_digest: Blake3Digest::from_bytes(cursor.array_32()?),
    })
}

fn encode_expected_generation(bytes: &mut Vec<u8>, expected_generation: ExpectedGeneration) {
    match expected_generation {
        ExpectedGeneration::NotApplicable => put_u8(bytes, 0),
        ExpectedGeneration::Exact(generation) => {
            put_u8(bytes, 1);
            put_i64(bytes, generation.value());
        }
    }
}

fn decode_expected_generation(cursor: &mut Cursor<'_>) -> Result<ExpectedGeneration, WireError> {
    match cursor.u8()? {
        0 => Ok(ExpectedGeneration::NotApplicable),
        1 => Ok(ExpectedGeneration::Exact(
            AdmissionGeneration::try_from(cursor.i64()?).map_err(|_| WireError::InvalidValue)?,
        )),
        _ => Err(WireError::InvalidValue),
    }
}

fn encode_receipt(bytes: &mut Vec<u8>, receipt: CommandReceiptView) {
    match receipt {
        CommandReceiptView::Accepted {
            event_id,
            idempotent,
        } => {
            put_u8(bytes, 1);
            put_i64(bytes, event_id.value());
            put_bool(bytes, idempotent);
        }
        CommandReceiptView::Rejected {
            rejection,
            idempotent,
        } => {
            put_u8(bytes, 2);
            put_u8(bytes, rejection.as_u8());
            put_bool(bytes, idempotent);
        }
    }
}

fn decode_receipt(cursor: &mut Cursor<'_>) -> Result<CommandReceiptView, WireError> {
    match cursor.u8()? {
        1 => Ok(CommandReceiptView::Accepted {
            event_id: society_kernel::EventId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
            idempotent: cursor.bool()?,
        }),
        2 => Ok(CommandReceiptView::Rejected {
            rejection: rejection_from_u8(cursor.u8()?)?,
            idempotent: cursor.bool()?,
        }),
        _ => Err(WireError::InvalidValue),
    }
}

// Study observations are deliberately encoded as their fixed normalized
// projection rather than as a database-shaped row set or an application JSON
// document. This gives analysis clients the exact durable facts needed to
// qualify a paired result while leaving application measurement semantics in
// the sealed revision that names each slot.
fn encode_study_pair_observation(
    bytes: &mut Vec<u8>,
    pair: &StudyPairObservation,
) -> Result<(), WireError> {
    put_i64(bytes, pair.pair_id.value());
    encode_study_episode_observation(bytes, &pair.retained)?;
    encode_study_episode_observation(bytes, &pair.reset)
}

fn decode_study_pair_observation(
    cursor: &mut Cursor<'_>,
) -> Result<StudyPairObservation, WireError> {
    Ok(StudyPairObservation {
        pair_id: study_pair_id(cursor.i64()?)?,
        retained: decode_study_episode_observation(cursor)?,
        reset: decode_study_episode_observation(cursor)?,
    })
}

fn encode_study_episode_observation(
    bytes: &mut Vec<u8>,
    episode: &StudyEpisodeObservation,
) -> Result<(), WireError> {
    put_i64(bytes, episode.episode_id.value());
    put_i64(bytes, episode.protocol_revision_id.value());
    put_i64(bytes, episode.world_revision_id.value());
    put_i64(bytes, episode.measurement_revision_id.value());
    put_u8(bytes, episode.measurement_slot_count.value());
    put_i64(bytes, episode.institution_revision_id.value());
    put_i64(bytes, episode.source_population_snapshot_id.value());
    put_optional_i64(
        bytes,
        episode
            .successor_population_snapshot_id
            .map(StudyPopulationSnapshotId::value),
    );
    put_digest(bytes, episode.randomization_digest);
    put_u8(bytes, episode.treatment as u8);
    put_u8(bytes, episode.lifecycle_state as u8);
    put_i64(bytes, episode.source_actor_obligations);
    put_i64(bytes, episode.source_terminal_actor_obligations);
    put_i64(bytes, episode.successor_actor_obligations);
    put_i64(bytes, episode.successor_terminal_actor_obligations);
    put_i64(bytes, episode.failed_actor_obligations);
    put_i64(bytes, episode.runtime_bindings);
    put_i64(bytes, episode.reconciled_runtime_bindings);
    put_optional_i64(bytes, episode.frozen_forum_head);
    put_i64(bytes, episode.forum_messages);
    put_i64(bytes, episode.forum_reads);
    put_i64(bytes, episode.forum_returned_bytes);
    put_i64(bytes, episode.decisions);
    put_optional_digest(bytes, episode.ground_truth_reveal_digest);
    put_u8(
        bytes,
        u8::try_from(episode.measurements.len()).map_err(|_| WireError::InvalidValue)?,
    );
    for measurement in &episode.measurements {
        encode_study_measurement_observation(bytes, measurement);
    }
    Ok(())
}

fn decode_study_episode_observation(
    cursor: &mut Cursor<'_>,
) -> Result<StudyEpisodeObservation, WireError> {
    let episode = StudyEpisodeObservation {
        episode_id: study_episode_id(cursor.i64()?)?,
        protocol_revision_id: StudyProtocolRevisionId::try_from(cursor.i64()?)
            .map_err(|_| WireError::InvalidValue)?,
        world_revision_id: StudyWorldRevisionId::try_from(cursor.i64()?)
            .map_err(|_| WireError::InvalidValue)?,
        measurement_revision_id: StudyMeasurementRevisionId::try_from(cursor.i64()?)
            .map_err(|_| WireError::InvalidValue)?,
        measurement_slot_count: StudyMeasurementSlotCount::try_from(i64::from(cursor.u8()?))
            .map_err(|_| WireError::InvalidValue)?,
        institution_revision_id: StudyInstitutionRevisionId::try_from(cursor.i64()?)
            .map_err(|_| WireError::InvalidValue)?,
        source_population_snapshot_id: StudyPopulationSnapshotId::try_from(cursor.i64()?)
            .map_err(|_| WireError::InvalidValue)?,
        successor_population_snapshot_id: cursor
            .optional_i64()?
            .map(StudyPopulationSnapshotId::try_from)
            .transpose()
            .map_err(|_| WireError::InvalidValue)?,
        randomization_digest: cursor.digest()?,
        treatment: study_treatment(cursor.u8()?)?,
        lifecycle_state: study_episode_state(cursor.u8()?)?,
        source_actor_obligations: cursor.i64()?,
        source_terminal_actor_obligations: cursor.i64()?,
        successor_actor_obligations: cursor.i64()?,
        successor_terminal_actor_obligations: cursor.i64()?,
        failed_actor_obligations: cursor.i64()?,
        runtime_bindings: cursor.i64()?,
        reconciled_runtime_bindings: cursor.i64()?,
        frozen_forum_head: cursor.optional_i64()?,
        forum_messages: cursor.i64()?,
        forum_reads: cursor.i64()?,
        forum_returned_bytes: cursor.i64()?,
        decisions: cursor.i64()?,
        ground_truth_reveal_digest: cursor.optional_digest()?,
        measurements: Vec::new(),
    };
    let measurement_count = cursor.u8()?;
    if measurement_count > episode.measurement_slot_count.value() {
        return Err(WireError::InvalidValue);
    }
    let mut measurements = Vec::with_capacity(usize::from(measurement_count));
    for _ in 0..measurement_count {
        measurements.push(decode_study_measurement_observation(cursor)?);
    }
    Ok(StudyEpisodeObservation {
        measurements,
        ..episode
    })
}

fn encode_study_measurement_observation(
    bytes: &mut Vec<u8>,
    measurement: &StudyMeasurementObservation,
) {
    put_u8(bytes, measurement.measurement_slot.value());
    put_u8(bytes, measurement.status as u8);
    put_optional_i64(bytes, measurement.value);
    put_optional_digest(bytes, measurement.value_digest);
    put_optional_digest(bytes, measurement.reason_digest);
}

fn decode_study_measurement_observation(
    cursor: &mut Cursor<'_>,
) -> Result<StudyMeasurementObservation, WireError> {
    Ok(StudyMeasurementObservation {
        measurement_slot: StudyMeasurementSlot::try_from(i64::from(cursor.u8()?))
            .map_err(|_| WireError::InvalidValue)?,
        status: study_measurement_status(cursor.u8()?)?,
        value: cursor.optional_i64()?,
        value_digest: cursor.optional_digest()?,
        reason_digest: cursor.optional_digest()?,
    })
}

fn encode_study_run_summary(bytes: &mut Vec<u8>, study_run: &StudyRunSummary) {
    put_i64(bytes, study_run.study_run_id.value());
    put_i64(bytes, study_run.protocol_revision_id.value());
    put_i64(bytes, study_run.plan_content_object_id.value());
    put_digest(bytes, study_run.plan_digest);
    put_u16(bytes, study_run.pair_count.value());
    put_u16(bytes, study_run.registered_pair_count.value());
    put_u8(bytes, study_run.lifecycle_state as u8);
}

fn decode_study_run_summary(cursor: &mut Cursor<'_>) -> Result<StudyRunSummary, WireError> {
    let study_run_id = StudyRunId::try_from(cursor.i64()?).map_err(|_| WireError::InvalidValue)?;
    let protocol_revision_id =
        StudyProtocolRevisionId::try_from(cursor.i64()?).map_err(|_| WireError::InvalidValue)?;
    let plan_content_object_id = society_kernel::ContentObjectId::try_from(cursor.i64()?)
        .map_err(|_| WireError::InvalidValue)?;
    let plan_digest = cursor.digest()?;
    let pair_count = StudyRunPairCount::try_from(i64::from(cursor.u16()?))
        .map_err(|_| WireError::InvalidValue)?;
    let registered_pair_count = StudyRunRegisteredPairCount::try_from(i64::from(cursor.u16()?))
        .map_err(|_| WireError::InvalidValue)?;
    let lifecycle_state = study_run_lifecycle_state(cursor.u8()?)?;
    Ok(StudyRunSummary {
        study_run_id,
        protocol_revision_id,
        plan_content_object_id,
        plan_digest,
        pair_count,
        registered_pair_count,
        lifecycle_state,
    })
}

fn encode_study_run_pair_registration(
    bytes: &mut Vec<u8>,
    registration: StudyRunPairRegistrationObservation,
) {
    put_u16(bytes, registration.pair_ordinal.value());
    put_i64(bytes, registration.pair_id.value());
    put_digest(bytes, registration.randomization_digest);
}

fn decode_study_run_pair_registration(
    cursor: &mut Cursor<'_>,
) -> Result<StudyRunPairRegistrationObservation, WireError> {
    Ok(StudyRunPairRegistrationObservation {
        pair_ordinal: StudyRunPairOrdinal::try_from(i64::from(cursor.u16()?))
            .map_err(|_| WireError::InvalidValue)?,
        pair_id: study_pair_id(cursor.i64()?)?,
        randomization_digest: cursor.digest()?,
    })
}

fn study_pair_id(value: i64) -> Result<StudyPairId, WireError> {
    StudyPairId::try_from(value).map_err(|_| WireError::InvalidValue)
}

fn study_episode_id(value: i64) -> Result<StudyEpisodeId, WireError> {
    StudyEpisodeId::try_from(value).map_err(|_| WireError::InvalidValue)
}

fn study_treatment(value: u8) -> Result<StudyTreatment, WireError> {
    match value {
        1 => Ok(StudyTreatment::Retained),
        2 => Ok(StudyTreatment::Reset),
        _ => Err(WireError::InvalidValue),
    }
}

fn study_episode_state(value: u8) -> Result<StudyEpisodeState, WireError> {
    match value {
        1 => Ok(StudyEpisodeState::Admitted),
        2 => Ok(StudyEpisodeState::SourceActive),
        3 => Ok(StudyEpisodeState::SourceReconciled),
        4 => Ok(StudyEpisodeState::SuccessorAdmitted),
        5 => Ok(StudyEpisodeState::CorrectionReleased),
        6 => Ok(StudyEpisodeState::SuccessorActive),
        7 => Ok(StudyEpisodeState::Closed),
        _ => Err(WireError::InvalidValue),
    }
}

fn study_measurement_status(value: u8) -> Result<StudyMeasurementStatus, WireError> {
    match value {
        1 => Ok(StudyMeasurementStatus::Observed),
        2 => Ok(StudyMeasurementStatus::Unavailable),
        3 => Ok(StudyMeasurementStatus::Invalidated),
        _ => Err(WireError::InvalidValue),
    }
}

fn study_run_lifecycle_state(value: u8) -> Result<StudyRunLifecycleState, WireError> {
    match value {
        1 => Ok(StudyRunLifecycleState::Pairing),
        2 => Ok(StudyRunLifecycleState::Ready),
        3 => Ok(StudyRunLifecycleState::Running),
        4 => Ok(StudyRunLifecycleState::Completed),
        _ => Err(WireError::InvalidValue),
    }
}

fn put_correlation(bytes: &mut Vec<u8>, correlation: CorrelationId) {
    bytes.extend_from_slice(&correlation.value().to_be_bytes());
}

fn put_string(bytes: &mut Vec<u8>, value: &str) {
    let length = u32::try_from(value.len()).expect("domain strings fit u32");
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn put_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    let length = u32::try_from(value.len()).expect("bounded domain bytes fit u32");
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value);
}

fn put_u8(bytes: &mut Vec<u8>, value: u8) {
    bytes.push(value);
}

fn put_bool(bytes: &mut Vec<u8>, value: bool) {
    put_u8(bytes, u8::from(value));
}

fn put_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn put_i64(bytes: &mut Vec<u8>, value: i64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn put_optional_i64(bytes: &mut Vec<u8>, value: Option<i64>) {
    match value {
        Some(value) => {
            put_bool(bytes, true);
            put_i64(bytes, value);
        }
        None => put_bool(bytes, false),
    }
}

fn put_digest(bytes: &mut Vec<u8>, digest: Blake3Digest) {
    bytes.extend_from_slice(&digest.as_bytes());
}

fn put_optional_digest(bytes: &mut Vec<u8>, digest: Option<Blake3Digest>) {
    match digest {
        Some(digest) => {
            put_bool(bytes, true);
            put_digest(bytes, digest);
        }
        None => put_bool(bytes, false),
    }
}

struct Cursor<'a> {
    remaining: &'a [u8],
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], WireError> {
        if self.remaining.len() < length {
            return Err(WireError::MissingField);
        }
        let (field, remaining) = self.remaining.split_at(length);
        self.remaining = remaining;
        Ok(field)
    }

    fn u8(&mut self) -> Result<u8, WireError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, WireError> {
        let bytes: [u8; 2] = self
            .take(2)?
            .try_into()
            .map_err(|_| WireError::MissingField)?;
        Ok(u16::from_be_bytes(bytes))
    }

    fn i64(&mut self) -> Result<i64, WireError> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| WireError::MissingField)?;
        Ok(i64::from_be_bytes(bytes))
    }

    fn correlation(&mut self) -> Result<CorrelationId, WireError> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| WireError::MissingField)?;
        CorrelationId::new(u64::from_be_bytes(bytes)).ok_or(WireError::InvalidCorrelation)
    }

    fn bool(&mut self) -> Result<bool, WireError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(WireError::InvalidBoolean),
        }
    }

    fn array_32(&mut self) -> Result<[u8; 32], WireError> {
        self.take(32)?
            .try_into()
            .map_err(|_| WireError::MissingField)
    }

    fn digest(&mut self) -> Result<Blake3Digest, WireError> {
        Ok(Blake3Digest::from_bytes(self.array_32()?))
    }

    fn optional_i64(&mut self) -> Result<Option<i64>, WireError> {
        match self.bool()? {
            true => Ok(Some(self.i64()?)),
            false => Ok(None),
        }
    }

    fn optional_digest(&mut self) -> Result<Option<Blake3Digest>, WireError> {
        match self.bool()? {
            true => Ok(Some(self.digest()?)),
            false => Ok(None),
        }
    }

    fn string(&mut self, maximum: usize) -> Result<String, WireError> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| WireError::MissingField)?;
        let length = u32::from_be_bytes(bytes) as usize;
        if length > maximum {
            return Err(WireError::StringTooLong);
        }
        String::from_utf8(self.take(length)?.to_vec()).map_err(|_| WireError::InvalidUtf8)
    }

    fn bytes(&mut self, maximum: usize) -> Result<Vec<u8>, WireError> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| WireError::MissingField)?;
        let length = u32::from_be_bytes(bytes) as usize;
        if length > maximum {
            return Err(WireError::BytesTooLong);
        }
        Ok(self.take(length)?.to_vec())
    }

    fn finish(self) -> Result<(), WireError> {
        if self.remaining.is_empty() {
            Ok(())
        } else {
            Err(WireError::TrailingBytes)
        }
    }
}

fn parse_command_id(value: String) -> Result<CommandId, WireError> {
    CommandId::parse(value).map_err(|_| WireError::InvalidValue)
}

fn capability_from_u8(value: u8) -> Result<Capability, WireError> {
    match value {
        1 => Ok(Capability::CreateSocietyIdentity),
        2 => Ok(Capability::InstallRootAuthorityOffice),
        3 => Ok(Capability::InstallFoundingMission),
        4 => Ok(Capability::AppointInitialRootAuthority),
        5 => Ok(Capability::SetR0HardCeiling),
        6 => Ok(Capability::BootstrapSociety),
        7 => Ok(Capability::ProposeOperatingCycle),
        8 => Ok(Capability::AdmitOperatingCycle),
        9 => Ok(Capability::QuiesceOperatingCycle),
        10 => Ok(Capability::ResumeOperatingCycle),
        11 => Ok(Capability::ReconcileOperatingCycle),
        12 => Ok(Capability::CloseOperatingCycle),
        13 => Ok(Capability::StartRootAuthorityOfficeSession),
        14 => Ok(Capability::OpenOfficeTurn),
        15 => Ok(Capability::RequestCancellation),
        16 => Ok(Capability::ReserveBudget),
        17 => Ok(Capability::ReconcileBudget),
        18 => Ok(Capability::RecordCycleDrained),
        19 => Ok(Capability::RecordOfficeSessionReady),
        20 => Ok(Capability::SettleOfficeTurn),
        21 => Ok(Capability::ReconcileCancellation),
        22 => Ok(Capability::RecordOfficeSessionTerminal),
        23 => Ok(Capability::CloseCostPostmortem),
        _ => Err(WireError::InvalidValue),
    }
}

fn supervisor_capability_is_representable(capability: Capability) -> bool {
    matches!(
        capability,
        Capability::CreateSocietyIdentity
            | Capability::InstallRootAuthorityOffice
            | Capability::InstallFoundingMission
            | Capability::AppointInitialRootAuthority
            | Capability::SetR0HardCeiling
            | Capability::BootstrapSociety
            | Capability::ProposeOperatingCycle
            | Capability::AdmitOperatingCycle
            | Capability::QuiesceOperatingCycle
            | Capability::ResumeOperatingCycle
            | Capability::ReconcileOperatingCycle
            | Capability::CloseOperatingCycle
            | Capability::StartRootAuthorityOfficeSession
            | Capability::OpenOfficeTurn
            | Capability::RequestCancellation
            | Capability::ReserveBudget
            | Capability::CloseCostPostmortem
    )
}

fn operating_cycle_treatment_from_u8(value: u8) -> Result<OperatingCycleTreatment, WireError> {
    match value {
        1 => Ok(OperatingCycleTreatment::PiSdkQualificationV1),
        2 => Ok(OperatingCycleTreatment::PinnedPiSdkLiveV1),
        3 => Ok(OperatingCycleTreatment::DeterministicPiHostFixtureV1),
        _ => Err(WireError::InvalidValue),
    }
}

fn mission_principle_kind_from_u8(value: u8) -> Result<MissionPrincipleKind, WireError> {
    match value {
        1 => Ok(MissionPrincipleKind::Purpose),
        2 => Ok(MissionPrincipleKind::Evidence),
        3 => Ok(MissionPrincipleKind::Boundary),
        4 => Ok(MissionPrincipleKind::Revision),
        _ => Err(WireError::InvalidValue),
    }
}

fn office_turn_purpose_from_u8(value: u8) -> Result<OfficeTurnPurpose, WireError> {
    match value {
        1 => Ok(OfficeTurnPurpose::OrdinaryWork),
        2 => Ok(OfficeTurnPurpose::Recovery),
        3 => Ok(OfficeTurnPurpose::Cancellation),
        4 => Ok(OfficeTurnPurpose::Closure),
        _ => Err(WireError::InvalidValue),
    }
}

fn cancellation_mode_from_u8(value: u8) -> Result<CancellationMode, WireError> {
    match value {
        1 => Ok(CancellationMode::Quiesce),
        2 => Ok(CancellationMode::GracefulCancel),
        3 => Ok(CancellationMode::EmergencyStop),
        _ => Err(WireError::InvalidValue),
    }
}

fn cost_postmortem_resolution_from_u8(value: u8) -> Result<CostPostmortemResolution, WireError> {
    match value {
        1 => Ok(CostPostmortemResolution::ConservativeFullReservation),
        2 => Ok(CostPostmortemResolution::ChargeObservedOverrun),
        _ => Err(WireError::InvalidValue),
    }
}

fn rejection_from_u8(value: u8) -> Result<Rejection, WireError> {
    Rejection::try_from(value).map_err(|_| WireError::InvalidValue)
}

fn protocol_error_from_u8(value: u8) -> Result<ProtocolErrorCode, WireError> {
    match value {
        1 => Ok(ProtocolErrorCode::MalformedFrame),
        2 => Ok(ProtocolErrorCode::UnsupportedVersion),
        3 => Ok(ProtocolErrorCode::UnknownTag),
        4 => Ok(ProtocolErrorCode::PeerNotAuthorized),
        5 => Ok(ProtocolErrorCode::RecoveryFenced),
        6 => Ok(ProtocolErrorCode::IdempotencyConflict),
        7 => Ok(ProtocolErrorCode::KernelFailure),
        8 => Ok(ProtocolErrorCode::DaemonStopping),
        9 => Ok(ProtocolErrorCode::MissionSourceDigestMismatch),
        _ => Err(WireError::InvalidValue),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_application_mission() -> ApplicationMissionInput {
        ApplicationMissionInput {
            application_identity: ApplicationIdentity::parse("wire-mission-fixture")
                .expect("the fixed application identity is valid"),
            application_name: ApplicationName::parse("Wire mission fixture")
                .expect("the fixed application name is valid"),
            revision_ordinal: ApplicationRevisionOrdinal::new(7)
                .expect("the fixed positive revision ordinal is valid"),
            statement: MissionStatement::parse("Keep the resident protocol bounded and exact.")
                .expect("the fixed mission statement is valid"),
            principles: MissionPrinciples::new(vec![
                MissionPrinciple {
                    kind: MissionPrincipleKind::Purpose,
                    text: MissionPrincipleText::parse("Preserve a legible mission boundary.")
                        .expect("the fixed principle text is valid"),
                },
                MissionPrinciple {
                    kind: MissionPrincipleKind::Evidence,
                    text: MissionPrincipleText::parse("Retain exact evidence for each change.")
                        .expect("the fixed principle text is valid"),
                },
            ])
            .expect("the fixed principles are bounded and nonempty"),
            north_star_questions: NorthStarQuestionSet {
                change: NorthStarChangeQuestion::parse("What bounded change is needed?")
                    .expect("the fixed change question is valid"),
                improvement_evidence: NorthStarImprovementEvidenceQuestion::parse(
                    "What evidence proves the improvement?",
                )
                .expect("the fixed evidence question is valid"),
                boundary_commitment: NorthStarBoundaryCommitmentQuestion::parse(
                    "Which authority boundary must remain intact?",
                )
                .expect("the fixed boundary question is valid"),
                revisit: NorthStarRevisitQuestion::parse("When should this mission be revisited?")
                    .expect("the fixed revisit question is valid"),
            },
            source_rendering_digest: Blake3Digest::of_bytes(b"wire-mission-fixture-revision-7"),
        }
    }

    fn mission_request(mission: ApplicationMissionInput) -> SupervisorRequest {
        SupervisorRequest::Execute {
            correlation: CorrelationId::new(1).expect("the fixed nonzero correlation is valid"),
            command: ClientCommandRequest {
                command_id: CommandId::parse("wire-mission-001")
                    .expect("the fixed command identity is valid"),
                principal_id: PrincipalId::new(3).expect("the fixed nonzero principal is valid"),
                capability_grant_id: CapabilityGrantId::new(1)
                    .expect("the fixed nonzero capability grant is valid"),
                capability: Capability::InstallFoundingMission,
                expected_generation: ExpectedGeneration::NotApplicable,
                body: ClientCommandBody::InstallFoundingMission {
                    mission: Box::new(mission),
                    source_rendering: MissionSourceRendering::parse(
                        b"wire-mission-fixture-revision-7".to_vec(),
                    )
                    .expect("the fixed source rendering is valid"),
                },
            },
        }
    }

    fn raw_mission_request_payload(
        application_identity: &str,
        revision_ordinal: i64,
        statement: &str,
        principle_kinds: &[u8],
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        put_u16(&mut payload, PROTOCOL_VERSION);
        put_u8(&mut payload, SUPERVISOR_EXECUTE_TAG);
        put_correlation(
            &mut payload,
            CorrelationId::new(1).expect("the fixed nonzero correlation is valid"),
        );
        put_string(&mut payload, "wire-mission-001");
        put_i64(&mut payload, 3);
        put_i64(&mut payload, 1);
        put_u8(&mut payload, Capability::InstallFoundingMission as u8);
        put_u8(&mut payload, 0);
        put_u8(&mut payload, 3);
        put_string(&mut payload, application_identity);
        put_string(&mut payload, "Wire mission fixture");
        put_i64(&mut payload, revision_ordinal);
        put_string(&mut payload, statement);
        put_u8(&mut payload, principle_kinds.len() as u8);
        for kind in principle_kinds {
            put_u8(&mut payload, *kind);
            put_string(&mut payload, "A bounded mission principle.");
        }
        put_string(&mut payload, "What bounded change is needed?");
        put_string(&mut payload, "What evidence proves the improvement?");
        put_string(&mut payload, "Which authority boundary must remain intact?");
        put_string(&mut payload, "When should this mission be revisited?");
        payload.extend_from_slice(&[0x5A; 32]);
        put_bytes(&mut payload, b"wire source rendering");
        payload
    }

    fn framed(payload: &[u8]) -> Vec<u8> {
        let mut framed = Vec::new();
        write_frame(&mut framed, payload).expect("the bounded test frame must encode");
        framed
    }

    fn sample_episode(episode_id: i64, treatment: StudyTreatment) -> StudyEpisodeObservation {
        StudyEpisodeObservation {
            episode_id: StudyEpisodeId::new(episode_id)
                .expect("the fixed positive episode identity is valid"),
            protocol_revision_id: StudyProtocolRevisionId::new(11)
                .expect("the fixed positive protocol identity is valid"),
            world_revision_id: StudyWorldRevisionId::new(12)
                .expect("the fixed positive world identity is valid"),
            measurement_revision_id: StudyMeasurementRevisionId::new(13)
                .expect("the fixed positive measurement identity is valid"),
            measurement_slot_count: StudyMeasurementSlotCount::new(2)
                .expect("the fixed measurement slot count is valid"),
            institution_revision_id: StudyInstitutionRevisionId::new(14)
                .expect("the fixed positive institution identity is valid"),
            source_population_snapshot_id: StudyPopulationSnapshotId::new(15)
                .expect("the fixed positive source population identity is valid"),
            successor_population_snapshot_id: Some(
                StudyPopulationSnapshotId::new(16)
                    .expect("the fixed positive successor population identity is valid"),
            ),
            randomization_digest: Blake3Digest::of_bytes(b"wire-randomization"),
            treatment,
            lifecycle_state: StudyEpisodeState::Closed,
            source_actor_obligations: 8,
            source_terminal_actor_obligations: 8,
            successor_actor_obligations: 8,
            successor_terminal_actor_obligations: 8,
            failed_actor_obligations: 0,
            runtime_bindings: 16,
            reconciled_runtime_bindings: 16,
            frozen_forum_head: Some(17),
            forum_messages: 16,
            forum_reads: 32,
            forum_returned_bytes: 9_000,
            decisions: 16,
            ground_truth_reveal_digest: Some(Blake3Digest::of_bytes(b"wire-ground-truth")),
            measurements: vec![
                StudyMeasurementObservation {
                    measurement_slot: StudyMeasurementSlot::new(1)
                        .expect("the fixed first slot is valid"),
                    status: StudyMeasurementStatus::Observed,
                    value: Some(42),
                    value_digest: Some(Blake3Digest::of_bytes(b"wire-observed-value")),
                    reason_digest: None,
                },
                StudyMeasurementObservation {
                    measurement_slot: StudyMeasurementSlot::new(2)
                        .expect("the fixed second slot is valid"),
                    status: StudyMeasurementStatus::Unavailable,
                    value: None,
                    value_digest: None,
                    reason_digest: Some(Blake3Digest::of_bytes(b"wire-unavailable-reason")),
                },
            ],
        }
    }

    #[test]
    fn public_study_observation_queries_round_trip_as_closed_normalized_values() {
        let correlation = CorrelationId::new(73).expect("the fixed correlation is valid");
        for request in [
            PublicRequest::StudyPairObservation {
                correlation,
                pair_id: StudyPairId::new(31).expect("the fixed pair identity is valid"),
            },
            PublicRequest::StudyRunObservation {
                correlation,
                study_run_id: StudyRunId::new(32).expect("the fixed run identity is valid"),
            },
            PublicRequest::StudyRunPairRegistration {
                correlation,
                study_run_id: StudyRunId::new(32).expect("the fixed run identity is valid"),
                pair_ordinal: StudyRunPairOrdinal::new(1).expect("the fixed pair ordinal is valid"),
            },
        ] {
            let mut encoded = Vec::new();
            write_public_request(&mut encoded, &request)
                .expect("the closed study query must encode");
            assert_eq!(
                read_public_request(&mut encoded.as_slice())
                    .expect("the closed study query must decode"),
                request
            );
        }

        let pair = StudyPairObservation {
            pair_id: StudyPairId::new(31).expect("the fixed pair identity is valid"),
            retained: sample_episode(41, StudyTreatment::Retained),
            reset: sample_episode(42, StudyTreatment::Reset),
        };
        let study_run = StudyRunObservation {
            study_run_id: StudyRunId::new(32).expect("the fixed run identity is valid"),
            protocol_revision_id: StudyProtocolRevisionId::new(11)
                .expect("the fixed protocol identity is valid"),
            plan_content_object_id: society_kernel::ContentObjectId::new(33)
                .expect("the fixed plan object identity is valid"),
            plan_digest: Blake3Digest::of_bytes(b"wire-plan"),
            pair_count: StudyRunPairCount::new(1).expect("the fixed pair count is valid"),
            registered_pair_count: StudyRunRegisteredPairCount::new(1)
                .expect("the fixed registered count is valid"),
            lifecycle_state: StudyRunLifecycleState::Completed,
            pairs: vec![StudyRunPairRegistrationObservation {
                pair_ordinal: StudyRunPairOrdinal::new(1).expect("the fixed pair ordinal is valid"),
                pair_id: pair.pair_id,
                randomization_digest: Blake3Digest::of_bytes(b"wire-randomization"),
            }],
        };
        for response in [
            Response::StudyPairObservation {
                correlation,
                pair: Some(pair),
            },
            Response::StudyRunSummary {
                correlation,
                study_run: Some(StudyRunSummary::from(&study_run)),
            },
            Response::StudyRunPairRegistration {
                correlation,
                registration: Some(study_run.pairs[0]),
            },
        ] {
            let mut encoded = Vec::new();
            write_response(&mut encoded, &response)
                .expect("the closed study observation must encode");
            assert_eq!(
                read_response(&mut encoded.as_slice())
                    .expect("the closed study observation must decode"),
                response
            );
        }

        let maximum_run_summary = StudyRunSummary {
            pair_count: StudyRunPairCount::new(10_000)
                .expect("the generic maximum pair count is valid"),
            registered_pair_count: StudyRunRegisteredPairCount::new(10_000)
                .expect("the generic maximum registered count is valid"),
            ..StudyRunSummary::from(&study_run)
        };
        let response = Response::StudyRunSummary {
            correlation,
            study_run: Some(maximum_run_summary),
        };
        let mut encoded = Vec::new();
        write_response(&mut encoded, &response)
            .expect("a maximum-pair run summary must remain within one frame");
        assert!(encoded.len() < MAX_FRAME_BYTES);
        assert_eq!(
            read_response(&mut encoded.as_slice()).expect("the maximum-pair summary must decode"),
            response
        );
    }

    #[test]
    fn every_kernel_rejection_round_trips_through_the_receipt_wire() {
        assert_eq!(Rejection::ProjectNorthStarAlignmentMismatch.as_u8(), 58);
        assert!(Rejection::ALL.contains(&Rejection::ProjectNorthStarAlignmentMismatch));
        for rejection in Rejection::ALL {
            let response = Response::CommandReceipt {
                correlation: CorrelationId::new(1).expect("the fixed nonzero correlation is valid"),
                receipt: CommandReceiptView::Rejected {
                    rejection: *rejection,
                    idempotent: false,
                },
            };
            let mut encoded = Vec::new();
            write_response(&mut encoded, &response)
                .expect("the closed rejection receipt must encode");
            assert_eq!(
                read_response(&mut encoded.as_slice())
                    .expect("the closed rejection receipt must decode"),
                response
            );
        }
    }

    #[test]
    fn mission_source_digest_mismatch_round_trips_as_a_closed_protocol_error() {
        let response = Response::Error {
            correlation: CorrelationId::new(1).expect("the fixed nonzero correlation is valid"),
            code: ProtocolErrorCode::MissionSourceDigestMismatch,
        };
        let mut encoded = Vec::new();
        write_response(&mut encoded, &response).expect("the closed error must encode");
        assert_eq!(
            read_response(&mut encoded.as_slice()).expect("the closed error must decode"),
            response
        );
    }

    #[test]
    fn supervisor_founding_mission_round_trips_as_one_exact_typed_input() {
        let request = mission_request(example_application_mission());
        let mut encoded = Vec::new();
        write_supervisor_request(&mut encoded, &request)
            .expect("the closed supervisor request must encode");
        assert_eq!(
            read_supervisor_request(&mut encoded.as_slice())
                .expect("the closed supervisor request must decode"),
            request
        );
    }

    #[test]
    fn supervisor_founding_mission_admits_the_exact_rendering_byte_bound() {
        let request = SupervisorRequest::Execute {
            correlation: CorrelationId::new(2).expect("the fixed nonzero correlation is valid"),
            command: ClientCommandRequest {
                command_id: CommandId::parse("wire-mission-rendering-max")
                    .expect("the fixed command identity is valid"),
                principal_id: PrincipalId::new(3).expect("the fixed nonzero principal is valid"),
                capability_grant_id: CapabilityGrantId::new(1)
                    .expect("the fixed nonzero capability grant is valid"),
                capability: Capability::InstallFoundingMission,
                expected_generation: ExpectedGeneration::NotApplicable,
                body: ClientCommandBody::InstallFoundingMission {
                    mission: Box::new(example_application_mission()),
                    source_rendering: MissionSourceRendering::parse(vec![
                        0xA5;
                        MissionSourceRendering::MAX_BYTES
                    ])
                    .expect("the exact source rendering bound is valid"),
                },
            },
        };
        let mut encoded = Vec::new();
        write_supervisor_request(&mut encoded, &request)
            .expect("the exact rendering bound fits one supervisor frame");
        assert_eq!(
            read_supervisor_request(&mut encoded.as_slice())
                .expect("the exact rendering bound must decode"),
            request
        );
    }

    #[test]
    fn supervisor_founding_mission_rejects_malformed_principles_and_fields_at_the_wire() {
        let invalid_value_payloads = [
            raw_mission_request_payload("wire-mission-fixture", 7, "A bounded mission.", &[9]),
            raw_mission_request_payload("wire-mission-fixture", 7, "A bounded mission.", &[]),
            raw_mission_request_payload(
                "wire-mission-fixture",
                7,
                "A bounded mission.",
                &[1; MissionPrinciples::MAX_COUNT + 1],
            ),
            raw_mission_request_payload("", 7, "A bounded mission.", &[1]),
            raw_mission_request_payload("wire-mission-fixture", 0, "A bounded mission.", &[1]),
            raw_mission_request_payload("wire-mission-fixture", 7, "", &[1]),
        ];
        for payload in invalid_value_payloads {
            let encoded = framed(&payload);
            assert!(matches!(
                read_supervisor_request(&mut encoded.as_slice()),
                Err(WireError::InvalidValue)
            ));
        }

        let valid_payload =
            raw_mission_request_payload("wire-mission-fixture", 7, "A bounded mission.", &[1]);
        let mut truncated_payload = valid_payload.clone();
        truncated_payload.pop();
        let truncated = framed(&truncated_payload);
        assert!(matches!(
            read_supervisor_request(&mut truncated.as_slice()),
            Err(WireError::MissingField)
        ));

        let mut trailing_payload = valid_payload;
        trailing_payload.push(0);
        let trailing = framed(&trailing_payload);
        assert!(matches!(
            read_supervisor_request(&mut trailing.as_slice()),
            Err(WireError::TrailingBytes)
        ));

        let mut empty_rendering =
            raw_mission_request_payload("wire-mission-fixture", 7, "A bounded mission.", &[1]);
        let rendering_length_offset = empty_rendering.len() - b"wire source rendering".len() - 4;
        empty_rendering.splice(rendering_length_offset.., [0, 0, 0, 0]);
        let encoded = framed(&empty_rendering);
        assert!(matches!(
            read_supervisor_request(&mut encoded.as_slice()),
            Err(WireError::InvalidValue)
        ));

        let mut oversized_rendering =
            raw_mission_request_payload("wire-mission-fixture", 7, "A bounded mission.", &[1]);
        let rendering_length_offset =
            oversized_rendering.len() - b"wire source rendering".len() - 4;
        oversized_rendering.splice(
            rendering_length_offset..,
            u32::try_from(MissionSourceRendering::MAX_BYTES + 1)
                .expect("the closed byte bound fits the wire length")
                .to_be_bytes(),
        );
        let encoded = framed(&oversized_rendering);
        assert!(matches!(
            read_supervisor_request(&mut encoded.as_slice()),
            Err(WireError::BytesTooLong)
        ));
    }

    #[test]
    fn digest_only_v5_founding_mission_frame_is_rejected_by_version_before_decoding() {
        let mut historical =
            raw_mission_request_payload("wire-mission-fixture", 7, "A bounded mission.", &[1]);
        historical[0..2].copy_from_slice(&5_u16.to_be_bytes());
        historical.truncate(historical.len() - 4 - b"wire source rendering".len());
        let encoded = framed(&historical);
        assert!(matches!(
            read_supervisor_request(&mut encoded.as_slice()),
            Err(WireError::UnsupportedVersion)
        ));
    }

    #[test]
    fn supervisor_cycle_proposal_round_trips_exact_budget_micros() {
        let request = SupervisorRequest::Execute {
            correlation: CorrelationId::new(1).expect("the fixed nonzero correlation is valid"),
            command: ClientCommandRequest {
                command_id: CommandId::parse("wire-cycle-budget-001")
                    .expect("the fixed command identity is valid"),
                principal_id: PrincipalId::new(3).expect("the fixed nonzero principal is valid"),
                capability_grant_id: CapabilityGrantId::new(1)
                    .expect("the fixed nonzero capability grant is valid"),
                capability: Capability::ProposeOperatingCycle,
                expected_generation: ExpectedGeneration::NotApplicable,
                body: ClientCommandBody::ProposeOperatingCycle {
                    treatment: OperatingCycleTreatment::PinnedPiSdkLiveV1,
                    budget_ceiling: UsdMicros::new(42_000)
                        .expect("the fixed positive budget ceiling is valid"),
                },
            },
        };
        let mut encoded = Vec::new();
        write_supervisor_request(&mut encoded, &request)
            .expect("the closed supervisor request must encode");
        assert_eq!(
            read_supervisor_request(&mut encoded.as_slice())
                .expect("the closed supervisor request must decode"),
            request
        );
    }
}
