use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
            net::{UnixListener, UnixStream},
        },
    },
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use society_content::{
    ContentSealLimit, ContentStoreError, ContentStoreRoot, ContentStoreRootError,
};
use society_kernel::{
    Capability, CommandBody, CommandDisposition, CommandId, CommandRequest, ExpectedGeneration,
    KernelStore, PrincipalId, StoreError,
};
use thiserror::Error;

use crate::content::{
    ContentObjectRegistration, ContentSealOperationId, ContentSealingAuthority, ContentSealingError,
};
use crate::pi_execution::{
    OfficePiExecutionChild, OfficePiExecutionStart, OfficePiSpawnRegistration, PiExecutionDriver,
    PiExecutionError, UnregisteredOfficePiChild,
};
use crate::protocol::{
    ClientCommandBody, ClientCommandRequest, CorrelationId, DaemonStatus, ProtocolErrorCode,
    PublicRequest, Response, SupervisorRequest, WireError,
};

const DATABASE_FILE_NAME: &str = "society.sqlite3";
const SOCKET_FILE_NAME: &str = "societyd.sock";
const LOCK_FILE_NAME: &str = "societyd.lock";
const CONTENT_STORE_DIRECTORY_NAME: &str = "content";
const DAEMON_CONTENT_SEAL_LIMIT_BYTES: u64 = 64 * 1024 * 1024;
const SOCKET_READ_TIMEOUT: Duration = Duration::from_secs(2);
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(5);

// Set only while `SignalWake` is alive. The installed handler uses no heap,
// lock, or Rust I/O: it merely best-effort writes one byte to this nonblocking
// self-pipe, which is an async-signal-safe POSIX operation.
static SIGNAL_WAKE_WRITE_FD: AtomicI32 = AtomicI32::new(-1);
static SIGNAL_WAKE_INSTALLED: AtomicBool = AtomicBool::new(false);
static SIGNAL_WAKE_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);

/// Startup state is deliberately visible: without the missing kernel recovery
/// query/fence API, a restarted nonempty ledger may answer receipt queries and
/// exact duplicate submissions but refuses every new command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupMode {
    FreshServing,
    RecoveryFenced,
}

/// Deterministic boundary seams for crash-recovery integration tests. They are
/// not a wire command and cannot be selected by a socket peer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultInjection {
    None,
    BeforeNextCommandCommit,
    AfterNextCommandCommit,
}

pub struct DaemonConfig {
    runtime_root: PathBuf,
    fault_injection: FaultInjection,
    supervisor_stream: Option<UnixStream>,
}

impl DaemonConfig {
    pub fn new(runtime_root: impl AsRef<Path>) -> Self {
        Self {
            runtime_root: runtime_root.as_ref().to_path_buf(),
            fault_injection: FaultInjection::None,
            supervisor_stream: None,
        }
    }

    pub fn with_fault_injection(mut self, fault_injection: FaultInjection) -> Self {
        self.fault_injection = fault_injection;
        self
    }

    /// Supplies the daemon end of an anonymous Unix stream inherited from the
    /// trusted process supervisor. No pathname or secret material represents
    /// this authority, and the peer endpoint must never enter an actor
    /// workspace or the public socket protocol.
    pub fn with_supervisor_stream(mut self, supervisor_stream: UnixStream) -> Self {
        self.supervisor_stream = Some(supervisor_stream);
        self
    }

