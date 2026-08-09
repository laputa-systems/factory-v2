//! Closed, versioned Unix-domain wire values for the resident authority.
//!
//! The outer frame is `u32-be length || payload`. Every payload starts with a
//! fixed protocol version, a closed request/response tag, and a nonzero typed
//! correlation id. There is intentionally no JSON, map, generic payload, or
//! extension tag. Adding a command changes this module exhaustively.

use std::io::{self, Read, Write};

use society_kernel::{
    AdmissionGeneration, CancellationMode, Capability, CapabilityGrantId, CommandBody,
    CommandDisposition, CommandId, CommandReceipt, CommandRequest, CostPostmortemId,
    CostPostmortemResolution, ExpectedGeneration, GrandArchitectOfficeSessionId, OfficeTurnPurpose,
    OperatingCycleId, OperatingCycleTreatment, PrincipalDisplayName, PrincipalId, Rejection,
    Sha256Digest, SocietyName, UsdMicros,
};
use thiserror::Error;

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

// Request discriminants are intentionally partitioned by transport. A raw
// supervisor `Execute` tag is unknown to the named monitor socket before any
// command-shaped body could be considered.
const PUBLIC_RECEIPT_TAG: u8 = 0x21;
const PUBLIC_STATUS_TAG: u8 = 0x22;
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
    InstallGrandArchitectOffice,
    InstallFoundingUniverseSeed {
        rendering_digest: Sha256Digest,
    },
    AppointInitialGrandArchitect {
        actor_display_name: PrincipalDisplayName,
    },
    SetR0HardCeiling {
        ceiling: UsdMicros,
    },
    BootstrapSociety,
    ProposeOperatingCycle {
        treatment: OperatingCycleTreatment,
    },
    AdmitOperatingCycle {
        cycle_id: OperatingCycleId,
    },
    StartGrandArchitectOfficeSession {
        cycle_id: OperatingCycleId,
    },
    OpenOfficeTurn {
        session_id: GrandArchitectOfficeSessionId,
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
            Self::InstallGrandArchitectOffice => 2,
            Self::InstallFoundingUniverseSeed { .. } => 3,
            Self::AppointInitialGrandArchitect { .. } => 4,
            Self::SetR0HardCeiling { .. } => 5,
            Self::BootstrapSociety => 6,
            Self::ProposeOperatingCycle { .. } => 7,
            Self::AdmitOperatingCycle { .. } => 8,
            Self::StartGrandArchitectOfficeSession { .. } => 9,
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
            Self::InstallGrandArchitectOffice => CommandBody::InstallGrandArchitectOffice,
            Self::InstallFoundingUniverseSeed { rendering_digest } => {
                CommandBody::InstallFoundingUniverseSeed { rendering_digest }
            }
            Self::AppointInitialGrandArchitect { actor_display_name } => {
                CommandBody::AppointInitialGrandArchitect { actor_display_name }
            }
            Self::SetR0HardCeiling { ceiling } => CommandBody::SetR0HardCeiling { ceiling },
            Self::BootstrapSociety => CommandBody::BootstrapSociety,
            Self::ProposeOperatingCycle { treatment } => {
                CommandBody::ProposeOperatingCycle { treatment }
            }
            Self::AdmitOperatingCycle { cycle_id } => CommandBody::AdmitOperatingCycle { cycle_id },
            Self::StartGrandArchitectOfficeSession { cycle_id } => {
                CommandBody::StartGrandArchitectOfficeSession { cycle_id }
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
            Self::InstallGrandArchitectOffice => Capability::InstallGrandArchitectOffice,
            Self::InstallFoundingUniverseSeed { .. } => Capability::InstallFoundingUniverseSeed,
            Self::AppointInitialGrandArchitect { .. } => Capability::AppointInitialGrandArchitect,
            Self::SetR0HardCeiling { .. } => Capability::SetR0HardCeiling,
            Self::BootstrapSociety => Capability::BootstrapSociety,
            Self::ProposeOperatingCycle { .. } => Capability::ProposeOperatingCycle,
            Self::AdmitOperatingCycle { .. } => Capability::AdmitOperatingCycle,
            Self::StartGrandArchitectOfficeSession { .. } => {
                Capability::StartGrandArchitectOfficeSession
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
        ClientCommandBody::InstallGrandArchitectOffice | ClientCommandBody::BootstrapSociety => {}
        ClientCommandBody::InstallFoundingUniverseSeed { rendering_digest } => {
            bytes.extend_from_slice(&rendering_digest.as_bytes());
        }
        ClientCommandBody::AppointInitialGrandArchitect { actor_display_name } => {
            put_string(bytes, actor_display_name.as_str());
        }
        ClientCommandBody::SetR0HardCeiling { ceiling } => put_i64(bytes, ceiling.value()),
        ClientCommandBody::ProposeOperatingCycle { treatment } => put_u8(bytes, *treatment as u8),
        ClientCommandBody::AdmitOperatingCycle { cycle_id }
        | ClientCommandBody::StartGrandArchitectOfficeSession { cycle_id }
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
        2 => Ok(ClientCommandBody::InstallGrandArchitectOffice),
        3 => Ok(ClientCommandBody::InstallFoundingUniverseSeed {
            rendering_digest: Sha256Digest::from_bytes(cursor.array_32()?),
        }),
        4 => Ok(ClientCommandBody::AppointInitialGrandArchitect {
            actor_display_name: PrincipalDisplayName::parse(cursor.string(160)?)
                .map_err(|_| WireError::InvalidValue)?,
        }),
        5 => Ok(ClientCommandBody::SetR0HardCeiling {
            ceiling: UsdMicros::try_from(cursor.i64()?).map_err(|_| WireError::InvalidValue)?,
        }),
        6 => Ok(ClientCommandBody::BootstrapSociety),
        7 => Ok(ClientCommandBody::ProposeOperatingCycle {
            treatment: operating_cycle_treatment_from_u8(cursor.u8()?)?,
        }),
        8 => Ok(ClientCommandBody::AdmitOperatingCycle {
            cycle_id: OperatingCycleId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
        }),
        9 => Ok(ClientCommandBody::StartGrandArchitectOfficeSession {
            cycle_id: OperatingCycleId::try_from(cursor.i64()?)
                .map_err(|_| WireError::InvalidValue)?,
        }),
        11 => Ok(ClientCommandBody::OpenOfficeTurn {
            session_id: GrandArchitectOfficeSessionId::try_from(cursor.i64()?)
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

fn put_correlation(bytes: &mut Vec<u8>, correlation: CorrelationId) {
    bytes.extend_from_slice(&correlation.value().to_be_bytes());
}

fn put_string(bytes: &mut Vec<u8>, value: &str) {
    let length = u32::try_from(value.len()).expect("domain strings fit u32");
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
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
        2 => Ok(Capability::InstallGrandArchitectOffice),
        3 => Ok(Capability::InstallFoundingUniverseSeed),
        4 => Ok(Capability::AppointInitialGrandArchitect),
        5 => Ok(Capability::SetR0HardCeiling),
        6 => Ok(Capability::BootstrapSociety),
        7 => Ok(Capability::ProposeOperatingCycle),
        8 => Ok(Capability::AdmitOperatingCycle),
        9 => Ok(Capability::QuiesceOperatingCycle),
        10 => Ok(Capability::ResumeOperatingCycle),
        11 => Ok(Capability::ReconcileOperatingCycle),
        12 => Ok(Capability::CloseOperatingCycle),
        13 => Ok(Capability::StartGrandArchitectOfficeSession),
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
            | Capability::InstallGrandArchitectOffice
            | Capability::InstallFoundingUniverseSeed
            | Capability::AppointInitialGrandArchitect
            | Capability::SetR0HardCeiling
            | Capability::BootstrapSociety
            | Capability::ProposeOperatingCycle
            | Capability::AdmitOperatingCycle
            | Capability::QuiesceOperatingCycle
            | Capability::ResumeOperatingCycle
            | Capability::ReconcileOperatingCycle
            | Capability::CloseOperatingCycle
            | Capability::StartGrandArchitectOfficeSession
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
        _ => Err(WireError::InvalidValue),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kernel_rejection_round_trips_through_the_receipt_wire() {
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
}
