//! Canonical application-side driver for one resident-owned Pi TaskAttempt.
//!
//! This module owns no provider client, child handle, workspace, or M3
//! identity.  It advances only the public, claim-gated `TaskAttemptRunner`
//! lifecycle.  The daemon still materializes the pinned host/session profile,
//! routes Forum calls, seals transcripts, contains failures, and reconciles
//! native custody.  This is intentionally separate from the paired-study
//! coordinator: it is the physical one-seat execution primitive that a
//! concrete `PilotExecutionBackend` invokes after it has admitted the plan
//! and the Root Authority has provisioned the matching allocation.

use std::{thread, time::{Duration, Instant}};

use society_kernel::{PiCorrelationIdentity, StudyBudgetUnits};
use societyd::{
    ControlWriteDeadline, ControlWriteProgress, Daemon, HandshakeDeadline, MonotonicTick,
    SealedStudyContent, StudyPlanLifetimeError, StudyPlanLifetimeKey, TaskAttemptDisposeEvent,
    TaskAttemptDisposeRequest, TaskAttemptPromptEvent, TaskAttemptPromptRequest,
    TaskAttemptRunnerError, TaskAttemptRunnerOperationId,
};

/// Stable timing policy for one canonical native TaskAttempt.  These limits
/// bound waiting at the process boundary; they are not model sampling or
/// provider retry settings, which remain in the supervisor-installed pinned
/// session profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeTaskDriveLimits {
    handshake_timeout: Duration,
    control_timeout: Duration,
    terminal_timeout: Duration,
    poll_interval: Duration,
}

impl NativeTaskDriveLimits {
    /// Conservative fixed limits for the low-cost CL-001 pilot. A changed
    /// limit is a changed application runner revision, not a CLI override.
    pub const fn canonical() -> Self {
        Self {
            handshake_timeout: Duration::from_secs(30),
            control_timeout: Duration::from_secs(30),
            terminal_timeout: Duration::from_secs(300),
            poll_interval: Duration::from_millis(10),
        }
    }
}

/// Closed failure vocabulary for the application driver.  `TaskAttemptRunner`
/// retains the detailed resident failure and containment state; this outer
/// type prevents the coordinator from mistaking a silent timeout or malformed
/// application prompt for a successful actor output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeTaskDriveError {
    InvalidOperationIdentity,
    InvalidPrompt,
    InvalidCorrelationIdentity,
    StudyPlan(StudyPlanLifetimeError),
    Runner(TaskAttemptRunnerError),
    AdapterHandshakeTimedOut,
    SessionHandshakeTimedOut,
    PromptTerminalTimedOut,
    DisposeTimedOut,
}

impl std::fmt::Display for NativeTaskDriveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidOperationIdentity => {
                formatter.write_str("canonical TaskAttempt operation identity is invalid")
            }
            Self::InvalidPrompt => formatter.write_str("canonical task prompt is not UTF-8"),
            Self::InvalidCorrelationIdentity => {
                formatter.write_str("canonical TaskAttempt correlation identity is invalid")
            }
            Self::StudyPlan(error) => write!(formatter, "sealed study lifetime rejected: {error}"),
            Self::Runner(error) => write!(formatter, "resident TaskAttempt failed: {error}"),
            Self::AdapterHandshakeTimedOut => formatter.write_str("Pi adapter handshake timed out"),
            Self::SessionHandshakeTimedOut => formatter.write_str("Pi session handshake timed out"),
            Self::PromptTerminalTimedOut => formatter.write_str("Pi task prompt terminal timed out"),
            Self::DisposeTimedOut => formatter.write_str("Pi TaskAttempt disposal timed out"),
        }
    }
}

impl std::error::Error for NativeTaskDriveError {}

/// Drives the complete native lifecycle for one already allocated sealed
/// study seat. A construction has no provider side effect; `execute` is the
/// only method which can ask the resident to spawn the pinned Pi host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeTaskAttemptDriver {
    limits: NativeTaskDriveLimits,
}

impl NativeTaskAttemptDriver {
    pub const fn canonical() -> Self {
        Self {
            limits: NativeTaskDriveLimits::canonical(),
        }
    }

    pub const fn limits(self) -> NativeTaskDriveLimits {
        self.limits
    }