    pub fn runtime_root(&self) -> &Path {
        &self.runtime_root
    }
}

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("runtime I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("kernel operation failed: {0}")]
    Kernel(#[from] StoreError),
    #[error("local protocol failed: {0}")]
    Wire(#[from] WireError),
    #[error("another societyd instance owns this runtime root")]
    AlreadyRunning,
    #[error("refusing to delete a non-socket at the daemon socket path")]
    UnsafeSocketPath,
    #[error("runtime root is not a private directory owned by this daemon user")]
    UnsafeRuntimeRoot,
    #[error("runtime-owned file is not a private regular file owned by this daemon user")]
    UnsafeRuntimeFile,
    #[error("content-store root is not a canonical private daemon path")]
    ContentStoreRoot(#[from] ContentStoreRootError),
    #[error("daemon content-store failed: {0}")]
    ContentStore(#[from] ContentStoreError),
    #[error("compiled daemon content seal limit must be nonzero")]
    InvalidContentSealLimit,
    #[error("supervisor authority must be a connected same-user Unix stream")]
    InvalidSupervisorStream,
    #[error("the test-only fault seam stopped before command commit")]
    InjectedCrashBeforeCommit,
    #[error("the test-only fault seam stopped after command commit")]
    InjectedCrashAfterCommit,
}

/// A resident daemon owns exactly one kernel connection and serially dispatches
/// every socket command on its control-loop thread.
pub struct Daemon {
    config: DaemonConfig,
    store: KernelStore,
    /// The resident daemon exclusively owns this physical content-store writer.
    /// Its only mutation method is crate-private and has no local-wire form.
    #[allow(dead_code)]
    content_sealing: ContentSealingAuthority,
    /// The daemon exclusively owns live Pi process physics.  The driver has
    /// no local-wire constructor and cannot survive a restart attach.
    #[allow(dead_code)]
    pi_execution: PiExecutionDriver,
    listener: UnixListener,
    _lock: File,
    owner_uid: libc::uid_t,
    supervisor_stream: Option<UnixStream>,
    mode: StartupMode,
    fault_injection: FaultInjection,
}

/// Typed cooperative shutdown for the synchronous accept loop. Requesting
/// shutdown both sets the durable-in-process cancellation bit and opens a local
/// socket connection to wake a nonblocking accept loop without an async runtime.
#[derive(Clone)]
pub struct ShutdownHandle {
    requested: Arc<AtomicBool>,
    socket_path: PathBuf,
    signal_wake: Option<Arc<SignalWake>>,
}

impl ShutdownHandle {
    pub fn request_shutdown(&self) {
        self.requested.store(true, Ordering::Release);
        let _ = UnixStream::connect(&self.socket_path);
    }

    pub fn is_requested(&self) -> bool {
        if let Some(signal_wake) = &self.signal_wake
            && signal_wake.take_signal().unwrap_or(false)
        {
            self.requested.store(true, Ordering::Release);
        }
        self.requested.load(Ordering::Acquire)
    }

    /// Installs the process-local `SIGINT`/`SIGTERM` bridge. The first signal
    /// requests orderly daemon shutdown; additional signals before that loop
    /// exits coalesce in the nonblocking pipe. Dropping the final handle
    /// restores the previous dispositions.
    pub fn with_process_signals(mut self) -> Result<Self, DaemonError> {
        self.signal_wake = Some(Arc::new(SignalWake::install()?));
        Ok(self)
    }
}

/// Process-local signal-to-control-loop bridge. This intentionally is not a
/// wire command and grants no authority to a socket peer.
struct SignalWake {
    read_end: File,
    _write_end: File,
    previous_sigint: libc::sigaction,
    previous_sigterm: libc::sigaction,
}

impl SignalWake {
    fn install() -> io::Result<Self> {
        let mut descriptors = [-1_i32; 2];
        // SAFETY: `descriptors` supplies room for the two file descriptors
        // returned by POSIX `pipe`.
        if unsafe { libc::pipe(descriptors.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        if let Err(error) = configure_self_pipe_end(descriptors[0])
            .and_then(|()| configure_self_pipe_end(descriptors[1]))
        {
            // SAFETY: both descriptor values came directly from `pipe` and
            // have not yet been transferred into owning `File` values.
            unsafe {
                let _ = libc::close(descriptors[0]);
                let _ = libc::close(descriptors[1]);
            }
            return Err(error);
        }
        // SAFETY: the read descriptor is a live endpoint returned by `pipe`
        // and is transferred into exactly one owning `File` value.
        let read_end = unsafe { File::from_raw_fd(descriptors[0]) };
        // SAFETY: the write descriptor is the other live endpoint returned by
        // `pipe` and is transferred into exactly one owning `File` value.
        let write_end = unsafe { File::from_raw_fd(descriptors[1]) };
        let installed = install_signal_handlers(write_end.as_raw_fd());
        let (previous_sigint, previous_sigterm) = match installed {
            Ok(handlers) => handlers,
            Err(error) => {
                return Err(error);
            }
        };
        Ok(Self {
            read_end,
            _write_end: write_end,
            previous_sigint,
            previous_sigterm,
        })
    }

    fn take_signal(&self) -> io::Result<bool> {
        let mut bytes = [0_u8; 32];
        match (&self.read_end).read(&mut bytes) {
            Ok(count) => Ok(count > 0),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(false),
            Err(error) => Err(error),
        }
    }
}

fn configure_self_pipe_end(descriptor: libc::c_int) -> io::Result<()> {
    // SAFETY: `descriptor` comes from a live pipe and the `fcntl` operations
    // do not dereference caller memory.
    let status_flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    if status_flags < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: uses the just-read status flags augmented by the defined
    // nonblocking flag for this live descriptor.
    if unsafe { libc::fcntl(descriptor, libc::F_SETFL, status_flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `F_GETFD` has no pointer argument and reads flags only.
    let descriptor_flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    if descriptor_flags < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: applies close-on-exec to this exact live pipe endpoint.
    if unsafe {
        libc::fcntl(
            descriptor,
            libc::F_SETFD,
            descriptor_flags | libc::FD_CLOEXEC,
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

extern "C" fn received_process_signal(_signal: libc::c_int) {
    // SAFETY: this is an async signal handler. It uses a lock-free atomic to
    // read the installed descriptor and calls only POSIX `write`; the pipe is
    // nonblocking, so a full pipe merely coalesces repeated signals.
    unsafe {
        SIGNAL_WAKE_IN_FLIGHT.fetch_add(1, Ordering::SeqCst);
        let descriptor = SIGNAL_WAKE_WRITE_FD.load(Ordering::SeqCst);
        if descriptor >= 0 {
            let byte = 1_u8;
            let _ = libc::write(descriptor, (&byte as *const u8).cast::<libc::c_void>(), 1);
        }
        SIGNAL_WAKE_IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
    }
}

fn install_signal_handlers(
    write_fd: libc::c_int,
) -> io::Result<(libc::sigaction, libc::sigaction)> {
    // SAFETY: zero initialization is the documented starting representation
    // for `sigaction`; fields are filled before either call observes it.
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = received_process_signal as *const () as usize;
    action.sa_flags = 0;
    // SAFETY: `action.sa_mask` is valid writable signal-set storage.
    if unsafe { libc::sigemptyset(&mut action.sa_mask) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if SIGNAL_WAKE_INSTALLED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "a societyd process signal bridge is already installed",
        ));
    }
    SIGNAL_WAKE_WRITE_FD.store(write_fd, Ordering::SeqCst);
    // SAFETY: zero initialization provides writable storage for the previous
    // SIGINT disposition returned by `sigaction` below.
    let mut previous_sigint: libc::sigaction = unsafe { std::mem::zeroed() };
    // SAFETY: `action` is fully initialized, `previous_sigint` is valid output
    // storage, and the global wake descriptor is set before the handler runs.
    if unsafe { libc::sigaction(libc::SIGINT, &action, &mut previous_sigint) } != 0 {
        SIGNAL_WAKE_WRITE_FD.store(-1, Ordering::SeqCst);
        wait_for_signal_handlers();
        SIGNAL_WAKE_INSTALLED.store(false, Ordering::Release);
        return Err(io::Error::last_os_error());
    }
    // SAFETY: zero initialization provides writable storage for the previous
    // SIGTERM disposition returned by `sigaction` below.
    let mut previous_sigterm: libc::sigaction = unsafe { std::mem::zeroed() };
    // SAFETY: `action` is fully initialized, `previous_sigterm` is valid output
    // storage, and the global wake descriptor is set before the handler runs.
    if unsafe { libc::sigaction(libc::SIGTERM, &action, &mut previous_sigterm) } != 0 {
        // SAFETY: restore the successful first installation before returning
        // an error, so a failed bridge never leaves a partial disposition.
        unsafe {
            let _ = libc::sigaction(libc::SIGINT, &previous_sigint, std::ptr::null_mut());
        }
        SIGNAL_WAKE_WRITE_FD.store(-1, Ordering::SeqCst);
        wait_for_signal_handlers();
        SIGNAL_WAKE_INSTALLED.store(false, Ordering::Release);
        return Err(io::Error::last_os_error());
    }
    Ok((previous_sigint, previous_sigterm))
}

impl Drop for SignalWake {
    fn drop(&mut self) {
        // First prohibit further handler writes, then restore the prior
        // dispositions before `read_end` is closed. The daemon is single
        // process-owner by construction, so no competing installation exists.
        // Replacing the lock-free handler-visible descriptor with -1 makes
        // subsequent asynchronous deliveries a no-op.
        SIGNAL_WAKE_WRITE_FD.store(-1, Ordering::SeqCst);
        wait_for_signal_handlers();
        // SAFETY: both values were returned by successful `sigaction` calls
        // for these exact signals in `install_signal_handlers`.
        unsafe {
            let _ = libc::sigaction(libc::SIGINT, &self.previous_sigint, std::ptr::null_mut());
            let _ = libc::sigaction(libc::SIGTERM, &self.previous_sigterm, std::ptr::null_mut());
        }
        SIGNAL_WAKE_INSTALLED.store(false, Ordering::Release);
    }
}

fn wait_for_signal_handlers() {
    while SIGNAL_WAKE_IN_FLIGHT.load(Ordering::SeqCst) != 0 {
        std::hint::spin_loop();
    }
}

impl Daemon {
    /// Acquires the runtime-root lock, opens the only writable kernel store,
    /// verifies replay before socket bind, then creates a user-only socket.
    pub fn bind(mut config: DaemonConfig) -> Result<Self, DaemonError> {
        prepare_runtime_root(&config.runtime_root)?;
        let lock = acquire_runtime_lock(&config.runtime_root)?;
        let supervisor_stream = config.supervisor_stream.take();
        if let Some(stream) = &supervisor_stream {
            validate_supervisor_stream(stream)?;
            set_close_on_exec(stream.as_raw_fd())?;
            stream.set_read_timeout(Some(SOCKET_READ_TIMEOUT))?;
            stream.set_write_timeout(Some(SOCKET_READ_TIMEOUT))?;
        }
        let socket_path = config.runtime_root.join(SOCKET_FILE_NAME);
        remove_stale_socket(&socket_path)?;

        let database_path = config.runtime_root.join(DATABASE_FILE_NAME);
        validate_runtime_file_if_present(&database_path)?;
        let store = KernelStore::open(&database_path)?;
        set_mode(&database_path, 0o600)?;
        validate_runtime_file_metadata(&fs::symlink_metadata(&database_path)?)?;
        let command_count = store.command_count()?;
        store.validate_replayed_materialized_state()?;
        let mode = if command_count == 0 {
            StartupMode::FreshServing
        } else {
            StartupMode::RecoveryFenced
        };
        let content_root =
            ContentStoreRoot::parse(config.runtime_root.join(CONTENT_STORE_DIRECTORY_NAME))?;
        let Some(content_limit) = ContentSealLimit::new(DAEMON_CONTENT_SEAL_LIMIT_BYTES) else {
            return Err(DaemonError::InvalidContentSealLimit);
        };
        let content_sealing = ContentSealingAuthority::open(content_root, content_limit)?;

        let listener = UnixListener::bind(&socket_path)?;
        set_mode(&socket_path, 0o600)?;
        listener.set_nonblocking(true)?;
        let owner_uid = effective_uid();
        tracing::info!(
            target: "society.ledger",
            command_count,
            ?mode,
            "resident authority bound after replay validation"
        );
        let fault_injection = config.fault_injection;
        Ok(Self {
            config,
            store,
            content_sealing,
            pi_execution: PiExecutionDriver::new(),
            listener,
            _lock: lock,
            owner_uid,
            supervisor_stream,
            mode,
            fault_injection,
        })
    }

    pub fn socket_path(&self) -> PathBuf {
        self.config.runtime_root.join(SOCKET_FILE_NAME)
    }

    pub fn startup_mode(&self) -> StartupMode {
        self.mode
    }

    /// Daemon-internal content writer. It is intentionally unavailable from
    /// both local protocols: physical sealing must occur under resident
    /// custody before the two kernel-service commands are issued.
    #[allow(dead_code)]
    pub(crate) fn seal_content_object(
        &mut self,
        operation: &ContentSealOperationId,
        bytes: &[u8],
    ) -> Result<ContentObjectRegistration, ContentSealingError> {
        if self.mode == StartupMode::RecoveryFenced {
            return Err(ContentSealingError::RecoveryFenced);
        }
        self.content_sealing
            .seal_and_register(&mut self.store, operation, bytes)
    }

    /// Daemon-only M5 process admission.  Neither local protocol can encode
    /// this request or acquire the kernel service capabilities it consumes.
    /// A recovered daemon deliberately refuses it: parentage cannot be
    /// reconstructed by spawning a replacement host.
    #[allow(dead_code)]
    pub(crate) fn admit_office_pi_child(
        &mut self,
        start: OfficePiExecutionStart,
    ) -> Result<OfficePiSpawnRegistration, PiExecutionError> {
        if self.mode == StartupMode::RecoveryFenced {
            return Err(PiExecutionError::RecoveryFenced);
        }
        let registration = self
            .pi_execution
            .admit_spawn_and_register(&mut self.store, start)?;
        if matches!(
            &registration,
            OfficePiSpawnRegistration::RegistrationUnresolved { .. }
        ) {
            // The kernel has an admission but no child-process identity, so
            // this resident may only finish containment of the returned
            // native handle. It must not admit a successor or new work.
            self.mode = StartupMode::RecoveryFenced;
        }
        Ok(registration)
    }

    /// Drives the fixed emergency path for any registered child that crossed
    /// a daemon-private Pi boundary failure after its PID/PGID receipt.
    #[allow(dead_code)]
    pub(crate) fn drive_office_pi_boundary_containment(
        &mut self,
        child: &OfficePiExecutionChild,
        now: crate::supervision::MonotonicTick,
    ) -> Result<(), PiExecutionError> {
        self.pi_execution.drive_boundary_containment(child, now)
    }

    /// The only permitted work while a post-exec registration failure has
    /// fenced this resident: finish physical containment of that exact
    /// unregistered child. It does not issue a kernel process receipt.
    #[allow(dead_code)]
    pub(crate) fn drive_unregistered_office_pi_containment(
        &mut self,
        child: &mut UnregisteredOfficePiChild,
        now: crate::supervision::MonotonicTick,
    ) -> Result<bool, PiExecutionError> {
        self.pi_execution
            .drive_unregistered_spawn_containment(child, now)
    }

    #[allow(dead_code)]
    pub(crate) fn observe_office_pi_adapter_ready(
        &mut self,
        child: &mut OfficePiExecutionChild,
        now: crate::supervision::MonotonicTick,
        deadline: crate::supervision::HandshakeDeadline,
    ) -> Result<bool, PiExecutionError> {
        self.pi_execution
            .observe_adapter_ready(&mut self.store, child, now, deadline)
    }

    #[allow(dead_code)]
    pub(crate) fn authorize_and_begin_office_pi_create(
        &mut self,
        child: &mut OfficePiExecutionChild,
        now: crate::supervision::MonotonicTick,
        deadline: crate::supervision::ControlWriteDeadline,
    ) -> Result<crate::supervision::ControlWriteProgress, PiExecutionError> {
        self.pi_execution
            .authorize_and_begin_create(&mut self.store, child, now, deadline)
    }

    #[allow(dead_code)]
    pub(crate) fn drive_office_pi_create_delivery(
        &mut self,
        child: &mut OfficePiExecutionChild,
        now: crate::supervision::MonotonicTick,
    ) -> Result<crate::supervision::ControlWriteProgress, PiExecutionError> {
        self.pi_execution
            .drive_create_delivery(&mut self.store, child, now)
    }

    #[allow(dead_code)]
    pub(crate) fn observe_office_pi_session_ready(
        &mut self,
        child: &mut OfficePiExecutionChild,
        now: crate::supervision::MonotonicTick,
        deadline: crate::supervision::HandshakeDeadline,
    ) -> Result<bool, PiExecutionError> {
        self.pi_execution
            .observe_session_ready(&mut self.store, child, now, deadline)
    }

    #[allow(dead_code)]
    pub(crate) fn begin_office_pi_dispose(
        &mut self,
        child: &mut OfficePiExecutionChild,
        correlation: society_pi::CorrelationIdentity,
        now: crate::supervision::MonotonicTick,
        deadline: crate::supervision::ControlWriteDeadline,
    ) -> Result<crate::supervision::ControlWriteProgress, PiExecutionError> {
        self.pi_execution
            .begin_dispose(child, correlation, now, deadline)
    }

    #[allow(dead_code)]
    pub(crate) fn drive_office_pi_dispose_delivery(
        &mut self,
        child: &mut OfficePiExecutionChild,
        now: crate::supervision::MonotonicTick,
    ) -> Result<crate::supervision::ControlWriteProgress, PiExecutionError> {
        self.pi_execution.drive_dispose_delivery(child, now)
    }

    #[allow(dead_code)]
    pub(crate) fn observe_office_pi_disposed(
        &mut self,
        child: &mut OfficePiExecutionChild,
        now: crate::supervision::MonotonicTick,
        deadline: crate::supervision::HandshakeDeadline,
    ) -> Result<bool, PiExecutionError> {
        self.pi_execution.observe_disposed(child, now, deadline)
    }

    #[allow(dead_code)]
    pub(crate) fn reconcile_reaped_office_pi_child(
        &mut self,
        child: &mut OfficePiExecutionChild,
        now: crate::supervision::MonotonicTick,
    ) -> Result<bool, PiExecutionError> {
        if self.mode == StartupMode::RecoveryFenced {
            return Err(PiExecutionError::RecoveryFenced);
        }
        self.pi_execution.poll_reap_and_reconcile(
            &mut self.store,
            &self.content_sealing,
            child,
            now,
        )
    }

    /// Reports whether the adopted supervisor endpoint is marked close-on-exec.
    ///
    /// This is a narrow process-containment observation, not a way to recover
    /// the endpoint or grant command authority. Future Pi child spawning must
    /// retain this invariant before it can be admitted to the daemon.
    pub fn supervisor_authority_close_on_exec(&self) -> Result<Option<bool>, DaemonError> {
        self.supervisor_stream
            .as_ref()
            .map(|stream| descriptor_has_close_on_exec(stream.as_raw_fd()))
            .transpose()
    }

    pub fn shutdown_handle(&self) -> ShutdownHandle {
        ShutdownHandle {
            requested: Arc::new(AtomicBool::new(false)),
            socket_path: self.socket_path(),
            signal_wake: None,
        }
    }

    /// Processes one connection at a time until [`ShutdownHandle`] is asked to
    /// stop. A malformed peer receives no state transition; every valid request
    /// gets exactly one typed response unless a deterministic crash seam fires.
    pub fn serve_until(&mut self, shutdown: &ShutdownHandle) -> Result<(), DaemonError> {
        loop {
            if shutdown.is_requested() {
                break;
            }
            if self.serve_supervisor_once()? {
                continue;
            }
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if shutdown.is_requested() {
                        break;
                    }
                    self.serve_public_connection(stream)?;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(ACCEPT_POLL_INTERVAL);
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(DaemonError::Io(error)),
            }
        }
        tracing::info!(target: "society.ledger", "resident authority control loop stopped");
        Ok(())
    }

    /// The named public socket is deliberately query-only. Same-UID actor
    /// workspaces can connect to it, so accepting its claimed principal or
    /// grant would make the kernel's internal authority forgeable.
    fn serve_public_connection(&mut self, mut stream: UnixStream) -> Result<(), DaemonError> {
        if let Err(error) = stream
            .set_read_timeout(Some(SOCKET_READ_TIMEOUT))
            .and_then(|()| stream.set_write_timeout(Some(SOCKET_READ_TIMEOUT)))
        {
            tracing::warn!(target: "society.ledger", error = %error, "rejected unusable public monitor peer");
            return Ok(());
        }
        match peer_uid(&stream) {
            Ok(uid) if uid == self.owner_uid => {}
            Ok(_) => return Ok(()),
            // A peer may close immediately after connecting with a malformed
            // frame. On Darwin `getpeereid` reports that as `EINVAL`; it is a
            // rejected public peer, never a daemon-fatal condition.
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::InvalidInput
                        | io::ErrorKind::NotConnected
                        | io::ErrorKind::ConnectionReset
                ) =>
            {
                return Ok(());
            }
            Err(error) => {
                tracing::warn!(target: "society.ledger", error = %error, "rejected public monitor peer without usable attribution");
                return Ok(());
            }
        }
        let request = match crate::protocol::read_public_request(&mut stream) {
            Ok(request) => request,
            Err(WireError::EndOfStream) => return Ok(()),
            Err(WireError::Io(error)) if error.kind() == io::ErrorKind::Interrupted => {
                return Ok(());
            }
            Err(error) => {
                tracing::warn!(target: "society.ledger", error = %error, "rejected malformed local frame");
                return Ok(());
            }
        };
        let response = self.dispatch_public_query(request)?;
        match crate::protocol::write_response(&mut stream, &response) {
            Ok(()) => Ok(()),
            // A named-socket peer owns no mutation authority. Its response
            // loss must never terminate the resident single writer.
            Err(error) => {
                tracing::warn!(target: "society.ledger", error = %error, "public monitor reply was not delivered");
                Ok(())
            }
        }
    }

    /// Services one frame on the anonymous supervisor stream if one is ready.
    /// A closed or malformed supervisor peer loses admission rather than
    /// falling back to the public socket. This makes authority loss fail shut.
    fn serve_supervisor_once(&mut self) -> Result<bool, DaemonError> {
        let Some(stream) = self.supervisor_stream.as_ref() else {
            return Ok(false);
        };
        if !stream_is_readable(stream)? {
            return Ok(false);
        }
        let result = match self.supervisor_stream.as_mut() {
            Some(stream) => crate::protocol::read_supervisor_request(stream),
            None => return Ok(false),
        };
        let request = match result {
            Ok(request) => request,
            Err(WireError::EndOfStream) => {
                self.supervisor_stream = None;
                tracing::warn!(target: "society.ledger", "supervisor admission stream closed; public daemon remains query-only");
                return Ok(false);
            }
            Err(error) => {
                self.supervisor_stream = None;
                tracing::warn!(target: "society.ledger", error = %error, "supervisor admission stream became invalid; failing closed");
                return Ok(false);
            }
        };
        let response = self.dispatch_supervisor(request)?;
        let write_result = match self.supervisor_stream.as_mut() {
            Some(stream) => crate::protocol::write_response(stream, &response),
            None => return Ok(false),
        };
        if let Err(error) = write_result {
            self.supervisor_stream = None;
            tracing::warn!(target: "society.ledger", error = %error, "supervisor reply failed; command admission is now fail-closed");
        }
        Ok(true)
    }

    fn dispatch_supervisor(&mut self, request: SupervisorRequest) -> Result<Response, DaemonError> {
        match request {
            SupervisorRequest::Execute {
                correlation,
                command,
            } => {
                // The supervisor may authorize a founding or Root Authority
                // command, but no peer may claim the daemon-only kernel
                // service identity or its lifecycle capabilities through this
                // protocol. Those facts are constructed in this control loop.
                if command.principal_id == PrincipalId::KERNEL
                    || daemon_only_capability(command.capability)
                {
                    return Ok(Response::Error {
                        correlation,
                        code: ProtocolErrorCode::PeerNotAuthorized,
                    });
                }
                self.execute(correlation, command)
            }
            SupervisorRequest::CommandReceipt {
                correlation,
                command_id,
            } => self.dispatch_public_query(PublicRequest::CommandReceipt {
                correlation,
                command_id,
            }),
            SupervisorRequest::Status { correlation } => {
                self.dispatch_public_query(PublicRequest::Status { correlation })
            }
            SupervisorRequest::ActiveCapabilityGrant {
                correlation,
                principal_id,
                capability,
            } => {
                if principal_id == PrincipalId::KERNEL || daemon_only_capability(capability) {
                    return Ok(Response::Error {
                        correlation,
                        code: ProtocolErrorCode::PeerNotAuthorized,
                    });
                }
                Ok(Response::ActiveCapabilityGrant {
                    correlation,
                    capability_grant_id: self
                        .store
                        .active_capability_grant(principal_id, capability)?,
                })
            }
        }
    }

    fn dispatch_public_query(&self, request: PublicRequest) -> Result<Response, DaemonError> {
        match request {
            PublicRequest::CommandReceipt {
                correlation,
                command_id,
            } => Ok(Response::CommandReceiptLookup {
                correlation,
                receipt: self.store.command_receipt(&command_id)?.map(Into::into),
            }),
            PublicRequest::Status { correlation } => Ok(Response::Status {
                correlation,
                status: self.status()?,
            }),
        }
    }

    fn execute(
        &mut self,
        correlation: CorrelationId,
        command: ClientCommandRequest,
    ) -> Result<Response, DaemonError> {
        if self.mode == StartupMode::RecoveryFenced
            && self.store.command_receipt(&command.command_id)?.is_none()
        {
            return Ok(Response::Error {
                correlation,
                code: ProtocolErrorCode::RecoveryFenced,
            });
        }
        if self.fault_injection == FaultInjection::BeforeNextCommandCommit {
            self.fault_injection = FaultInjection::None;
            return Err(DaemonError::InjectedCrashBeforeCommit);
        }
        let command_id = command.command_id.clone();
        let drain_cycle_id = match &command.body {
            ClientCommandBody::QuiesceOperatingCycle { cycle_id } => Some(*cycle_id),
            _ => None,
        };
        let receipt = match self.store.execute(command.into_kernel()) {
            Ok(receipt) => receipt,
            Err(StoreError::IdempotencyConflict) => {
                return Ok(Response::Error {
                    correlation,
                    code: ProtocolErrorCode::IdempotencyConflict,
                });
            }
            Err(error) => {
                tracing::error!(target: "society.ledger", error = %error, "kernel rejected a transport-valid command");
                return Ok(Response::Error {
                    correlation,
                    code: ProtocolErrorCode::KernelFailure,
                });
            }
        };
        if self.fault_injection == FaultInjection::AfterNextCommandCommit {
            self.fault_injection = FaultInjection::None;
            return Err(DaemonError::InjectedCrashAfterCommit);
        }
        if let (Some(cycle_id), CommandDisposition::Accepted(_)) =
            (drain_cycle_id, receipt.disposition)
            && !receipt.idempotent
        {
            self.record_empty_cycle_drained(&command_id, cycle_id)?;
        }
        Ok(Response::CommandReceipt {
            correlation,
            receipt: receipt.into(),
        })
    }

    /// In this no-Pi tranche a newly quiesced cycle has no registered child or
    /// live session. The daemon therefore records the closed kernel lifecycle
    /// fact itself. Once supervision exists, only its reaper can issue this
    /// internal command after proving those same facts.
    fn record_empty_cycle_drained(
        &mut self,
        outer_command_id: &CommandId,
        cycle_id: society_kernel::OperatingCycleId,
    ) -> Result<(), DaemonError> {
        let capability = Capability::RecordCycleDrained;
        let Some(grant_id) = self
            .store
            .active_capability_grant(PrincipalId::KERNEL, capability)?
        else {
            return Err(DaemonError::Kernel(StoreError::LedgerCorruption(
                "kernel service drain capability is absent",
            )));
        };
        let internal_command_id = CommandId::parse(format!(
            "kernel-drain-{:?}",
            society_kernel::Blake3Digest::of_bytes(outer_command_id.as_str().as_bytes())
        ))
        .map_err(|_| {
            DaemonError::Kernel(StoreError::LedgerCorruption("invalid internal command id"))
        })?;
        let receipt = self.store.execute(CommandRequest {
            command_id: internal_command_id,
            principal_id: PrincipalId::KERNEL,
            capability_grant_id: grant_id,
            capability,
            expected_generation: ExpectedGeneration::NotApplicable,
            body: CommandBody::RecordCycleDrained { cycle_id },
        })?;
        if !matches!(receipt.disposition, CommandDisposition::Accepted(_)) {
            return Err(DaemonError::Kernel(StoreError::LedgerCorruption(
                "empty cycle was not drainable after accepted quiescence",
            )));
        }
        Ok(())
    }

    fn status(&self) -> Result<DaemonStatus, DaemonError> {
        let command_count = self.store.command_count()?;
        Ok(match self.mode {
            StartupMode::FreshServing => DaemonStatus::FreshServing { command_count },
            StartupMode::RecoveryFenced => DaemonStatus::RecoveryFenced { command_count },
        })
    }
}

fn daemon_only_capability(capability: Capability) -> bool {
    Capability::KERNEL_SERVICE.contains(&capability)
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let socket_path = self.socket_path();
        let _ = fs::remove_file(socket_path);
    }
}

fn prepare_runtime_root(root: &Path) -> Result<(), DaemonError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) => validate_runtime_root_metadata(&metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(root)?;
            set_mode(root, 0o700)?;
            validate_runtime_root_metadata(&fs::symlink_metadata(root)?)
        }
        Err(error) => Err(DaemonError::Io(error)),
    }
}

fn validate_runtime_root_metadata(metadata: &fs::Metadata) -> Result<(), DaemonError> {
    if !metadata.file_type().is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != effective_uid()
        || metadata.permissions().mode() & 0o777 != 0o700
    {
        return Err(DaemonError::UnsafeRuntimeRoot);
    }
    Ok(())
}

fn validate_runtime_file_if_present(path: &Path) -> Result<(), DaemonError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_runtime_file_metadata(&metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(DaemonError::Io(error)),
    }
}

fn validate_runtime_file_metadata(metadata: &fs::Metadata) -> Result<(), DaemonError> {
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != effective_uid()
        || metadata.permissions().mode() & 0o777 != 0o600
    {
        return Err(DaemonError::UnsafeRuntimeFile);
    }
    Ok(())
}

fn acquire_runtime_lock(root: &Path) -> Result<File, DaemonError> {
    let path = root.join(LOCK_FILE_NAME);
    validate_runtime_file_if_present(&path)?;
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)?;
    validate_runtime_file_metadata(&lock.metadata()?)?;
    // SAFETY: `lock` remains alive in `Daemon` for the entire ownership
    // interval, and the call receives only its valid file descriptor.
    let outcome = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if outcome != 0 {
        let error = io::Error::last_os_error();
        return match error.kind() {
            io::ErrorKind::WouldBlock => Err(DaemonError::AlreadyRunning),
            _ => Err(DaemonError::Io(error)),
        };
    }
    Ok(lock)
}

fn validate_supervisor_stream(stream: &UnixStream) -> Result<(), DaemonError> {
    let descriptor = stream.as_raw_fd();
    let mut socket_type = 0_i32;
    let mut socket_type_length = std::mem::size_of::<i32>() as libc::socklen_t;
    // SAFETY: the stream owns a live descriptor; both output pointers address
    // writable storage of the documented `SO_TYPE` representation.
    if unsafe {
        libc::getsockopt(
            descriptor,
            libc::SOL_SOCKET,
            libc::SO_TYPE,
            (&mut socket_type as *mut i32).cast(),
            &mut socket_type_length,
        )
    } != 0
        || socket_type != libc::SOCK_STREAM
    {
        return Err(DaemonError::InvalidSupervisorStream);
    }
    // SAFETY: zero-initialized `sockaddr_storage` has space for the returned
    // peer address; `getpeername` fills no more than its supplied capacity.
    let mut peer_address: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
    let mut peer_address_length = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
    // SAFETY: the descriptor is a live stream socket and both output pointers
    // reference writable storage whose supplied capacity is exact.
    if unsafe {
        libc::getpeername(
            descriptor,
            (&mut peer_address as *mut libc::sockaddr_storage).cast(),
            &mut peer_address_length,
        )
    } != 0
    {
        return Err(DaemonError::InvalidSupervisorStream);
    }
    // SAFETY: `getpeername` above initialized the leading sockaddr fields in
    // the storage buffer, so reading its family through the ABI prefix is valid.
    let family = unsafe {
        (&peer_address as *const libc::sockaddr_storage)
            .cast::<libc::sockaddr>()
            .read()
            .sa_family
    };
    if family != libc::AF_UNIX as libc::sa_family_t || peer_uid(stream)? != effective_uid() {
        return Err(DaemonError::InvalidSupervisorStream);
    }
    Ok(())
}

fn set_close_on_exec(descriptor: libc::c_int) -> Result<(), DaemonError> {
    // SAFETY: `F_GETFD` reads descriptor flags only and has no pointer input.
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    if flags < 0 {
        return Err(DaemonError::Io(io::Error::last_os_error()));
    }
    // SAFETY: this sets the defined close-on-exec bit on the validated live
    // supervisor descriptor, preventing ordinary child exec inheritance.
    if unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0 {
        return Err(DaemonError::Io(io::Error::last_os_error()));
    }
    Ok(())
}

fn descriptor_has_close_on_exec(descriptor: libc::c_int) -> Result<bool, DaemonError> {
    // SAFETY: `F_GETFD` only inspects flags on the live supervisor descriptor.
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    if flags < 0 {
        return Err(DaemonError::Io(io::Error::last_os_error()));
    }
    Ok(flags & libc::FD_CLOEXEC != 0)
}

fn stream_is_readable(stream: &UnixStream) -> Result<bool, DaemonError> {
    let mut descriptor = libc::pollfd {
        fd: stream.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: `descriptor` is valid writable poll storage for one live fd;
    // zero timeout only checks readiness and never blocks the control loop.
    let outcome = unsafe { libc::poll(&mut descriptor, 1, 0) };
    if outcome < 0 {
        let error = io::Error::last_os_error();
        return if error.kind() == io::ErrorKind::Interrupted {
            Ok(false)
        } else {
            Err(DaemonError::Io(error))
        };
    }
    Ok(outcome > 0 && descriptor.revents & (libc::POLLIN | libc::POLLERR | libc::POLLHUP) != 0)
}

fn remove_stale_socket(path: &Path) -> Result<(), DaemonError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(DaemonError::Io(error)),
    };
    if !metadata.file_type().is_socket() {
        return Err(DaemonError::UnsafeSocketPath);
    }
    fs::remove_file(path)?;
    Ok(())
}

fn set_mode(path: &Path, mode: u32) -> Result<(), DaemonError> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

fn effective_uid() -> libc::uid_t {
    // SAFETY: `geteuid` has no pointer arguments or memory preconditions.
    unsafe { libc::geteuid() }
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd"
))]
fn peer_uid(stream: &UnixStream) -> io::Result<libc::uid_t> {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    // SAFETY: the stream file descriptor is live for this call, and both
    // output pointers reference initialized writable local values.
    let outcome = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    if outcome == 0 {
        Ok(uid)
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "linux")]
fn peer_uid(stream: &UnixStream) -> io::Result<libc::uid_t> {
    let mut credential = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: the socket descriptor is live; `credential` and `length` point
    // to writable storage of the exact `SO_PEERCRED` layout and length.
    let outcome = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credential as *mut libc::ucred).cast(),
            &mut length,
        )
    };
    if outcome == 0 {
        Ok(credential.uid)
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "linux"
)))]
fn peer_uid(_stream: &UnixStream) -> io::Result<libc::uid_t> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "this Unix platform has no supported peer-credential query",
    ))
}
