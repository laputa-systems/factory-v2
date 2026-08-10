//! Provider-free native-child custody shared by every execution driver.
//!
//! This is deliberately a process-physics core, not a generic execution API.
//! It accepts only a verified executable, a canonical daemon-owned workspace,
//! explicit argv atoms, and an empty v1 environment. It never accepts a shell
//! snippet, a caller-selected working directory, ambient environment, Pi
//! protocol data, or content-writing authority. Pi is an optional strict
//! sidecar over this custody nucleus; deterministic evaluators have no such
//! sidecar. Callers must first obtain a closed kernel admission, then persist
//! spawn and direct-reap facts around this core's side effects. Output stays
//! transient until the direct child has been reaped and a later sealing
//! authority accepts it.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, BufReader, Read},
    os::{fd::AsRawFd, unix::process::CommandExt},
    process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use blake3::Hasher;
use society_pi::{AbsolutePath, Blake3Digest, HostProcessId};

use crate::supervision::{
    MonotonicTick, NativeWorkspace, OwnedProcessGroupId, ProcessGroupLiveness, ReapStatus,
    SignalAction, SignalDelivery, SignalReceipt, SupervisedChildId, SupervisionError,
    TransientByteCount, TransientRetention, VerifiedArtifact,
};

/// OS-only ownership facts shared by every resident child. Protocol sidecars
/// deliberately do not participate in this state: Pi framing and evaluator
/// output treatment can evolve independently without reimplementing PGID
/// custody, liveness probes, or signal delivery races.
#[derive(Debug)]
pub(crate) struct NativeProcessGroup {
    host_process_id: HostProcessId,
    process_group_id: OwnedProcessGroupId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeSignalGroupOutcome {
    AbsentBeforeSignal,
    InaccessibleBeforeSignal,
    AbsentDuringSignal,
    InaccessibleDuringSignal,
    Delivered {
        group_liveness_after_delivery: ProcessGroupLiveness,
    },
}

impl NativeSignalGroupOutcome {
    pub(crate) const fn was_delivered(self) -> bool {
        matches!(self, Self::Delivered { .. })
    }
}

impl NativeProcessGroup {
    pub(crate) const fn new(
        host_process_id: HostProcessId,
        process_group_id: OwnedProcessGroupId,
    ) -> Self {
        Self {
            host_process_id,
            process_group_id,
        }
    }

    pub(crate) const fn host_process_id(&self) -> HostProcessId {
        self.host_process_id
    }

    pub(crate) const fn process_group_id(&self) -> OwnedProcessGroupId {
        self.process_group_id
    }

    pub(crate) fn owns_expected_group(&self) -> Result<bool, SupervisionError> {
        // SAFETY: `getpgid` observes the PID registered directly from the
        // successful spawn; it neither signals nor mutates process state.
        let observed = unsafe { libc::getpgid(self.process_group_id.value()) };
        if observed < 0 {
            return match io::Error::last_os_error().raw_os_error() {
                Some(libc::ESRCH) => Ok(false),
                _ => Err(SupervisionError::ProcessGroup(io::Error::last_os_error())),
            };
        }
        Ok(observed == self.process_group_id.value())
    }

    pub(crate) fn liveness(&self) -> Result<ProcessGroupLiveness, SupervisionError> {
        // SAFETY: signal zero probes only the PGID created by this child.
        if unsafe { libc::kill(-self.process_group_id.value(), 0) } == 0 {
            return Ok(ProcessGroupLiveness::Present);
        }
        match io::Error::last_os_error().raw_os_error() {
            Some(libc::ESRCH) => Ok(ProcessGroupLiveness::Absent),
            Some(libc::EPERM) => Ok(ProcessGroupLiveness::Inaccessible),
            _ => Err(SupervisionError::ProcessGroup(io::Error::last_os_error())),
        }
    }

    pub(crate) fn signal(
        &self,
        signal: libc::c_int,
    ) -> Result<NativeSignalGroupOutcome, SupervisionError> {
        match self.liveness()? {
            ProcessGroupLiveness::Absent => {
                return Ok(NativeSignalGroupOutcome::AbsentBeforeSignal);
            }
            ProcessGroupLiveness::Inaccessible => {
                return Ok(NativeSignalGroupOutcome::InaccessibleBeforeSignal);
            }
            ProcessGroupLiveness::Present => {}
        }
        // SAFETY: the negative PGID addresses only the dedicated group.
        if unsafe { libc::kill(-self.process_group_id.value(), signal) } != 0 {
            return match io::Error::last_os_error().raw_os_error() {
                Some(libc::ESRCH) => Ok(NativeSignalGroupOutcome::AbsentDuringSignal),
                Some(libc::EPERM) => Ok(NativeSignalGroupOutcome::InaccessibleDuringSignal),
                _ => Err(SupervisionError::ProcessGroup(io::Error::last_os_error())),
            };
        }
        Ok(NativeSignalGroupOutcome::Delivered {
            group_liveness_after_delivery: self.liveness()?,
        })
    }
}

/// The fixed transient cap is a containment boundary, not an output quota
/// which an evaluator may negotiate. The digest always covers every observed
/// byte, while retained bytes are strictly bounded.
pub(crate) const MAX_NATIVE_CHILD_STREAM_BYTES: usize = 64 * 1024;

/// This profile intentionally has no relationship to the Pi process double
/// or a paid/live profile. Adding another native execution treatment requires
/// a new closed variant and a matching kernel admission contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeChildProfile {
    DeterministicEvaluatorDirectExecutableV1,
}