    /// Execute one exact sealed task payload and complete its study actor
    /// only after prompt termination, Pi disposal, and native reconciliation.
    /// All Forum calls observed during the prompt are routed back through the
    /// resident runner before execution resumes.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        self,
        daemon: &mut Daemon,
        operation_label: &str,
        lifetime: StudyPlanLifetimeKey,
        prompt_content: SealedStudyContent,
        prompt_bytes: &[u8],
        charged_budget: StudyBudgetUnits,
    ) -> Result<(), NativeTaskDriveError> {
        let operation = TaskAttemptRunnerOperationId::parse(operation_label.to_owned())
            .map_err(|_| NativeTaskDriveError::InvalidOperationIdentity)?;
        let prompt = std::str::from_utf8(prompt_bytes)
            .map_err(|_| NativeTaskDriveError::InvalidPrompt)?
            .to_owned();
        let correlation = PiCorrelationIdentity::parse(format!("{operation_label}-prompt"))
            .map_err(|_| NativeTaskDriveError::InvalidCorrelationIdentity)?;
        let dispose_correlation = PiCorrelationIdentity::parse(format!("{operation_label}-dispose"))
            .map_err(|_| NativeTaskDriveError::InvalidCorrelationIdentity)?;
        let started = Instant::now();
        let mut runner = daemon
            .open_task_attempt_runner_for_study_lifetime(operation.clone(), lifetime)
            .map_err(NativeTaskDriveError::StudyPlan)?;
        let outcome = (|| {
            runner
                .spawn_from_launch_claim()
                .map_err(NativeTaskDriveError::Runner)?;
            runner
                .bind_runtime()
                .map_err(NativeTaskDriveError::Runner)?;

            self.await_adapter_ready(&mut runner, started)?;
            self.deliver_create(&mut runner, started)?;
            self.await_session_ready(&mut runner, started)?;
            self.deliver_prompt(
                &mut runner,
                operation.clone(),
                correlation,
                prompt_content,
                prompt,
                started,
            )?;
            self.await_prompt_terminal(&mut runner, started)?;
            self.dispose_and_reconcile(&mut runner, operation, dispose_correlation, started)?;
            runner
                .complete_actor_obligation(charged_budget)
                .map_err(NativeTaskDriveError::Runner)?;
            Ok(())
        })();
        if outcome.is_err() {
            self.contain(&mut runner, started);
        }
        outcome
    }

    /// Fail closed after a launched task. The resident owns the actual child;
    /// this best-effort bounded driver repeatedly advances its explicit
    /// containment suffix and intentionally never manufactures an actor
    /// completion result if reconciliation cannot be observed.
    fn contain(self, runner: &mut societyd::TaskAttemptRunner<'_>, started: Instant) {
        let containment_deadline = started + self.limits.terminal_timeout;
        while Instant::now() < containment_deadline {
            match runner.drive_containment(tick(started)) {
                Ok(true) => return,
                Ok(false) | Err(_) => pause(self.limits.poll_interval),
            }
        }
    }

    fn await_adapter_ready(
        self,
        runner: &mut societyd::TaskAttemptRunner<'_>,
        started: Instant,
    ) -> Result<(), NativeTaskDriveError> {
        while started.elapsed() < self.limits.handshake_timeout {
            let now = tick(started);
            if runner
                .observe_adapter_ready(now, HandshakeDeadline::at(deadline(now, self.limits.handshake_timeout)))
                .map_err(NativeTaskDriveError::Runner)?
            {
                return Ok(());
            }
            pause(self.limits.poll_interval);
        }
        Err(NativeTaskDriveError::AdapterHandshakeTimedOut)
    }

    fn deliver_create(
        self,
        runner: &mut societyd::TaskAttemptRunner<'_>,
        started: Instant,
    ) -> Result<(), NativeTaskDriveError> {
        let now = tick(started);
        let mut progress = runner
            .begin_create(now, ControlWriteDeadline::at(deadline(now, self.limits.control_timeout)))
            .map_err(NativeTaskDriveError::Runner)?;
        while progress == ControlWriteProgress::Pending {
            if started.elapsed() >= self.limits.control_timeout {
                return Err(NativeTaskDriveError::SessionHandshakeTimedOut);
            }
            pause(self.limits.poll_interval);
            progress = runner.drive_create(tick(started)).map_err(NativeTaskDriveError::Runner)?;
        }
        Ok(())
    }

    fn await_session_ready(
        self,
        runner: &mut societyd::TaskAttemptRunner<'_>,
        started: Instant,
    ) -> Result<(), NativeTaskDriveError> {
        while started.elapsed() < self.limits.handshake_timeout {
            let now = tick(started);
            if runner
                .observe_session_ready(now, HandshakeDeadline::at(deadline(now, self.limits.handshake_timeout)))
                .map_err(NativeTaskDriveError::Runner)?
            {
                return Ok(());
            }
            pause(self.limits.poll_interval);
        }
        Err(NativeTaskDriveError::SessionHandshakeTimedOut)
    }

    fn deliver_prompt(
        self,
        runner: &mut societyd::TaskAttemptRunner<'_>,
        operation: TaskAttemptRunnerOperationId,
        correlation_identity: PiCorrelationIdentity,
        prompt_content: SealedStudyContent,
        prompt: String,
        started: Instant,
    ) -> Result<(), NativeTaskDriveError> {
        let now = tick(started);
        let request = TaskAttemptPromptRequest {
            operation,
            correlation_identity,
            prompt_content_object_id: prompt_content.content_object_id(),
            prompt_digest: prompt_content.digest(),
            prompt,
            frontier_event_id: prompt_content.registration_frontier_event_id(),
        };
        let mut progress = runner
            .begin_prompt(request, now, ControlWriteDeadline::at(deadline(now, self.limits.control_timeout)))
            .map_err(NativeTaskDriveError::Runner)?;
        while progress == ControlWriteProgress::Pending {
            if started.elapsed() >= self.limits.control_timeout {
                return Err(NativeTaskDriveError::PromptTerminalTimedOut);
            }
            pause(self.limits.poll_interval);
            progress = runner.drive_prompt(tick(started)).map_err(NativeTaskDriveError::Runner)?;
        }
        Ok(())
    }

    fn await_prompt_terminal(
        self,
        runner: &mut societyd::TaskAttemptRunner<'_>,
        started: Instant,
    ) -> Result<(), NativeTaskDriveError> {
        while started.elapsed() < self.limits.terminal_timeout {
            match runner.observe_prompt(tick(started)).map_err(NativeTaskDriveError::Runner)? {
                Some(TaskAttemptPromptEvent::ForumToolCall { .. }) => {
                    self.deliver_forum_result(runner, started)?;
                }
                Some(TaskAttemptPromptEvent::TerminalRecorded) => return Ok(()),
                Some(
                    TaskAttemptPromptEvent::ControlInterleaving
                    | TaskAttemptPromptEvent::PromptAccepted
                    | TaskAttemptPromptEvent::KnownUsageRecorded,
                )
                | None => pause(self.limits.poll_interval),
                Some(TaskAttemptPromptEvent::UsageFrozen) => {
                    return Err(NativeTaskDriveError::Runner(
                        TaskAttemptRunnerError::ContainmentRequired,
                    ));
                }
            }
        }
        Err(NativeTaskDriveError::PromptTerminalTimedOut)
    }

    fn deliver_forum_result(
        self,
        runner: &mut societyd::TaskAttemptRunner<'_>,
        started: Instant,
    ) -> Result<(), NativeTaskDriveError> {
        let now = tick(started);
        let mut progress = runner
            .route_forum_tool_call(now, ControlWriteDeadline::at(deadline(now, self.limits.control_timeout)))
            .map_err(NativeTaskDriveError::Runner)?;
        while progress == ControlWriteProgress::Pending {
            if started.elapsed() >= self.limits.terminal_timeout {
                return Err(NativeTaskDriveError::PromptTerminalTimedOut);
            }
            pause(self.limits.poll_interval);
            progress = runner
                .drive_forum_tool_result(tick(started))
                .map_err(NativeTaskDriveError::Runner)?;
        }
        Ok(())
    }

    fn dispose_and_reconcile(
        self,
        runner: &mut societyd::TaskAttemptRunner<'_>,
        operation: TaskAttemptRunnerOperationId,
        correlation_identity: PiCorrelationIdentity,
        started: Instant,
    ) -> Result<(), NativeTaskDriveError> {
        let now = tick(started);
        let request = TaskAttemptDisposeRequest {
            operation,
            correlation_identity,
        };
        let mut progress = runner
            .begin_dispose(request, now, ControlWriteDeadline::at(deadline(now, self.limits.control_timeout)))
            .map_err(NativeTaskDriveError::Runner)?;
        while progress == ControlWriteProgress::Pending {
            if started.elapsed() >= self.limits.control_timeout {
                return Err(NativeTaskDriveError::DisposeTimedOut);
            }
            pause(self.limits.poll_interval);
            progress = runner.drive_dispose(tick(started)).map_err(NativeTaskDriveError::Runner)?;
        }
        let mut disposed = false;
        while started.elapsed() < self.limits.terminal_timeout {
            match runner
                .observe_dispose(
                    tick(started),
                    HandshakeDeadline::at(deadline(tick(started), self.limits.handshake_timeout)),
                )
                .map_err(NativeTaskDriveError::Runner)?
            {
                Some(TaskAttemptDisposeEvent::Disposed) => disposed = true,
                Some(TaskAttemptDisposeEvent::UsageFrozen) => {
                    return Err(NativeTaskDriveError::Runner(
                        TaskAttemptRunnerError::ContainmentRequired,
                    ));
                }
                Some(
                    TaskAttemptDisposeEvent::DeliveryRecorded
                    | TaskAttemptDisposeEvent::Accepted
                    | TaskAttemptDisposeEvent::KnownUsageRecorded,
                )
                | None => pause(self.limits.poll_interval),
            }
            if disposed && runner.reconcile(tick(started)).map_err(NativeTaskDriveError::Runner)? {
                return Ok(());
            }
        }
        Err(NativeTaskDriveError::DisposeTimedOut)
    }
}

fn tick(started: Instant) -> MonotonicTick {
    MonotonicTick::from_milliseconds(
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    )
}

fn deadline(now: MonotonicTick, after: Duration) -> MonotonicTick {
    MonotonicTick::from_milliseconds(
        now.milliseconds()
            .saturating_add(u64::try_from(after.as_millis()).unwrap_or(u64::MAX)),
    )
}

fn pause(duration: Duration) {
    thread::sleep(duration);
}
