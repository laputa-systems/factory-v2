//! Typed local query client and supervisor-only command peer for `societyd`.
//!
//! This crate knows only the Unix-domain protocol. It deliberately has no
//! PostgreSQL dependency or database-URL API, so it cannot bypass the daemon's
//! single-writer boundary. The named socket client is query-only; only an
//! inherited anonymous stream supplied by the trusted process supervisor can
//! construct [`SupervisorClient`] and submit a kernel command.

use std::{
    io,
    os::{fd::AsRawFd, unix::net::UnixStream},
    path::{Path, PathBuf},
};

use society_kernel::{Capability, CapabilityGrantId, PrincipalId};
use societyd::protocol::{
    self, ClientCommandRequest, CommandReceiptView, CorrelationId, DaemonStatus, ProtocolErrorCode,
    PublicRequest, Response, SupervisorRequest,
};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct SocietyctlClient {
    socket_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum SocietyctlError {
    #[error("local socket I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("local protocol failed: {0}")]
    Wire(#[from] protocol::WireError),
    #[error("daemon rejected the request before dispatch: {0:?}")]
    Daemon(ProtocolErrorCode),
    #[error("daemon returned a response for another correlation")]
    CorrelationMismatch,
    #[error("daemon returned an unexpected response variant")]
    UnexpectedResponse,
    #[error("failed to contain an inherited supervisor descriptor: {0}")]
    AuthorityDescriptor(io::Error),
}

impl SocietyctlClient {
    pub fn connect(socket_path: impl AsRef<Path>) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
        }
    }

    pub fn command_receipt(
        &self,
        correlation: CorrelationId,
        command_id: society_kernel::CommandId,
    ) -> Result<Option<CommandReceiptView>, SocietyctlError> {
        match self.round_trip(PublicRequest::CommandReceipt {
            correlation,
            command_id,
        })? {
            Response::CommandReceiptLookup {
                correlation: received,
                receipt,
            } if received == correlation => Ok(receipt),
            Response::Error {
                correlation: received,
                code,
            } if received == correlation => Err(SocietyctlError::Daemon(code)),
            Response::CommandReceiptLookup { .. } | Response::Error { .. } => {
                Err(SocietyctlError::CorrelationMismatch)
            }
            _ => Err(SocietyctlError::UnexpectedResponse),
        }
    }

    pub fn status(&self, correlation: CorrelationId) -> Result<DaemonStatus, SocietyctlError> {
        match self.round_trip(PublicRequest::Status { correlation })? {
            Response::Status {
                correlation: received,
                status,
            } if received == correlation => Ok(status),
            Response::Error {
                correlation: received,
                code,
            } if received == correlation => Err(SocietyctlError::Daemon(code)),
            Response::Status { .. } | Response::Error { .. } => {
                Err(SocietyctlError::CorrelationMismatch)
            }
            _ => Err(SocietyctlError::UnexpectedResponse),
        }
    }

    fn round_trip(&self, request: PublicRequest) -> Result<Response, SocietyctlError> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        protocol::write_public_request(&mut stream, &request)?;
        Ok(protocol::read_response(&mut stream)?)
    }
}

/// The supervisor-held peer of the daemon's anonymous inherited stream. This
/// value is deliberately constructible only from an already-owned stream; it
/// has no pathname, environment-variable, or credential-file constructor.
pub struct SupervisorClient {
    stream: UnixStream,
}

impl SupervisorClient {
    pub fn from_inherited_stream(stream: UnixStream) -> Result<Self, SocietyctlError> {
        set_close_on_exec(stream.as_raw_fd()).map_err(SocietyctlError::AuthorityDescriptor)?;
        Ok(Self { stream })
    }

    pub fn execute(
        &mut self,
        correlation: CorrelationId,
        command: ClientCommandRequest,
    ) -> Result<CommandReceiptView, SocietyctlError> {
        match round_trip_stream(
            &mut self.stream,
            SupervisorRequest::Execute {
                correlation,
                command,
            },
        )? {
            Response::CommandReceipt {
                correlation: received,
                receipt,
            } if received == correlation => Ok(receipt),
            Response::Error {
                correlation: received,
                code,
            } if received == correlation => Err(SocietyctlError::Daemon(code)),
            Response::CommandReceipt { .. } | Response::Error { .. } => {
                Err(SocietyctlError::CorrelationMismatch)
            }
            _ => Err(SocietyctlError::UnexpectedResponse),
        }
    }

    /// Resolves the kernel's current exact grant identity over the same
    /// supervisor-only authority channel. A caller still cannot use that
    /// identity on the named monitor socket, which has no execute vocabulary.
    pub fn active_capability_grant(
        &mut self,
        correlation: CorrelationId,
        principal_id: PrincipalId,
        capability: Capability,
    ) -> Result<Option<CapabilityGrantId>, SocietyctlError> {
        match round_trip_stream(
            &mut self.stream,
            SupervisorRequest::ActiveCapabilityGrant {
                correlation,
                principal_id,
                capability,
            },
        )? {
            Response::ActiveCapabilityGrant {
                correlation: received,
                capability_grant_id,
            } if received == correlation => Ok(capability_grant_id),
            Response::Error {
                correlation: received,
                code,
            } if received == correlation => Err(SocietyctlError::Daemon(code)),
            Response::ActiveCapabilityGrant { .. } | Response::Error { .. } => {
                Err(SocietyctlError::CorrelationMismatch)
            }
            _ => Err(SocietyctlError::UnexpectedResponse),
        }
    }
}

fn set_close_on_exec(descriptor: libc::c_int) -> io::Result<()> {
    // SAFETY: `F_GETFD` inspects only descriptor flags for this live stream.
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `F_SETFD` updates only the close-on-exec flag for this endpoint.
    if unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn round_trip_stream(
    stream: &mut UnixStream,
    request: SupervisorRequest,
) -> Result<Response, SocietyctlError> {
    protocol::write_supervisor_request(stream, &request)?;
    Ok(protocol::read_response(stream)?)
}