/// The native core has no inherited environment. A future allowlist must be a
/// new closed variant rather than a map supplied by a scheduler or evaluator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeChildEnvironment {
    EmptyV1,
}

/// One argv atom. It cannot contain a NUL and is never interpreted by a
/// shell: [`Command`] receives it as one direct exec argument.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeChildArgument(String);

impl NativeChildArgument {
    pub fn literal(value: impl Into<String>) -> Result<Self, SupervisionError> {
        let value = value.into();
        if value.is_empty() || value.len() > 4096 || value.contains('\0') {
            return Err(SupervisionError::InvalidNativeChildRequest);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An exact native executable plus all artefacts which define its treatment.
/// The `required_artifacts` list exists to bind an evaluator/script/input
/// launcher identity to the same pre-exec TOCTOU check as the executable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeChildExecution {
    program: VerifiedArtifact,
    required_artifacts: Vec<VerifiedArtifact>,
    argv: Vec<NativeChildArgument>,
    profile: NativeChildProfile,
}

impl NativeChildExecution {
    /// The v1 evaluator artifact is itself the direct executable. The core
    /// constructs the entire ABI: no launcher, script path, `-c`, or caller
    /// argv can be smuggled across this boundary.
    pub(crate) fn direct_evaluator(
        evaluator_artifact: VerifiedArtifact,
        input_manifest_artifact: VerifiedArtifact,
    ) -> Result<Self, SupervisionError> {
        Ok(Self {
            program: evaluator_artifact,
            required_artifacts: vec![input_manifest_artifact.clone()],
            argv: vec![
                NativeChildArgument::literal("--input-manifest")?,
                NativeChildArgument::literal(input_manifest_artifact.path().as_str())?,
            ],
            profile: NativeChildProfile::DeterministicEvaluatorDirectExecutableV1,
        })
    }

    #[cfg(test)]
    fn direct_test_fixture(executable: VerifiedArtifact, argv: Vec<NativeChildArgument>) -> Self {
        Self {
            program: executable,
            required_artifacts: Vec::new(),
            argv,
            profile: NativeChildProfile::DeterministicEvaluatorDirectExecutableV1,
        }
    }

    fn verify_before_exec(&self) -> Result<(), SupervisionError> {
        self.program.verify_current_identity()?;
        for artifact in &self.required_artifacts {
            artifact.verify_current_identity()?;
        }
        Ok(())
    }
}

/// Inputs selected by a trusted daemon driver after its kernel admission. No
/// generic paths or environment are present at this boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeChildSpawnRequest {
    pub child_process_id: SupervisedChildId,
    pub workspace: NativeWorkspace,
    pub execution: NativeChildExecution,
    pub environment: NativeChildEnvironment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeChildSpawnFacts {
    pub child_process_id: SupervisedChildId,
    pub host_process_id: HostProcessId,
    pub process_group_id: OwnedProcessGroupId,
    pub workspace_directory: AbsolutePath,
    pub profile: NativeChildProfile,
}

/// A child becomes a durable containment subject as soon as exec succeeds.
/// Pipe setup failures therefore return its exact registered identities rather
/// than hiding them behind a bare error which would invite orphaning it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeChildPostSpawnSetupFailure {
    MissingStdout,
    StdoutNonblocking,
    MissingStderr,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NativeChildSpawnOutcome {
    Ready(NativeChildSpawnFacts),
    RegisteredSetupFailure {
        facts: NativeChildSpawnFacts,
        failure: NativeChildPostSpawnSetupFailure,
    },
}

/// Fixed escalation points supplied as monotonic control-loop coordinates.
/// The core never sleeps, so tests and the resident may drive exact races.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeChildDeadline {
    terminate_at: MonotonicTick,
    kill_at: MonotonicTick,
}

impl NativeChildDeadline {
    pub const fn new(terminate_at: MonotonicTick, kill_at: MonotonicTick) -> Self {
        Self {
            terminate_at,
            kill_at,
        }
    }

    pub const fn terminate_at(self) -> MonotonicTick {
        self.terminate_at
    }

    pub const fn kill_at(self) -> MonotonicTick {
        self.kill_at
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeChildDirectReapFacts {
    pub child_process_id: SupervisedChildId,
    pub host_process_id: HostProcessId,
    pub process_group_id: OwnedProcessGroupId,
    pub status: ReapStatus,
    pub group_liveness_after_direct_child_reap: ProcessGroupLiveness,
    pub prior_signal_receipts: Vec<SignalReceipt>,
}

/// Transient bytes remain physically owned by the daemon. A sealing authority
/// may consume these only after [`NativeChildSupervisor::complete_deferred_reap`]
/// returned this receipt; it must not infer an output object from a running
/// child or a pipe prefix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeChildStreamCapture {
    pub observed_byte_count: TransientByteCount,
    pub blake3: Blake3Digest,
    pub retention: TransientRetention,
    retained_bytes: Vec<u8>,
}

impl NativeChildStreamCapture {
    pub fn retained_bytes(&self) -> &[u8] {
        &self.retained_bytes
    }

    pub const fn blake3(&self) -> &Blake3Digest {
        &self.blake3
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeChildReceipt {
    pub child_process_id: SupervisedChildId,
    pub host_process_id: HostProcessId,
    pub process_group_id: OwnedProcessGroupId,
    pub profile: NativeChildProfile,
    pub workspace_directory: AbsolutePath,
    pub status: ReapStatus,
    pub group_liveness_before_cleanup: ProcessGroupLiveness,
    pub group_liveness_after_cleanup: ProcessGroupLiveness,
    pub signal_receipts: Vec<SignalReceipt>,
    pub stdout: NativeChildStreamCapture,
    pub stderr: NativeChildStreamCapture,
}

/// Single-owner registry for native, non-Pi children. It has no automatic
/// restart or generic execute method: an evaluator driver owns its closed
/// admission/evidence sequence around this small process-custody core.
pub(crate) struct NativeChildSupervisor {
    children: BTreeMap<SupervisedChildId, ManagedNativeChild>,
    historical_child_ids: BTreeSet<SupervisedChildId>,
}

/// The generic native ownership nucleus. Pi attaches a strict protocol
/// sidecar after this exact direct child has been registered; deterministic
/// evaluators attach no protocol sidecar at all.
pub(crate) struct OwnedNativeChildProcess {
    pub(crate) child: Child,
    pub(crate) native_process: NativeProcessGroup,
    direct_wait: Option<ExitStatus>,
}

impl OwnedNativeChildProcess {
    pub(crate) fn poll_direct_wait(&mut self) -> Result<Option<ExitStatus>, SupervisionError> {
        if let Some(status) = self.direct_wait {
            return Ok(Some(status));
        }
        let status = self.child.try_wait()?;
        self.direct_wait = status;
        Ok(status)
    }

    pub(crate) fn wait_direct(&mut self) -> Result<ExitStatus, SupervisionError> {
        if let Some(status) = self.direct_wait {
            return Ok(status);
        }
        let status = self.child.wait()?;
        self.direct_wait = Some(status);
        Ok(status)
    }
}

/// Creates the one owned direct process group used by every resident child.
/// Callers configure only stdio/argv/cwd on `command`; this function alone
/// owns the pre-exec group transition and PID-to-PGID derivation.
pub(crate) fn spawn_owned_native_child(
    mut command: Command,
) -> Result<OwnedNativeChildProcess, SupervisionError> {
    // SAFETY: this runs in the forked child immediately before exec and calls
    // only async-signal-safe `setpgid(0, 0)`.
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().map_err(SupervisionError::NativeSpawn)?;
    let host_process_id = match HostProcessId::parse(u64::from(child.id())) {
        Ok(value) => value,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(SupervisionError::Protocol(error));
        }
    };
    let process_group_id = match OwnedProcessGroupId::from_host_process_id(host_process_id) {
        Ok(value) => value,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    Ok(OwnedNativeChildProcess {
        child,
        native_process: NativeProcessGroup::new(host_process_id, process_group_id),
        direct_wait: None,
    })
}

impl Default for NativeChildSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeChildSupervisor {
    pub fn new() -> Self {
        Self {
            children: BTreeMap::new(),
            historical_child_ids: BTreeSet::new(),
        }
    }

    /// Side-effect-free validation performed before a driver admits native
    /// work to the kernel. Spawn repeats all artifact checks immediately
    /// before exec to close the observation-to-exec gap.
    pub fn preflight_spawn(
        &self,
        request: &NativeChildSpawnRequest,
    ) -> Result<(), SupervisionError> {
        if self
            .historical_child_ids
            .contains(&request.child_process_id)
        {
            return Err(SupervisionError::DuplicateChildIdentity);
        }
        if request.execution.argv.is_empty()
            || request.environment != NativeChildEnvironment::EmptyV1
            || request.execution.profile
                != NativeChildProfile::DeterministicEvaluatorDirectExecutableV1
        {
            return Err(SupervisionError::InvalidNativeChildRequest);
        }
        let canonical = fs::canonicalize(request.workspace.directory().as_path())?;
        if canonical != request.workspace.directory().as_path()
            || !fs::metadata(&canonical)?.is_dir()
        {
            return Err(SupervisionError::InvalidNativeChildRequest);
        }
        request.execution.verify_before_exec()
    }

    /// Spawns exactly one direct child in its own process group. The returned
    /// facts exist before any output observation; callers persist them before
    /// driving deadlines, signals, reap, or content sealing.
    pub fn spawn(
        &mut self,
        request: NativeChildSpawnRequest,
    ) -> Result<NativeChildSpawnOutcome, SupervisionError> {
        self.preflight_spawn(&request)?;
        request.execution.verify_before_exec()?;
        let mut command = Command::new(request.execution.program.path().as_path());
        command
            .args(
                request
                    .execution
                    .argv
                    .iter()
                    .map(NativeChildArgument::as_str),
            )
            .current_dir(request.workspace.directory().as_path())
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut native_child = spawn_owned_native_child(command)?;
        let host_process_id = native_child.native_process.host_process_id();
        let process_group_id = native_child.native_process.process_group_id();
        // Registration precedes every fallible pipe operation so a setup
        // failure cannot orphan this direct PID/PGID.
        let stdout = native_child.child.stdout.take().map(BufReader::new);
        let stderr = native_child.child.stderr.take().map(spawn_stderr_capture);
        let child_process_id = request.child_process_id.clone();
        let facts = NativeChildSpawnFacts {
            child_process_id: child_process_id.clone(),
            host_process_id,
            process_group_id,
            workspace_directory: request.workspace.directory().clone(),
            profile: request.execution.profile,
        };
        self.historical_child_ids.insert(child_process_id.clone());
        self.children.insert(
            child_process_id,
            ManagedNativeChild {
                request,
                native_child,
                stdout,
                stderr,
                stdout_capture: StreamCapture::default(),
                signals: Vec::new(),
                term_sent: false,
                kill_sent: false,
                pending_reap: None,
                completed: None,
            },
        );
        let failure = if self.child_mut(&facts.child_process_id)?.stdout.is_none() {
            Some(NativeChildPostSpawnSetupFailure::MissingStdout)
        } else {
            let nonblocking_failed = {
                let child = self.child_mut(&facts.child_process_id)?;
                let stdout = child
                    .stdout
                    .as_ref()
                    .ok_or(SupervisionError::MissingNativeChildStdout)?;
                set_nonblocking(stdout.get_ref().as_raw_fd()).is_err()
            };
            if nonblocking_failed {
                Some(NativeChildPostSpawnSetupFailure::StdoutNonblocking)
            } else if self.child_mut(&facts.child_process_id)?.stderr.is_none() {
                Some(NativeChildPostSpawnSetupFailure::MissingStderr)
            } else {
                None
            }
        };
        if let Some(failure) = failure {
            let _ = self
                .child_mut(&facts.child_process_id)
                .and_then(|managed| managed.send_kill(MonotonicTick::ZERO));
            return Ok(NativeChildSpawnOutcome::RegisteredSetupFailure { facts, failure });
        }
        Ok(NativeChildSpawnOutcome::Ready(facts))
    }

    /// Polls nonblocking output and applies the fixed deadline escalation. An
    /// output cap breach starts TERM containment before returning its error;
    /// the caller must continue driving/reaping the registered child.
    pub fn drive_at(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
        deadline: NativeChildDeadline,
    ) -> Result<(), SupervisionError> {
        let child = self.child_mut(child_process_id)?;
        if child.pending_reap.is_some() || child.completed.is_some() {
            return Ok(());
        }
        if child.drain_stdout_nonblocking()? || child.stderr_exceeded()? {
            child.send_term(now)?;
            return Err(SupervisionError::NativeChildOutputLimitExceeded);
        }
        if now >= deadline.terminate_at() {
            child.send_term(now)?;
        }
        if now >= deadline.kill_at() {
            child.send_kill(now)?;
        }
        Ok(())
    }

    /// Explicit cancellation is indistinguishable from deadline containment
    /// at the OS boundary, but still produces a separate exact signal receipt.
    pub fn request_cancellation_at(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
    ) -> Result<(), SupervisionError> {
        self.child_mut(child_process_id)?.send_term(now)
    }

    /// Collects only the direct child. If descendants remain, this retains a
    /// distinct pre-cleanup fact; call `issue_lingering_group_cleanup` only
    /// after making it durable.
    pub fn poll_direct_child_reap(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<Option<NativeChildDirectReapFacts>, SupervisionError> {
        self.child_mut(child_process_id)?.poll_direct_reap()
    }

    pub fn issue_lingering_group_cleanup(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
    ) -> Result<Option<SignalReceipt>, SupervisionError> {
        self.child_mut(child_process_id)?
            .issue_lingering_cleanup(now)
    }

    /// Produces a transient receipt only once the direct wait fact was
    /// recorded and the owned process group is absent. This order prevents an
    /// escaped descendant's pipe from blocking the daemon before containment.
    pub fn complete_deferred_reap(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<NativeChildReceipt, SupervisionError> {
        self.child_mut(child_process_id)?.complete_deferred_reap()
    }

    /// Observes only the liveness of the PID-derived process group which this
    /// supervisor already owns. The caller remains responsible for recording
    /// the resulting exact kernel receipt before relying on it for closure.
    pub fn observe_group_liveness(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<ProcessGroupLiveness, SupervisionError> {
        self.child_mut(child_process_id)?.group_liveness()
    }

    /// Returns the exact signal-attempt history accumulated so far for one
    /// owned child. The history remains retained for the final native receipt;
    /// callers use their own projected-count cursor so a kernel receipt can
    /// be committed promptly without erasing the post-reap audit shape.
    pub fn signal_receipts(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<Vec<SignalReceipt>, SupervisionError> {
        Ok(self.child_mut(child_process_id)?.signals.clone())
    }

    fn child_mut(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<&mut ManagedNativeChild, SupervisionError> {
        self.children
            .get_mut(child_process_id)
            .ok_or(SupervisionError::InvalidLifecycle)
    }
}

struct ManagedNativeChild {
    request: NativeChildSpawnRequest,
    native_child: OwnedNativeChildProcess,
    stdout: Option<BufReader<ChildStdout>>,
    stderr: Option<StderrCaptureTask>,
    stdout_capture: StreamCapture,
    signals: Vec<SignalReceipt>,
    term_sent: bool,
    kill_sent: bool,
    pending_reap: Option<PendingReap>,
    completed: Option<NativeChildReceipt>,
}

struct PendingReap {
    status: ExitStatus,
    group_liveness: ProcessGroupLiveness,
}

impl ManagedNativeChild {
    fn drain_stdout_nonblocking(&mut self) -> Result<bool, SupervisionError> {
        let mut buffer = [0_u8; 8192];
        loop {
            let stdout = self
                .stdout
                .as_mut()
                .ok_or(SupervisionError::MissingNativeChildStdout)?;
            match stdout.read(&mut buffer) {
                Ok(0) => return Ok(self.stdout_capture.exceeded),
                Ok(count) => {
                    self.stdout_capture.observe(&buffer[..count]);
                    if self.stdout_capture.exceeded {
                        return Ok(true);
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    return Ok(self.stdout_capture.exceeded);
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn stderr_exceeded(&self) -> Result<bool, SupervisionError> {
        self.stderr
            .as_ref()
            .ok_or(SupervisionError::MissingNativeChildStderr)?
            .exceeded()
    }

    fn group_liveness(&self) -> Result<ProcessGroupLiveness, SupervisionError> {
        self.native_child.native_process.liveness()
    }

    fn signal_group(
        &self,
        signal: libc::c_int,
    ) -> Result<(SignalDelivery, ProcessGroupLiveness), SupervisionError> {
        Ok(match self.native_child.native_process.signal(signal)? {
            NativeSignalGroupOutcome::AbsentBeforeSignal => (
                SignalDelivery::AbsentBeforeSignal,
                ProcessGroupLiveness::Absent,
            ),
            NativeSignalGroupOutcome::InaccessibleBeforeSignal
            | NativeSignalGroupOutcome::InaccessibleDuringSignal => (
                SignalDelivery::GroupInaccessible,
                ProcessGroupLiveness::Inaccessible,
            ),
            NativeSignalGroupOutcome::AbsentDuringSignal => (
                SignalDelivery::AbsentDuringSignal,
                ProcessGroupLiveness::Absent,
            ),
            NativeSignalGroupOutcome::Delivered {
                group_liveness_after_delivery,
            } => (SignalDelivery::TermSent, group_liveness_after_delivery),
        })
    }

    fn send_term(&mut self, now: MonotonicTick) -> Result<(), SupervisionError> {
        if self.term_sent {
            return Ok(());
        }
        let (delivery, liveness) = self.signal_group(libc::SIGTERM)?;
        self.signals.push(SignalReceipt {
            action: SignalAction::Terminate,
            delivery,
            observed_at: now,
            group_liveness_after_attempt: liveness,
        });
        self.term_sent = true;
        Ok(())
    }

    fn send_kill(&mut self, now: MonotonicTick) -> Result<(), SupervisionError> {
        if self.kill_sent {
            return Ok(());
        }
        let (delivery, liveness) = self.signal_group(libc::SIGKILL)?;
        let delivery = match delivery {
            SignalDelivery::TermSent => SignalDelivery::KillSent,
            other => other,
        };
        self.signals.push(SignalReceipt {
            action: SignalAction::Kill,
            delivery,
            observed_at: now,
            group_liveness_after_attempt: liveness,
        });
        self.kill_sent = true;
        Ok(())
    }

    fn poll_direct_reap(&mut self) -> Result<Option<NativeChildDirectReapFacts>, SupervisionError> {
        if let Some(pending) = &self.pending_reap {
            return Ok(Some(self.reap_facts(pending)));
        }
        if self.completed.is_some() {
            return Err(SupervisionError::InvalidLifecycle);
        }
        let Some(status) = self.native_child.poll_direct_wait()? else {
            return Ok(None);
        };
        let group_liveness = self.group_liveness()?;
        let facts = NativeChildDirectReapFacts {
            child_process_id: self.request.child_process_id.clone(),
            host_process_id: self.native_child.native_process.host_process_id(),
            process_group_id: self.native_child.native_process.process_group_id(),
            status: reap_status(status),
            group_liveness_after_direct_child_reap: group_liveness,
            prior_signal_receipts: self.signals.clone(),
        };
        self.pending_reap = Some(PendingReap {
            status,
            group_liveness,
        });
        Ok(Some(facts))
    }

    fn reap_facts(&self, pending: &PendingReap) -> NativeChildDirectReapFacts {
        NativeChildDirectReapFacts {
            child_process_id: self.request.child_process_id.clone(),
            host_process_id: self.native_child.native_process.host_process_id(),
            process_group_id: self.native_child.native_process.process_group_id(),
            status: reap_status(pending.status),
            group_liveness_after_direct_child_reap: pending.group_liveness,
            prior_signal_receipts: self.signals.clone(),
        }
    }

    fn issue_lingering_cleanup(
        &mut self,
        now: MonotonicTick,
    ) -> Result<Option<SignalReceipt>, SupervisionError> {
        let pending = self
            .pending_reap
            .as_ref()
            .ok_or(SupervisionError::InvalidLifecycle)?;
        if pending.group_liveness == ProcessGroupLiveness::Absent {
            return Ok(None);
        }
        if self
            .signals
            .iter()
            .any(|receipt| receipt.action == SignalAction::LingeringGroupKill)
        {
            return Err(SupervisionError::InvalidLifecycle);
        }
        let (delivery, liveness) = self.signal_group(libc::SIGKILL)?;
        let delivery = match delivery {
            SignalDelivery::TermSent => SignalDelivery::LingeringGroupKillSent,
            other => other,
        };
        let receipt = SignalReceipt {
            action: SignalAction::LingeringGroupKill,
            delivery,
            observed_at: now,
            group_liveness_after_attempt: liveness,
        };
        self.signals.push(receipt.clone());
        Ok(Some(receipt))
    }

    fn complete_deferred_reap(&mut self) -> Result<NativeChildReceipt, SupervisionError> {
        if let Some(receipt) = &self.completed {
            return Ok(receipt.clone());
        }
        let pending = self
            .pending_reap
            .take()
            .ok_or(SupervisionError::InvalidLifecycle)?;
        let after = self.group_liveness()?;
        if after != ProcessGroupLiveness::Absent {
            self.pending_reap = Some(pending);
            return Err(SupervisionError::ContainmentAwaitingDrive);
        }
        let exceeded = self.drain_stdout_nonblocking()?;
        let stdout = self.stdout_capture.capture(if exceeded {
            TransientRetention::PrefixBounded
        } else {
            self.stdout_capture.retention
        });
        let stderr = self
            .stderr
            .take()
            .ok_or(SupervisionError::NativeChildOutputCaptureFailed)?
            .snapshot()?;
        let receipt = NativeChildReceipt {
            child_process_id: self.request.child_process_id.clone(),
            host_process_id: self.native_child.native_process.host_process_id(),
            process_group_id: self.native_child.native_process.process_group_id(),
            profile: self.request.execution.profile,
            workspace_directory: self.request.workspace.directory().clone(),
            status: reap_status(pending.status),
            group_liveness_before_cleanup: pending.group_liveness,
            group_liveness_after_cleanup: after,
            signal_receipts: self.signals.clone(),
            stdout,
            stderr,
        };
        self.completed = Some(receipt.clone());
        Ok(receipt)
    }
}

impl Drop for ManagedNativeChild {
    fn drop(&mut self) {
        if self.completed.is_some() {
            return;
        }
        // SAFETY: while the `Child` handle is unreaped its PID cannot be
        // recycled; the negative PGID is the group created for that child.
        let _ = self.native_child.native_process.signal(libc::SIGKILL);
        let _ = self.native_child.wait_direct();
    }
}

struct StderrCaptureTask {
    snapshot: Arc<Mutex<StreamCapture>>,
    worker: JoinHandle<io::Result<()>>,
}

fn spawn_stderr_capture(mut stderr: ChildStderr) -> StderrCaptureTask {
    let snapshot = Arc::new(Mutex::new(StreamCapture::default()));
    let shared = Arc::clone(&snapshot);
    let worker = thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            let count = stderr.read(&mut buffer)?;
            if count == 0 {
                return Ok(());
            }
            shared
                .lock()
                .map_err(|_| io::Error::other("stderr capture mutex poisoned"))?
                .observe(&buffer[..count]);
        }
    });
    StderrCaptureTask { snapshot, worker }
}

impl StderrCaptureTask {
    fn exceeded(&self) -> Result<bool, SupervisionError> {
        self.snapshot
            .lock()
            .map_err(|_| SupervisionError::NativeChildOutputCaptureFailed)
            .map(|value| value.exceeded)
    }

    fn snapshot(self) -> Result<NativeChildStreamCapture, SupervisionError> {
        // `complete_deferred_reap` calls this only after the owned group was
        // observed absent. The pipe reader can therefore reach EOF and join
        // without issuing a race-prone incomplete final digest.
        let Self { snapshot, worker } = self;
        worker
            .join()
            .map_err(|_| SupervisionError::NativeChildOutputCaptureFailed)??;
        let capture = snapshot
            .lock()
            .map_err(|_| SupervisionError::NativeChildOutputCaptureFailed)?;
        Ok(capture.capture(capture.retention))
    }
}

#[derive(Clone, Default)]
struct StreamCapture {
    observed: u64,
    overflowed: bool,
    hasher: Hasher,
    retained: Vec<u8>,
    retention: TransientRetention,
    exceeded: bool,
}

impl StreamCapture {
    fn observe(&mut self, bytes: &[u8]) {
        match self.observed.checked_add(bytes.len() as u64) {
            Some(value) if !self.overflowed => self.observed = value,
            _ => {
                self.overflowed = true;
                self.retention = TransientRetention::CountOverflow;
            }
        }
        self.hasher.update(bytes);
        let remaining = MAX_NATIVE_CHILD_STREAM_BYTES.saturating_sub(self.retained.len());
        let retained = remaining.min(bytes.len());
        self.retained.extend_from_slice(&bytes[..retained]);
        if retained != bytes.len() {
            self.retention = TransientRetention::PrefixBounded;
            self.exceeded = true;
        }
    }
    fn capture(&self, retention: TransientRetention) -> NativeChildStreamCapture {
        NativeChildStreamCapture {
            observed_byte_count: if self.overflowed {
                TransientByteCount::Overflowed
            } else {
                TransientByteCount::Exact(self.observed)
            },
            blake3: digest_to_type(self.hasher.clone().finalize().as_bytes()),
            retention,
            retained_bytes: self.retained.clone(),
        }
    }
}

fn set_nonblocking(file_descriptor: libc::c_int) -> Result<(), SupervisionError> {
    // SAFETY: changes status flags on the daemon-owned pipe descriptor only.
    let flags = unsafe { libc::fcntl(file_descriptor, libc::F_GETFL) };
    if flags < 0 {
        return Err(SupervisionError::Io(io::Error::last_os_error()));
    }
    // SAFETY: same owned descriptor; O_NONBLOCK is the only added flag.
    if unsafe { libc::fcntl(file_descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } != 0 {
        return Err(SupervisionError::Io(io::Error::last_os_error()));
    }
    Ok(())
}

fn reap_status(status: ExitStatus) -> ReapStatus {
    use std::os::unix::process::ExitStatusExt;
    match (status.code(), status.signal()) {
        (Some(code), _) => ReapStatus::Exited { code },
        (None, Some(signal)) => ReapStatus::Signaled { signal },
        (None, None) => ReapStatus::Unknown,
    }
}

fn digest_to_type(digest: impl AsRef<[u8]>) -> Blake3Digest {
    let mut output = String::with_capacity(64);
    for byte in digest.as_ref() {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    Blake3Digest::parse(output).expect("BLAKE3 formatter produces lowercase hex")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::Path,
        sync::atomic::{AtomicU64, Ordering},
        thread,
        time::Duration,
    };

    use super::*;
    use crate::supervision::{NativeWorkspaceId, NativeWorkspaceRoot};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn evaluator_core_binds_verified_argv_and_releases_capture_only_after_direct_reap() {
        let fixture = Fixture::new("exact-exit");
        let request = fixture.request("exact-exit");
        let child_id = request.child_process_id.clone();
        let mut supervisor = NativeChildSupervisor::new();
        let outcome = supervisor.spawn(request).unwrap();
        assert!(matches!(outcome, NativeChildSpawnOutcome::Ready(_)));

        let deadline = NativeChildDeadline::new(
            MonotonicTick::from_milliseconds(10_000),
            MonotonicTick::from_milliseconds(11_000),
        );
        let direct = loop {
            supervisor
                .drive_at(&child_id, MonotonicTick::ZERO, deadline)
                .unwrap();
            if let Some(facts) = supervisor.poll_direct_child_reap(&child_id).unwrap() {
                break facts;
            }
            thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(direct.status, ReapStatus::Exited { code: 7 });
        // A durable direct-wait fact is retry-stable: the second poll must
        // return the original status rather than attempt a second `wait(2)`.
        assert_eq!(
            supervisor
                .poll_direct_child_reap(&child_id)
                .unwrap()
                .unwrap()
                .status,
            direct.status
        );
        if direct.group_liveness_after_direct_child_reap != ProcessGroupLiveness::Absent {
            supervisor
                .issue_lingering_group_cleanup(&child_id, MonotonicTick::ZERO)
                .unwrap();
        }
        let receipt = supervisor.complete_deferred_reap(&child_id).unwrap();
        assert_eq!(receipt.status, ReapStatus::Exited { code: 7 });
        assert_eq!(
            receipt.profile,
            NativeChildProfile::DeterministicEvaluatorDirectExecutableV1
        );
        assert!(
            receipt
                .stdout
                .retained_bytes()
                .windows(b"deterministic-output\n".len())
                .any(|bytes| bytes == b"deterministic-output\n")
        );
        fixture.cleanup();
    }

    #[test]
    fn either_output_cap_starts_containment_without_a_pi_sidecar() {
        for mode in ["cap-stdout", "cap-stderr"] {
            let fixture = Fixture::new(mode);
            let request = fixture.request(mode);
            let child_id = request.child_process_id.clone();
            let mut supervisor = NativeChildSupervisor::new();
            assert!(matches!(
                supervisor.spawn(request).unwrap(),
                NativeChildSpawnOutcome::Ready(_)
            ));
            let deadline = NativeChildDeadline::new(
                MonotonicTick::from_milliseconds(10_000),
                MonotonicTick::from_milliseconds(11_000),
            );
            let mut capped = false;
            for _ in 0..1_000 {
                match supervisor.drive_at(&child_id, MonotonicTick::ZERO, deadline) {
                    Err(SupervisionError::NativeChildOutputLimitExceeded) => {
                        capped = true;
                        break;
                    }
                    Ok(()) => thread::sleep(Duration::from_millis(1)),
                    other => panic!("unexpected native-child drive result: {other:?}"),
                }
            }
            assert!(
                capped,
                "bounded {mode} capture must contain oversized output"
            );
            fixture.cleanup();
        }
    }

    #[test]
    fn native_child_fixture_child() {
        let mode = std::env::args().find_map(|argument| {
            argument
                .strip_prefix("native-child-fixture=")
                .map(str::to_owned)
        });
        match mode.as_deref() {
            Some("exact-exit") => {
                println!("deterministic-output");
                std::process::exit(7);
            }
            Some("cap-stdout") => {
                for _ in 0..70_000 {
                    print!("x");
                }
            }
            Some("cap-stderr") => {
                for _ in 0..70_000 {
                    eprint!("x");
                }
            }
            Some(other) => panic!("unknown native child fixture mode: {other}"),
            None => {}
        }
    }

    struct Fixture {
        root: std::path::PathBuf,
        workspace: NativeWorkspace,
        program: VerifiedArtifact,
        evaluator: VerifiedArtifact,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let unique = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!("societyd-native-child-{label}-{unique}"));
            fs::create_dir(&root).unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
            let workspace = NativeWorkspaceRoot::open_owned(&root)
                .unwrap()
                .allocate(NativeWorkspaceId::parse("evaluator-workspace").unwrap())
                .unwrap();
            let program = verified(std::env::current_exe().unwrap());
            let evaluator = program.clone();
            Self {
                root,
                workspace,
                program,
                evaluator,
            }
        }

        fn request(&self, mode: &str) -> NativeChildSpawnRequest {
            NativeChildSpawnRequest {
                child_process_id: SupervisedChildId::parse(format!("native-{mode}")).unwrap(),
                workspace: self.workspace.clone(),
                execution: NativeChildExecution::direct_test_fixture(
                    self.evaluator.clone(),
                    vec![
                        NativeChildArgument::literal("--exact").unwrap(),
                        NativeChildArgument::literal(
                            "native_child::tests::native_child_fixture_child",
                        )
                        .unwrap(),
                        NativeChildArgument::literal("--nocapture").unwrap(),
                        NativeChildArgument::literal("--").unwrap(),
                        NativeChildArgument::literal(format!("native-child-fixture={mode}"))
                            .unwrap(),
                    ],
                ),
                environment: NativeChildEnvironment::EmptyV1,
            }
        }

        fn cleanup(self) {
            let _ = fs::remove_dir_all(self.root);
        }
    }

    fn verified(path: impl AsRef<Path>) -> VerifiedArtifact {
        let bytes = fs::read(path.as_ref()).unwrap();
        VerifiedArtifact::inspect(path, digest_to_type(blake3::hash(&bytes).as_bytes())).unwrap()
    }
}
