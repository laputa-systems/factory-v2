//! Stateful validation of one host process's sealed protocol stream.
//!
//! This is intentionally a peer rather than a scheduler: callers admit only
//! already-authorized commands, give it exact process identity evidence, and
//! consume normalized deltas.  Any protocol ambiguity fences this peer; no
//! later frame can turn a failed execution back into a normal one.

use std::collections::{BTreeMap, BTreeSet};

use miniserde::json::Value;
use thiserror::Error;

use crate::{
    cost::{CostDecodeError, UsageDelta, UsageTracker},
    protocol::{
        AdapterFailureCode, AssistantStopReason, Blake3Digest, CommandName, CommandResult,
        CorrelationIdentity, CreateSessionPayload, EffectiveSessionConfiguration,
        FinalAssistantOutcome, HostProcessId, InboundCommand, InboundFrame, MAX_JSONL_FRAME_BYTES,
        OutboundEvent, OutboundFrame, ProjectedAgentEvent, PromptPurpose, ProtocolError,
        RuntimeIdentity, SessionIdentity, SessionKind, SpawnNonce, ToolCallIdentity,
        TranscriptFlushReceiptV1, UsageObservation, UsageUnavailableReason, decode_inbound_jsonl,
        decode_outbound_jsonl,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PeerPhase {
    AwaitingAdapterReady,
    Inert,
    Creating,
    Ready,
    Closing,
    Disposed,
    Fatal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TurnDisposition {
    Pending,
    Completed,
    Length,
    Error,
    Aborted,
    Failed,
    ProtocolFailed,
}

/// A caller-visible, normalized terminal or accounting fact.  The daemon may
/// persist these facts, but this boundary crate never makes a durable charge.
#[derive(Clone, Debug)]
pub enum PeerObservation {
    Usage(UsageDelta),
    UsageUnavailable {
        reason: UsageUnavailableReason,
    },
    TurnSettled(TurnReceipt),
    /// A Forum call is a peer-validated request whose JSON arguments remain
    /// at the SDK-host boundary. The resident runtime must translate it into
    /// a typed Forum transition before returning a result to the host.
    ForumToolCall {
        correlation_identity: CorrelationIdentity,
        tool_call_identity: ToolCallIdentity,
        tool_name: crate::forum::ForumToolName,
        args: Value,
    },
    Disposed,
    Fatal {
        failure_code: AdapterFailureCode,
    },
}

impl PartialEq for PeerObservation {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Usage(left), Self::Usage(right)) => left == right,
            (Self::UsageUnavailable { reason: left }, Self::UsageUnavailable { reason: right }) => {
                left == right
            }
            (Self::TurnSettled(left), Self::TurnSettled(right)) => left == right,
            (
                Self::ForumToolCall {
                    correlation_identity: left_correlation,
                    tool_call_identity: left_call,
                    tool_name: left_name,
                    args: left_args,
                },
                Self::ForumToolCall {
                    correlation_identity: right_correlation,
                    tool_call_identity: right_call,
                    tool_name: right_name,
                    args: right_args,
                },
            ) => {
                left_correlation == right_correlation
                    && left_call == right_call
                    && left_name == right_name
                    && miniserde::json::to_string(left_args)
                        == miniserde::json::to_string(right_args)
            }
            (Self::Disposed, Self::Disposed) => true,
            (
                Self::Fatal { failure_code: left },
                Self::Fatal {
                    failure_code: right,
                },
            ) => left == right,
            _ => false,
        }
    }
}

impl Eq for PeerObservation {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnReceipt {
    pub correlation_identity: CorrelationIdentity,
    pub disposition: TurnDisposition,
    pub final_assistant_outcome: FinalAssistantOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedLine {
    pub blake3: Blake3Digest,
}
impl SealedLine {
    /// Seals the exact raw bytes observed at the pipe boundary, before UTF-8
    /// decoding. This preserves malformed evidence rather than sealing a lossy
    /// replacement string.
    pub fn of_bytes(line: &[u8]) -> Self {
        Self {
            blake3: digest(line),
        }
    }

    pub fn of_utf8(line: &str) -> Self {
        Self::of_bytes(line.as_bytes())
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum PeerError {
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error(transparent)]
    Cost(#[from] CostDecodeError),
    #[error("inbound sequence gap or duplicate")]
    InboundSequence,
    #[error("outbound sequence gap or duplicate")]
    OutboundSequence,
    #[error("session identity does not match the supervised process")]
    SessionIdentity,
    #[error("the supervised host's spawn nonce or runtime identity drifted")]
    RuntimeIdentity,
    #[error("correlation was not an admitted command")]
    UnknownCorrelation,
    #[error("a command received more than one result")]
    DuplicateCommandResult,
    #[error("the host result command differs from the admitted command")]
    ResultCommandMismatch,
    #[error("a frame is invalid in this closed session state")]
    InvalidTransition,
    #[error("effective Pi configuration differs from the admitted profile")]
    ExecutionProfileDrift,
    #[error("transcript receipt is not the admitted session/prompt evidence")]
    TranscriptReceipt,
    #[error("a prompt lacks required terminal Pi evidence")]
    MissingTerminalEvidence,
    #[error("a protocol-fatal host cannot resume")]
    Fatal,
}

#[derive(Clone, Debug)]
struct PendingCommand {
    name: CommandName,
    result_seen: bool,
    accepted: bool,
}

/// An admitted Prompt has reached the host's FIFO but has not yet been
/// acknowledged. It is already an execution-bearing operation: no second
/// prompt or disposal may pass it, and an immediately following Abort must
/// attach to this exact prompt rather than disappear into a Ready-state gap.
#[derive(Clone, Debug)]
struct PendingPrompt {
    correlation: CorrelationIdentity,
    abort_intent_admitted: bool,
}

/// A Dispose has been admitted to the host's FIFO. The host transitions to
/// closing before its acknowledgement; allowing any later command through the
/// peer during that gap would fabricate a second control history.
#[derive(Clone, Debug)]
struct PendingDispose {
    correlation: CorrelationIdentity,
}

/// A peer-accepted Pi `Dispose` is a short, closed terminal chain rather
/// than an arbitrary Closing phase: accepted command result, exactly one
/// forced cumulative usage snapshot, then the transcript receipt. Keeping
/// this state separately from `PendingDispose` lets the boundary reject a
/// schema-valid `Disposed` record that skipped final accounting.
#[derive(Clone, Debug)]
struct ActiveDispose {
    correlation: CorrelationIdentity,
    accepted_sequence: u64,
    final_usage_sequence: Option<u64>,
}

/// The terminal evidence path Pi 0.83 actually exposes for an admitted
/// Prompt. Non-lifecycle projected events remain sealed evidence, but the
/// lifecycle facts that can certify a charged outcome are strictly ordered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TurnEvidencePhase {
    AwaitingAgentStart,
    ActiveAttempt,
    /// A retrying `agent_end` was observed. Pi's current stream may either
    /// announce the next attempt with `agent_start` or emit its final end
    /// directly (the latter is exercised by the pinned host fixture).
    RetryLifecycle,
    AwaitingAgentSettled {
        final_stop_reason: AssistantStopReason,
    },
    AwaitingFinalPromptUsage {
        final_stop_reason: AssistantStopReason,
        agent_settled_sequence: u64,
    },
    AwaitingSettledFrame {
        final_stop_reason: AssistantStopReason,
        agent_settled_sequence: u64,
        final_usage_sequence: u64,
    },
}

#[derive(Clone, Debug)]
struct ActiveTurn {
    correlation: CorrelationIdentity,
    evidence_phase: TurnEvidencePhase,
    latest_prompt_usage_sequence: Option<u64>,
    /// The host mutates its abort intent *before* awaiting `session.abort()`.
    /// Rust records the matching admitted command rather than waiting for its
    /// later result, which may legitimately arrive after Prompt settlement.
    abort_intent_admitted: bool,
}

/// A closed, per-child process peer. `expected_runtime` comes from the
/// supervisor's pinned Node/build/lock admission; the host merely echoes it.
#[derive(Clone, Debug)]
pub struct BoundaryPeer {
    session_identity: SessionIdentity,
    expected_process_id: HostProcessId,
    expected_spawn_nonce: SpawnNonce,
    expected_runtime: RuntimeIdentity,
    phase: PeerPhase,
    expected_inbound_sequence: u64,
    expected_outbound_sequence: u64,
    adapter_ready_seen: bool,
    fatal_frame_seen: bool,
    pending_commands: BTreeMap<CorrelationIdentity, PendingCommand>,
    completed_correlations: BTreeSet<CorrelationIdentity>,
    create: Option<CreateSessionPayload>,
    create_candidate: Option<(CorrelationIdentity, CreateSessionPayload)>,
    configuration: Option<EffectiveSessionConfiguration>,
    pending_prompt: Option<PendingPrompt>,
    pending_forum_tool_calls: BTreeSet<ToolCallIdentity>,
    pending_dispose: Option<PendingDispose>,
    active_dispose: Option<ActiveDispose>,
    active_turn: Option<ActiveTurn>,
    task_attempt_prompt_admitted: bool,
    prompt_candidates: BTreeMap<CorrelationIdentity, Blake3Digest>,
    first_prompt_digest: Option<Blake3Digest>,
    settled_turns: Vec<TurnReceipt>,
    usage: UsageTracker,
    inbound_seals: Vec<SealedLine>,
    outbound_seals: Vec<SealedLine>,
}

impl BoundaryPeer {
    pub fn new(
        session_identity: SessionIdentity,
        expected_process_id: HostProcessId,
        expected_spawn_nonce: SpawnNonce,
        expected_runtime: RuntimeIdentity,
    ) -> Result<Self, PeerError> {
        expected_runtime.assert_v1()?;
        Ok(Self {
            session_identity,
            expected_process_id,
            expected_spawn_nonce,
            expected_runtime,
            phase: PeerPhase::AwaitingAdapterReady,
            expected_inbound_sequence: 1,
            expected_outbound_sequence: 1,
            adapter_ready_seen: false,
            fatal_frame_seen: false,
            pending_commands: BTreeMap::new(),
            completed_correlations: BTreeSet::new(),
            create: None,
            create_candidate: None,
            configuration: None,
            pending_prompt: None,
            pending_forum_tool_calls: BTreeSet::new(),
            pending_dispose: None,
            active_dispose: None,
            active_turn: None,
            task_attempt_prompt_admitted: false,
            prompt_candidates: BTreeMap::new(),
            first_prompt_digest: None,
            settled_turns: Vec::new(),
            usage: UsageTracker::default(),
            inbound_seals: Vec::new(),
            outbound_seals: Vec::new(),
        })
    }

    pub const fn phase(&self) -> PeerPhase {
        self.phase
    }
    pub fn inbound_seals(&self) -> &[SealedLine] {
        &self.inbound_seals
    }
    pub fn outbound_seals(&self) -> &[SealedLine] {
        &self.outbound_seals
    }
    pub fn configuration(&self) -> Option<&EffectiveSessionConfiguration> {
        self.configuration.as_ref()
    }
    pub fn latest_usage(&self) -> Option<&crate::cost::NormalizedUsage> {
        self.usage.latest()
    }
    pub fn settled_turns(&self) -> &[TurnReceipt] {
        &self.settled_turns
    }

    /// Parse, seal, and register a Rust-to-host command before writing it to
    /// stdin. A caller must only write the returned decoded command if this
    /// admission succeeds; that pairing gives command/result correlations one
    /// meaning across reconnect-free child ownership.
    pub fn admit_inbound_jsonl(&mut self, line: &str) -> Result<InboundFrame, PeerError> {
        self.admit_inbound_jsonl_bytes(line.as_bytes())
    }

    /// Byte-oriented stdin admission is the authority-facing API. JSONL
    /// decoding never gets a chance to replace invalid bytes before their
    /// digest is recorded and the peer is fenced.
    pub fn admit_inbound_jsonl_bytes(&mut self, line: &[u8]) -> Result<InboundFrame, PeerError> {
        self.inbound_seals.push(SealedLine::of_bytes(line));
        let decoded = raw_jsonl_utf8(line);

        let line = match decoded {
            Ok(line) => line,
            Err(error) => {
                self.fence();
                return Err(error.into());
            }
        };
        let frame = match decode_inbound_jsonl(line) {
            Ok(frame) => frame,
            Err(error) => {
                self.fence();
                return Err(error.into());
            }
        };
        self.admit_inbound(frame.clone())?;
        Ok(frame)
    }

    pub fn admit_inbound(&mut self, frame: InboundFrame) -> Result<(), PeerError> {
        let result = self.admit_inbound_inner(frame);
        if result.is_err() {
            self.fence();
        }
        result
    }

    fn admit_inbound_inner(&mut self, frame: InboundFrame) -> Result<(), PeerError> {
        if self.phase == PeerPhase::Fatal {
            return Err(PeerError::Fatal);
        }
        if frame.session_identity != self.session_identity {
            self.fence();
            return Err(PeerError::SessionIdentity);
        }
        if frame.sequence.value() != self.expected_inbound_sequence {
            self.fence();
            return Err(PeerError::InboundSequence);
        }
        self.expected_inbound_sequence += 1;
        if self
            .pending_commands
            .contains_key(&frame.correlation_identity)
            || self
                .completed_correlations
                .contains(&frame.correlation_identity)
        {
            self.fence();
            return Err(PeerError::DuplicateCommandResult);
        }
        self.validate_command_admission(&frame)?;
        match &frame.command {
            InboundCommand::CreateSession(payload) => {
                self.create_candidate =
                    Some((frame.correlation_identity.clone(), (**payload).clone()))
            }
            InboundCommand::Prompt(payload) => {
                self.prompt_candidates.insert(
                    frame.correlation_identity.clone(),
                    digest(payload.text.as_bytes()),
                );
                self.pending_prompt = Some(PendingPrompt {
                    correlation: frame.correlation_identity.clone(),
                    abort_intent_admitted: false,
                });
            }
            InboundCommand::Abort(_) => {
                if let Some(active) = self.active_turn.as_mut() {
                    if matches!(
                        active.evidence_phase,
                        TurnEvidencePhase::AwaitingAgentStart
                            | TurnEvidencePhase::ActiveAttempt
                            | TurnEvidencePhase::RetryLifecycle
                    ) {
                        active.abort_intent_admitted = true;
                    }
                } else if let Some(pending) = self.pending_prompt.as_mut() {
                    pending.abort_intent_admitted = true;
                }
            }
            InboundCommand::Dispose(_) => {
                self.pending_dispose = Some(PendingDispose {
                    correlation: frame.correlation_identity.clone(),
                });
            }
            _ => {}
        }
        self.pending_commands.insert(
            frame.correlation_identity,
            PendingCommand {
                name: frame.command.name(),
                result_seen: false,
                accepted: false,
            },
        );
        Ok(())
    }

    /// Parse, seal, and normalize a host stdout frame. The returned usage delta
    /// is transient and idempotent; persistent charging remains outside this
    /// crate.
    pub fn observe_outbound_jsonl(
        &mut self,
        line: &str,
    ) -> Result<Option<PeerObservation>, PeerError> {
        self.observe_outbound_jsonl_bytes(line.as_bytes())
    }

    /// Byte-oriented stdout observation seals malformed host records exactly
    /// as written, including invalid UTF-8, before it closes the peer.
    pub fn observe_outbound_jsonl_bytes(
        &mut self,
        line: &[u8],
    ) -> Result<Option<PeerObservation>, PeerError> {
        self.outbound_seals.push(SealedLine::of_bytes(line));
        let line = match raw_jsonl_utf8(line) {
            Ok(line) => line,
            Err(error) => {
                self.fence();
                return Err(error.into());
            }
        };
        let frame = match decode_outbound_jsonl(line) {
            Ok(frame) => frame,
            Err(error) => {
                self.fence();
                return Err(error.into());
            }
        };
        self.observe_outbound(frame)
    }

    pub fn observe_outbound(
        &mut self,
        frame: OutboundFrame,
    ) -> Result<Option<PeerObservation>, PeerError> {
        let result = self.observe_outbound_inner(frame);
        if result.is_err() {
            self.fence();
        }
        result
    }

    fn observe_outbound_inner(
        &mut self,
        frame: OutboundFrame,
    ) -> Result<Option<PeerObservation>, PeerError> {
        let terminal_fatal_frame = matches!(frame.event, OutboundEvent::Fatal { .. });
        if self.phase == PeerPhase::Fatal && !terminal_fatal_frame {
            return Err(PeerError::Fatal);
        }
        if frame.session_identity != self.session_identity {
            self.fence();
            return Err(PeerError::SessionIdentity);
        }
        if frame.sequence.value() != self.expected_outbound_sequence {
            self.fence();
            return Err(PeerError::OutboundSequence);
        }
        self.expected_outbound_sequence += 1;
        let frame_sequence = frame.sequence.value();
        let result = match frame.event {
            OutboundEvent::AdapterReady {
                pid,
                spawn_nonce,
                runtime,
                ..
            } => {
                if self.adapter_ready_seen
                    || self.phase != PeerPhase::AwaitingAdapterReady
                    || pid != self.expected_process_id
                    || spawn_nonce != self.expected_spawn_nonce
                    || runtime != self.expected_runtime
                {
                    self.fence();
                    return Err(PeerError::RuntimeIdentity);
                }
                runtime.assert_v1()?;
                self.adapter_ready_seen = true;
                self.phase = PeerPhase::Inert;
                None
            }
            OutboundEvent::SessionReady { configuration } => {
                self.observe_session_ready(frame.correlation_identity, configuration)?;
                None
            }
            OutboundEvent::CommandResult(result) => {
                self.observe_command_result(frame_sequence, frame.correlation_identity, result)?;
                None
            }
            OutboundEvent::AgentEvent { agent_event } => {
                self.observe_agent_event(frame_sequence, frame.correlation_identity, agent_event)?;
                None
            }
            OutboundEvent::ForumToolCall {
                tool_call_identity,
                tool_name,
                args,
            } => {
                let Some(correlation_identity) = frame.correlation_identity.clone() else {
                    self.fence();
                    return Err(PeerError::UnknownCorrelation);
                };
                self.observe_forum_tool_call(
                    Some(correlation_identity.clone()),
                    tool_call_identity.clone(),
                    tool_name,
                    &args,
                )?;
                Some(PeerObservation::ForumToolCall {
                    correlation_identity,
                    tool_call_identity,
                    tool_name,
                    args,
                })
            }
            OutboundEvent::UsageSnapshot { usage } => {
                self.observe_usage(frame_sequence, frame.correlation_identity, usage)?
            }
            OutboundEvent::Settled {
                classification,
                final_assistant_outcome,
            } => Some(PeerObservation::TurnSettled(self.observe_settled(
                frame_sequence,
                frame.correlation_identity,
                classification,
                final_assistant_outcome,
            )?)),
            OutboundEvent::Disposed {
                transcript_flush_receipt,
            } => {
                self.observe_disposed(
                    frame_sequence,
                    frame.correlation_identity,
                    transcript_flush_receipt,
                )?;
                Some(PeerObservation::Disposed)
            }
            OutboundEvent::Fatal { failure_code } => {
                if self.fatal_frame_seen {
                    return Err(PeerError::Fatal);
                }
                self.fatal_frame_seen = true;
                self.fence();
                Some(PeerObservation::Fatal { failure_code })
            }
        };
        Ok(result)
    }

    /// EOF is abnormal host loss until an evidenced `Disposed` frame. An inert
    /// host or a zero process exit is not a normal peer terminal fact.
    pub fn observe_stdout_eof(&mut self) -> Result<(), PeerError> {
        if !matches!(self.phase, PeerPhase::Disposed | PeerPhase::Fatal) {
            self.fence();
            return Err(PeerError::MissingTerminalEvidence);
        }
        Ok(())
    }

    fn validate_command_admission(&mut self, frame: &InboundFrame) -> Result<(), PeerError> {
        if self.pending_dispose.is_some() {
            return Err(PeerError::InvalidTransition);
        }
        match &frame.command {
            InboundCommand::CreateSession(payload) => {
                if !self.adapter_ready_seen
                    || self.phase != PeerPhase::Inert
                    || self.create.is_some()
                    || self.create_candidate.is_some()
                {
                    self.fence();
                    return Err(PeerError::InvalidTransition);
                }
                if !payload
                    .auth_path
                    .is_strict_descendant_of(&payload.agent_directory)
                    || !payload
                        .models_path
                        .is_strict_descendant_of(&payload.agent_directory)
                    || digest(payload.system_prompt.as_bytes()) != payload.system_prompt_digest
                    || payload.model.provider != crate::protocol::Provider::OpenRouter
                    || payload.model.model_id != payload.model_catalog.effective_model.model_id
                    || !crate::protocol::model_thinking_level_is_admitted(
                        payload.model.model_id,
                        payload.model.thinking_level,
                    )
                {
                    self.fence();
                    return Err(PeerError::ExecutionProfileDrift);
                }
                payload.model_catalog.assert_pinned()?;
                payload.settings.assert_pinned()?;
                payload.forum_contract.assert_pinned()?;
            }
            InboundCommand::Prompt(payload) => {
                let Some(create) = self.create.as_ref() else {
                    self.fence();
                    return Err(PeerError::InvalidTransition);
                };
                if self.phase != PeerPhase::Ready
                    || self.active_turn.is_some()
                    || self.pending_prompt.is_some()
                    || !matches!(
                        (create.session_kind, payload.purpose),
                        (SessionKind::TaskAttempt, PromptPurpose::TaskAssignment)
                            | (SessionKind::RootAuthorityOffice, PromptPurpose::OfficeTurn)
                    )
                    || create.session_kind == SessionKind::TaskAttempt
                        && self.task_attempt_prompt_admitted
                {
                    self.fence();
                    return Err(PeerError::InvalidTransition);
                }
                if create.session_kind == SessionKind::TaskAttempt {
                    self.task_attempt_prompt_admitted = true;
                }
            }
            InboundCommand::FollowUp(_) | InboundCommand::Steer(_) => {
                if self.phase != PeerPhase::Ready
                    || !self.active_turn.as_ref().is_some_and(|turn| {
                        matches!(
                            turn.evidence_phase,
                            TurnEvidencePhase::ActiveAttempt | TurnEvidencePhase::RetryLifecycle
                        )
                    })
                    || self.create.as_ref().map(|create| create.session_kind)
                        != Some(SessionKind::RootAuthorityOffice)
                {
                    self.fence();
                    return Err(PeerError::InvalidTransition);
                }
            }
            InboundCommand::ForumToolResult(payload) => {
                if self.phase != PeerPhase::Ready
                    || self.active_turn.is_none()
                    || !self
                        .pending_forum_tool_calls
                        .remove(&payload.tool_call_identity)
                {
                    self.fence();
                    return Err(PeerError::InvalidTransition);
                }
            }
            InboundCommand::Abort(_) | InboundCommand::GetState => {
                if self.phase != PeerPhase::Ready {
                    self.fence();
                    return Err(PeerError::InvalidTransition);
                }
            }
            InboundCommand::Dispose(_) => {
                if self.phase != PeerPhase::Ready
                    || self.pending_prompt.is_some()
                    || self.active_turn.is_some()
                {
                    self.fence();
                    return Err(PeerError::InvalidTransition);
                }
            }
        }
        Ok(())
    }

    fn observe_command_result(
        &mut self,
        sequence: u64,
        correlation: Option<CorrelationIdentity>,
        result: CommandResult,
    ) -> Result<(), PeerError> {
        let Some(correlation) = correlation else {
            self.fence();
            return Err(PeerError::UnknownCorrelation);
        };
        let Some(pending) = self.pending_commands.get_mut(&correlation) else {
            self.fence();
            return Err(PeerError::UnknownCorrelation);
        };
        if pending.result_seen {
            self.fence();
            return Err(PeerError::DuplicateCommandResult);
        }
        let result_name = match &result {
            CommandResult::Accepted { command, .. } | CommandResult::Rejected { command, .. } => {
                *command
            }
        };
        if pending.name != result_name {
            self.fence();
            return Err(PeerError::ResultCommandMismatch);
        }
        pending.result_seen = true;
        self.completed_correlations.insert(correlation.clone());
        let accepted = matches!(result, CommandResult::Accepted { .. });
        pending.accepted = accepted;
        if accepted {
            match pending.name {
                CommandName::CreateSession => {
                    let Some((candidate_correlation, payload)) = self.create_candidate.take()
                    else {
                        self.fence();
                        return Err(PeerError::InvalidTransition);
                    };
                    if candidate_correlation != correlation {
                        self.fence();
                        return Err(PeerError::InvalidTransition);
                    }
                    self.create = Some(payload);
                    self.phase = PeerPhase::Creating;
                }
                CommandName::Prompt => {
                    let Some(pending_prompt) = self.pending_prompt.take() else {
                        return Err(PeerError::InvalidTransition);
                    };
                    if pending_prompt.correlation != correlation {
                        return Err(PeerError::InvalidTransition);
                    }
                    if self.first_prompt_digest.is_none() {
                        self.first_prompt_digest =
                            self.prompt_candidates.get(&correlation).cloned();
                    }
                    self.active_turn = Some(ActiveTurn {
                        correlation: correlation.clone(),
                        evidence_phase: TurnEvidencePhase::AwaitingAgentStart,
                        latest_prompt_usage_sequence: None,
                        abort_intent_admitted: pending_prompt.abort_intent_admitted,
                    });
                }
                CommandName::Dispose => {
                    self.phase = PeerPhase::Closing;
                    self.active_dispose = Some(ActiveDispose {
                        correlation: correlation.clone(),
                        accepted_sequence: sequence,
                        final_usage_sequence: None,
                    });
                }
                CommandName::FollowUp
                | CommandName::Steer
                | CommandName::Abort
                | CommandName::GetState
                | CommandName::ForumToolResult => {}
            }
        }
        if !accepted && pending.name == CommandName::Prompt {
            let Some(pending_prompt) = self.pending_prompt.take() else {
                return Err(PeerError::InvalidTransition);
            };
            if pending_prompt.correlation != correlation {
                return Err(PeerError::InvalidTransition);
            }
        }
        if accepted && pending.name == CommandName::Dispose {
            let Some(pending_dispose) = self.pending_dispose.take() else {
                return Err(PeerError::InvalidTransition);
            };
            if pending_dispose.correlation != correlation {
                return Err(PeerError::InvalidTransition);
            }
        }
        if !accepted && pending.name == CommandName::Dispose {
            return Err(PeerError::InvalidTransition);
        }
        Ok(())
    }

    fn observe_session_ready(
        &mut self,
        correlation: Option<CorrelationIdentity>,
        configuration: EffectiveSessionConfiguration,
    ) -> Result<(), PeerError> {
        let Some(correlation) = correlation else {
            self.fence();
            return Err(PeerError::UnknownCorrelation);
        };
        let pending = self
            .pending_commands
            .get(&correlation)
            .ok_or(PeerError::UnknownCorrelation)?;
        if pending.name != CommandName::CreateSession
            || !pending.result_seen
            || self.phase != PeerPhase::Creating
        {
            self.fence();
            return Err(PeerError::InvalidTransition);
        }
        let Some(create) = self.create.as_ref() else {
            // `create` is recorded after the inbound's matching acceptance;
            // this can only be a malicious/out-of-order host frame.
            self.fence();
            return Err(PeerError::ExecutionProfileDrift);
        };
        configuration.assert_pinned()?;
        if configuration.session_kind != create.session_kind
            || configuration.cwd != create.cwd
            || configuration.session_directory != create.session_directory
            || configuration.model != create.model
            || configuration.model_catalog != create.model_catalog
            || configuration.tool_profile != create.tool_profile
            || configuration.settings != create.settings
            || configuration.forum_contract != create.forum_contract
            || !configuration
                .session_file
                .is_strict_descendant_of(&create.session_directory)
        {
            self.fence();
            return Err(PeerError::ExecutionProfileDrift);
        }
        self.configuration = Some(configuration);
        self.phase = PeerPhase::Ready;
        Ok(())
    }

    fn observe_agent_event(
        &mut self,
        sequence: u64,
        correlation: Option<CorrelationIdentity>,
        event: ProjectedAgentEvent,
    ) -> Result<(), PeerError> {
        if !matches!(self.phase, PeerPhase::Ready | PeerPhase::Closing) {
            self.fence();
            return Err(PeerError::InvalidTransition);
        }
        if let Some(active) = self.active_turn.as_mut() {
            if correlation.as_ref() != Some(&active.correlation) {
                self.fence();
                return Err(PeerError::UnknownCorrelation);
            }
            observe_turn_evidence(&mut active.evidence_phase, event, sequence)?;
        } else if correlation.is_some() {
            self.fence();
            return Err(PeerError::UnknownCorrelation);
        }
        Ok(())
    }

    fn observe_forum_tool_call(
        &mut self,
        correlation: Option<CorrelationIdentity>,
        tool_call_identity: ToolCallIdentity,
        tool_name: crate::forum::ForumToolName,
        _args: &Value,
    ) -> Result<(), PeerError> {
        let Some(active) = self.active_turn.as_ref() else {
            self.fence();
            return Err(PeerError::InvalidTransition);
        };
        let tool_profile_is_forum = self.configuration.as_ref().is_some_and(|configuration| {
            configuration.tool_profile == crate::protocol::ToolProfile::ForumIsolatedV1
                && configuration.tools.as_slice()
                    == [
                        crate::protocol::PiToolName::SocietyForumRead,
                        crate::protocol::PiToolName::SocietyForumPost,
                    ]
        });
        if correlation.as_ref() != Some(&active.correlation)
            || !tool_profile_is_forum
            || !self.pending_forum_tool_calls.insert(tool_call_identity)
        {
            self.fence();
            return Err(PeerError::InvalidTransition);
        }
        match tool_name {
            crate::forum::ForumToolName::SocietyForumRead
            | crate::forum::ForumToolName::SocietyForumPost => Ok(()),
        }
    }

    fn observe_usage(
        &mut self,
        sequence: u64,
        correlation: Option<CorrelationIdentity>,
        usage: UsageObservation,
    ) -> Result<Option<PeerObservation>, PeerError> {
        let Some(correlation) = correlation else {
            self.fence();
            return Err(PeerError::UnknownCorrelation);
        };
        let Some(command) = self.pending_commands.get(&correlation) else {
            self.fence();
            return Err(PeerError::UnknownCorrelation);
        };
        if !command.result_seen || !command.accepted {
            self.fence();
            return Err(PeerError::InvalidTransition);
        }
        let is_final_dispose_usage = if let Some(active_dispose) = self.active_dispose.as_ref() {
            if self.phase != PeerPhase::Closing
                || correlation != active_dispose.correlation
                || command.name != CommandName::Dispose
                || active_dispose.final_usage_sequence.is_some()
                || active_dispose.accepted_sequence.checked_add(1) != Some(sequence)
            {
                self.fence();
                return Err(PeerError::MissingTerminalEvidence);
            }
            true
        } else {
            false
        };
        let active_prompt_correlation = self
            .active_turn
            .as_ref()
            .map(|turn| turn.correlation.clone());
        if active_prompt_correlation.is_some() {
            if !matches!(
                command.name,
                CommandName::Prompt
                    | CommandName::FollowUp
                    | CommandName::Steer
                    | CommandName::Abort
                    | CommandName::GetState
            ) {
                self.fence();
                return Err(PeerError::InvalidTransition);
            }
        } else if !matches!(
            command.name,
            CommandName::Abort | CommandName::GetState | CommandName::Dispose
        ) {
            self.fence();
            return Err(PeerError::InvalidTransition);
        }
        if let UsageObservation::Unavailable(reason) = usage {
            self.fence();
            return Ok(Some(PeerObservation::UsageUnavailable { reason }));
        }
        match self.usage.observe(&usage) {
            Ok(delta) => {
                if is_final_dispose_usage {
                    // A host-forced same-total snapshot produces an explicit
                    // idempotent zero delta. Its sequence remains mandatory
                    // terminal accounting evidence and must not be dropped
                    // merely because cumulative charge did not change.
                    let Some(active_dispose) = self.active_dispose.as_mut() else {
                        self.fence();
                        return Err(PeerError::MissingTerminalEvidence);
                    };
                    active_dispose.final_usage_sequence = Some(sequence);
                }
                if let Some(active) = self.active_turn.as_mut()
                    && correlation == active.correlation
                {
                    match active.evidence_phase {
                        TurnEvidencePhase::AwaitingFinalPromptUsage {
                            final_stop_reason,
                            agent_settled_sequence,
                        } => {
                            active.evidence_phase = TurnEvidencePhase::AwaitingSettledFrame {
                                final_stop_reason,
                                agent_settled_sequence,
                                final_usage_sequence: sequence,
                            };
                        }
                        TurnEvidencePhase::AwaitingSettledFrame { .. } => {
                            return Err(PeerError::MissingTerminalEvidence);
                        }
                        _ => {}
                    }
                    active.latest_prompt_usage_sequence = Some(sequence);
                }
                Ok(delta.map(PeerObservation::Usage))
            }
            Err(error) => {
                self.fence();
                Err(error.into())
            }
        }
    }

    fn observe_settled(
        &mut self,
        sequence: u64,
        correlation: Option<CorrelationIdentity>,
        classification: crate::protocol::SettledClassification,
        outcome: FinalAssistantOutcome,
    ) -> Result<TurnReceipt, PeerError> {
        let Some(correlation) = correlation else {
            self.fence();
            return Err(PeerError::UnknownCorrelation);
        };
        let Some(active) = self.active_turn.as_ref() else {
            self.fence();
            return Err(PeerError::InvalidTransition);
        };
        if correlation != active.correlation {
            self.fence();
            return Err(PeerError::UnknownCorrelation);
        }
        let disposition = match (classification, &outcome) {
            (
                crate::protocol::SettledClassification::Completed,
                FinalAssistantOutcome::Observed {
                    stop_reason: AssistantStopReason::Stop,
                },
            ) => TurnDisposition::Completed,
            (
                crate::protocol::SettledClassification::Length,
                FinalAssistantOutcome::Observed {
                    stop_reason: AssistantStopReason::Length,
                },
            ) => TurnDisposition::Length,
            (
                crate::protocol::SettledClassification::Error,
                FinalAssistantOutcome::Observed {
                    stop_reason: AssistantStopReason::Error,
                },
            ) => TurnDisposition::Error,
            (
                crate::protocol::SettledClassification::Aborted,
                FinalAssistantOutcome::Observed {
                    stop_reason: AssistantStopReason::Aborted,
                },
            ) => TurnDisposition::Aborted,
            (
                crate::protocol::SettledClassification::Aborted,
                FinalAssistantOutcome::Observed {
                    stop_reason: AssistantStopReason::Stop,
                },
            ) if active.abort_intent_admitted => TurnDisposition::Aborted,
            (
                crate::protocol::SettledClassification::Failed,
                FinalAssistantOutcome::Unavailable {
                    reason: crate::protocol::FinalAssistantUnavailableReason::SdkPromiseRejected,
                },
            ) => TurnDisposition::Failed,
            (
                crate::protocol::SettledClassification::ProtocolFailed,
                FinalAssistantOutcome::Unavailable {
                    reason:
                        crate::protocol::FinalAssistantUnavailableReason::MissingFinalAssistantOutcome,
                },
            ) => TurnDisposition::ProtocolFailed,
            _ => {
                self.fence();
                return Err(PeerError::MissingTerminalEvidence);
            }
        };
        let final_usage_is_adjacent = active
            .latest_prompt_usage_sequence
            .is_some_and(|usage| usage.checked_add(1) == Some(sequence));
        let observed_evidence_is_closed = match (&outcome, active.evidence_phase) {
            (
                FinalAssistantOutcome::Observed { stop_reason },
                TurnEvidencePhase::AwaitingSettledFrame {
                    final_stop_reason,
                    agent_settled_sequence,
                    final_usage_sequence,
                },
            ) => {
                final_stop_reason == *stop_reason
                    && agent_settled_sequence < final_usage_sequence
                    && final_usage_sequence.checked_add(1) == Some(sequence)
            }
            _ => false,
        };
        if (matches!(&outcome, FinalAssistantOutcome::Observed { .. })
            && (!observed_evidence_is_closed || !final_usage_is_adjacent))
            || (matches!(&outcome, FinalAssistantOutcome::Unavailable { .. })
                && !final_usage_is_adjacent)
        {
            return Err(PeerError::MissingTerminalEvidence);
        }
        let session_authority_failed = disposition == TurnDisposition::ProtocolFailed;
        let receipt = TurnReceipt {
            correlation_identity: correlation,
            disposition,
            final_assistant_outcome: outcome,
        };
        self.settled_turns.push(receipt.clone());
        self.active_turn = None;
        if session_authority_failed {
            // The exact terminal remains evidence even though the missing
            // assistant outcome invalidates all further session authority.
            // Returning it alongside a Fatal phase lets the supervisor durably
            // project the terminal before it contains the child.
            self.fence();
        }
        Ok(receipt)
    }

    fn observe_disposed(
        &mut self,
        sequence: u64,
        correlation: Option<CorrelationIdentity>,
        receipt: TranscriptFlushReceiptV1,
    ) -> Result<(), PeerError> {
        let Some(correlation) = correlation else {
            self.fence();
            return Err(PeerError::UnknownCorrelation);
        };
        let pending = self
            .pending_commands
            .get(&correlation)
            .ok_or(PeerError::UnknownCorrelation)?;
        if pending.name != CommandName::Dispose
            || !pending.result_seen
            || !pending.accepted
            || self.phase != PeerPhase::Closing
            || self.active_turn.is_some()
        {
            self.fence();
            return Err(PeerError::InvalidTransition);
        }
        let Some(active_dispose) = self.active_dispose.as_ref() else {
            self.fence();
            return Err(PeerError::MissingTerminalEvidence);
        };
        if active_dispose.correlation != correlation
            || active_dispose
                .final_usage_sequence
                .is_none_or(|usage| usage.checked_add(1) != Some(sequence))
        {
            self.fence();
            return Err(PeerError::MissingTerminalEvidence);
        }
        let Some(create) = self.create.as_ref() else {
            self.fence();
            return Err(PeerError::TranscriptReceipt);
        };
        let Some(configuration) = self.configuration.as_ref() else {
            self.fence();
            return Err(PeerError::TranscriptReceipt);
        };
        match receipt {
            TranscriptFlushReceiptV1::Materialized {
                session_identity,
                session_file,
                header_cwd,
                first_user_prompt,
                ..
            } => {
                if session_identity != self.session_identity
                    || session_file != configuration.session_file
                    || !session_file.is_strict_descendant_of(&create.session_directory)
                    || header_cwd != create.cwd
                {
                    self.fence();
                    return Err(PeerError::TranscriptReceipt);
                }
                if let Some(expected_prompt_digest) = self.first_prompt_digest.as_ref() {
                    match first_user_prompt {
                        crate::protocol::FirstUserPromptReceipt::Verified { digest }
                            if &digest == expected_prompt_digest => {}
                        _ => {
                            self.fence();
                            return Err(PeerError::TranscriptReceipt);
                        }
                    }
                } else if first_user_prompt != crate::protocol::FirstUserPromptReceipt::Absent {
                    self.fence();
                    return Err(PeerError::TranscriptReceipt);
                }
            }
            TranscriptFlushReceiptV1::UnmaterializedNoPrompt {
                session_identity,
                session_file,
            } => {
                if self.first_prompt_digest.is_some()
                    || session_identity != self.session_identity
                    || session_file != configuration.session_file
                    || !session_file.is_strict_descendant_of(&create.session_directory)
                {
                    self.fence();
                    return Err(PeerError::TranscriptReceipt);
                }
            }
        }
        self.active_dispose = None;
        self.phase = PeerPhase::Disposed;
        Ok(())
    }

    fn fence(&mut self) {
        self.phase = PeerPhase::Fatal;
        self.pending_prompt = None;
        self.pending_dispose = None;
        self.active_dispose = None;
        self.active_turn = None;
    }
}

fn raw_jsonl_utf8(line: &[u8]) -> Result<&str, ProtocolError> {
    if line.len() > MAX_JSONL_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }
    // These APIs receive one logical record after the streaming decoder has
    // removed its newline delimiter. Accepting a delimiter here would make the
    // same exact bytes mean different things to a caller and to a JSONL pipe.
    if line.contains(&b'\n') || line.contains(&b'\r') {
        return Err(ProtocolError::InvalidJsonlLine);
    }
    std::str::from_utf8(line).map_err(|_| ProtocolError::InvalidUtf8)
}

fn observe_turn_evidence(
    state: &mut TurnEvidencePhase,
    event: ProjectedAgentEvent,
    sequence: u64,
) -> Result<(), PeerError> {
    match event {
        ProjectedAgentEvent::AgentStart => match state {
            TurnEvidencePhase::AwaitingAgentStart | TurnEvidencePhase::RetryLifecycle => {
                *state = TurnEvidencePhase::ActiveAttempt;
                Ok(())
            }
            TurnEvidencePhase::ActiveAttempt
            | TurnEvidencePhase::AwaitingAgentSettled { .. }
            | TurnEvidencePhase::AwaitingFinalPromptUsage { .. }
            | TurnEvidencePhase::AwaitingSettledFrame { .. } => {
                Err(PeerError::MissingTerminalEvidence)
            }
        },
        ProjectedAgentEvent::AgentEnd {
            messages,
            will_retry,
        } => {
            if !matches!(
                *state,
                TurnEvidencePhase::ActiveAttempt | TurnEvidencePhase::RetryLifecycle
            ) {
                return Err(PeerError::MissingTerminalEvidence);
            }
            if will_retry {
                *state = TurnEvidencePhase::RetryLifecycle;
                return Ok(());
            }
            let final_stop_reason =
                final_assistant_stop_reason(&messages).ok_or(PeerError::MissingTerminalEvidence)?;
            *state = TurnEvidencePhase::AwaitingAgentSettled { final_stop_reason };
            Ok(())
        }
        ProjectedAgentEvent::AgentSettled => match *state {
            TurnEvidencePhase::AwaitingAgentSettled { final_stop_reason } => {
                *state = TurnEvidencePhase::AwaitingFinalPromptUsage {
                    final_stop_reason,
                    agent_settled_sequence: sequence,
                };
                Ok(())
            }
            _ => Err(PeerError::MissingTerminalEvidence),
        },
        _ if matches!(
            *state,
            TurnEvidencePhase::AwaitingFinalPromptUsage { .. }
                | TurnEvidencePhase::AwaitingSettledFrame { .. }
        ) =>
        {
            // Pi's terminal proof is closed: after `agent_settled`, only the
            // prompt's forced cumulative snapshot and `Settled` may follow.
            Err(PeerError::MissingTerminalEvidence)
        }
        _ => Ok(()),
    }
}

fn digest(bytes: &[u8]) -> Blake3Digest {
    let digest = blake3::hash(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest.as_bytes() {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    Blake3Digest::parse(output).expect("BLAKE3 formatter creates valid lowercase hex")
}

/// The host's final outcome must be derivable from the final non-retried
/// `agent_end` messages. Retry-era assistant errors are not terminal facts.
fn final_assistant_stop_reason(messages: &[miniserde::json::Value]) -> Option<AssistantStopReason> {
    for message in messages.iter().rev() {
        let miniserde::json::Value::Object(object) = message else {
            continue;
        };
        if !matches!(
            object.get("role"),
            Some(miniserde::json::Value::String(role)) if role == "assistant"
        ) {
            continue;
        }
        return match object.get("stopReason") {
            Some(miniserde::json::Value::String(reason)) if reason == "stop" => {
                Some(AssistantStopReason::Stop)
            }
            Some(miniserde::json::Value::String(reason)) if reason == "length" => {
                Some(AssistantStopReason::Length)
            }
            Some(miniserde::json::Value::String(reason)) if reason == "error" => {
                Some(AssistantStopReason::Error)
            }
            Some(miniserde::json::Value::String(reason)) if reason == "aborted" => {
                Some(AssistantStopReason::Aborted)
            }
            _ => None,
        };
    }
    None
}

#[cfg(test)]
mod peer_tests {
    // These are closed fixture constructors and assertion boundaries; panicking
    // keeps invalid test data local and legible without weakening production code.
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::protocol::{
        AbsolutePath, AdapterVersion, Binary64BigEndianHex, CacheWritePerMillionRateV1,
        CompactionMode, CompactionPolicyV1, HostProcessId, KnownPerMillionRateV1, ModelApi,
        ModelCatalogPolicyV1, ModelId, ModelInput, ModelSelection, NonNegativeInteger,
        OpenRouterBaseUrl, PiSdkVersion, PositiveInteger, ProjectTrust, Provider,
        ProviderCostObservationV1, RetryPolicyV1, SettledClassification, ThinkingLevel,
        ToolProfile, Transport, UsageTotals,
    };
    use miniserde::json::{Object, Value};

    const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn digest_value() -> Blake3Digest {
        Blake3Digest::parse(DIGEST).unwrap()
    }
    fn runtime() -> RuntimeIdentity {
        RuntimeIdentity {
            node_version: crate::protocol::NodeRuntimeVersion::parse("v22.19.0").unwrap(),
            adapter_version: AdapterVersion::V1,
            pi_sdk_version: PiSdkVersion::V0830,
            node_executable_blake3: digest_value(),
            lockfile_blake3: digest_value(),
            adapter_build_blake3: digest_value(),
            pi_transitive_package_set_blake3: digest_value(),
        }
    }
    fn model() -> ModelSelection {
        ModelSelection {
            provider: Provider::OpenRouter,
            model_id: ModelId::DeepseekV4Flash0731,
            thinking_level: ThinkingLevel::High,
        }
    }
    fn policy() -> crate::protocol::ActorModelPolicyV1 {
        crate::protocol::ActorModelPolicyV1 {
            retry: RetryPolicyV1 {
                max_retries: NonNegativeInteger::parse(2).unwrap(),
                base_delay_milliseconds: NonNegativeInteger::parse(2_000).unwrap(),
                provider_timeout_milliseconds: PositiveInteger::parse(300_000).unwrap(),
                provider_max_retries: NonNegativeInteger::parse(1).unwrap(),
                provider_max_retry_delay_milliseconds: PositiveInteger::parse(30_000).unwrap(),
            },
            compaction: CompactionPolicyV1 {
                mode: CompactionMode::Enabled,
                reserve_tokens: NonNegativeInteger::parse(16_384).unwrap(),
                keep_recent_tokens: NonNegativeInteger::parse(20_000).unwrap(),
            },
            steering_mode: crate::protocol::QueueMode::OneAtATime,
            follow_up_mode: crate::protocol::QueueMode::OneAtATime,
            transport: Transport::Sse,
            project_trust: ProjectTrust::Never,
            install_telemetry: crate::protocol::Disabled::Disabled,
            analytics: crate::protocol::Disabled::Disabled,
            images: crate::protocol::Images::Blocked,
        }
    }
    fn catalog() -> ModelCatalogPolicyV1 {
        let rate = |value| KnownPerMillionRateV1 {
            usd_per_million: crate::protocol::UsdPerMillionDecimal::parse(value).unwrap(),
        };
        ModelCatalogPolicyV1 {
            catalog_blake3: digest_value(),
            effective_model: crate::protocol::EffectiveModelDescriptorV1 {
                provider: Provider::OpenRouter,
                base_url: OpenRouterBaseUrl::ApiV1,
                api: ModelApi::OpenAiCompletions,
                model_id: ModelId::DeepseekV4Flash0731,
                canonical_slug: crate::protocol::CanonicalModelSlug::DeepseekV4Flash20260731,
                input: ModelInput::TextOnly,
                context_window: PositiveInteger::parse(1_048_576).unwrap(),
                max_tokens: PositiveInteger::parse(384_000).unwrap(),
                input_usd_per_million: rate("0.09"),
                output_usd_per_million: rate("0.18"),
                cache_read_usd_per_million: rate("0.018"),
                cache_write_usd_per_million: CacheWritePerMillionRateV1::Absent,
            },
        }
    }
    fn create() -> CreateSessionPayload {
        CreateSessionPayload {
            session_kind: SessionKind::RootAuthorityOffice,
            cwd: AbsolutePath::parse("/tmp/peer/cwd").unwrap(),
            agent_directory: AbsolutePath::parse("/tmp/peer/agent").unwrap(),
            auth_path: AbsolutePath::parse("/tmp/peer/agent/auth.json").unwrap(),
            models_path: AbsolutePath::parse("/tmp/peer/agent/models.json").unwrap(),
            session_directory: AbsolutePath::parse("/tmp/peer/sessions").unwrap(),
            system_prompt: "founding mission".into(),
            system_prompt_digest: digest(b"founding mission"),
            model: model(),
            model_catalog: catalog(),
            tool_profile: ToolProfile::ReadExecuteV1,
            settings: policy(),
            forum_contract: crate::forum::ForumSessionContractV1::forum_enabled_v1().unwrap(),
        }
    }
    fn frame(sequence: u64, correlation: Option<&str>, event: OutboundEvent) -> OutboundFrame {
        OutboundFrame {
            sequence: crate::protocol::BoundarySequence::parse(sequence).unwrap(),
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            correlation_identity: correlation
                .map(|value| CorrelationIdentity::parse(value).unwrap()),
            event,
        }
    }
    fn setup_ready() -> BoundaryPeer {
        let mut peer = BoundaryPeer::new(
            SessionIdentity::parse("peer-session-001").unwrap(),
            HostProcessId::parse(42).unwrap(),
            SpawnNonce::parse("peer-nonce-001").unwrap(),
            runtime(),
        )
        .unwrap();
        peer.observe_outbound(frame(
            1,
            None,
            OutboundEvent::AdapterReady {
                pid: HostProcessId::parse(42).unwrap(),
                spawn_nonce: SpawnNonce::parse("peer-nonce-001").unwrap(),
                runtime: runtime(),
            },
        ))
        .unwrap();
        let payload = create();
        peer.admit_inbound(InboundFrame {
            sequence: crate::protocol::BoundarySequence::parse(1).unwrap(),
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            correlation_identity: CorrelationIdentity::parse("create-001").unwrap(),
            command: InboundCommand::CreateSession(Box::new(payload.clone())),
        })
        .unwrap();
        peer.observe_outbound(frame(
            2,
            Some("create-001"),
            OutboundEvent::CommandResult(CommandResult::Accepted {
                command: CommandName::CreateSession,
                detail: crate::protocol::CommandResultDetail::Acknowledged,
            }),
        ))
        .unwrap();
        let configuration = EffectiveSessionConfiguration {
            session_kind: payload.session_kind,
            cwd: payload.cwd.clone(),
            session_directory: payload.session_directory.clone(),
            session_file: AbsolutePath::parse("/tmp/peer/sessions/receipt.jsonl").unwrap(),
            model: payload.model.clone(),
            model_catalog: payload.model_catalog.clone(),
            tool_profile: payload.tool_profile,
            tools: payload.tool_profile.tools().to_vec(),
            settings: payload.settings.clone(),
            forum_contract: payload.forum_contract.clone(),
        };
        peer.observe_outbound(frame(
            3,
            Some("create-001"),
            OutboundEvent::SessionReady { configuration },
        ))
        .unwrap();
        peer
    }
    fn admit_dispose(peer: &mut BoundaryPeer) {
        peer.admit_inbound(InboundFrame {
            sequence: crate::protocol::BoundarySequence::parse(2).unwrap(),
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            correlation_identity: CorrelationIdentity::parse("dispose-001").unwrap(),
            command: InboundCommand::Dispose(crate::protocol::DisposePayload {
                reason: crate::protocol::DisposeReason::ProcessRecovery,
            }),
        })
        .unwrap();
        peer.observe_outbound(frame(
            4,
            Some("dispose-001"),
            OutboundEvent::CommandResult(CommandResult::Accepted {
                command: CommandName::Dispose,
                detail: crate::protocol::CommandResultDetail::Acknowledged,
            }),
        ))
        .unwrap();
    }
    fn unmaterialized_transcript_receipt() -> TranscriptFlushReceiptV1 {
        TranscriptFlushReceiptV1::UnmaterializedNoPrompt {
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            session_file: AbsolutePath::parse("/tmp/peer/sessions/receipt.jsonl").unwrap(),
        }
    }
    fn materialized_transcript_receipt() -> TranscriptFlushReceiptV1 {
        TranscriptFlushReceiptV1::Materialized {
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            session_file: AbsolutePath::parse("/tmp/peer/sessions/receipt.jsonl").unwrap(),
            session_file_blake3: digest_value(),
            header_cwd: AbsolutePath::parse("/tmp/peer/cwd").unwrap(),
            first_user_prompt: crate::protocol::FirstUserPromptReceipt::Verified {
                digest: digest(b"first prompt"),
            },
        }
    }
    fn admit_prompt(peer: &mut BoundaryPeer) {
        peer.admit_inbound(prompt_frame()).unwrap();
        peer.observe_outbound(frame(
            4,
            Some("prompt-001"),
            OutboundEvent::CommandResult(CommandResult::Accepted {
                command: CommandName::Prompt,
                detail: crate::protocol::CommandResultDetail::Acknowledged,
            }),
        ))
        .unwrap();
    }
    fn prompt_frame() -> InboundFrame {
        InboundFrame {
            sequence: crate::protocol::BoundarySequence::parse(2).unwrap(),
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            correlation_identity: CorrelationIdentity::parse("prompt-001").unwrap(),
            command: InboundCommand::Prompt(crate::protocol::PromptPayload {
                purpose: PromptPurpose::OfficeTurn,
                text: "first prompt".into(),
            }),
        }
    }
    fn zero_usage() -> UsageObservation {
        UsageObservation::Known(UsageTotals {
            input_tokens: NonNegativeInteger::parse(0).unwrap(),
            output_tokens: NonNegativeInteger::parse(0).unwrap(),
            cache_read_tokens: NonNegativeInteger::parse(0).unwrap(),
            cache_write_tokens: NonNegativeInteger::parse(0).unwrap(),
            total_tokens: NonNegativeInteger::parse(0).unwrap(),
            provider_cost: ProviderCostObservationV1 {
                binary64_big_endian_hex: Binary64BigEndianHex::parse("0000000000000000").unwrap(),
            },
        })
    }
    fn terminal_stop_event() -> ProjectedAgentEvent {
        ProjectedAgentEvent::AgentEnd {
            messages: vec![assistant_message("stop")],
            will_retry: false,
        }
    }

    fn assistant_message(stop_reason: &str) -> Value {
        Value::Object(Object::from_iter([
            ("role".into(), Value::String("assistant".into())),
            ("stopReason".into(), Value::String(stop_reason.into())),
        ]))
    }

    fn materialized_prompt_sequence(peer: &mut BoundaryPeer, start_sequence: u64) -> u64 {
        peer.observe_outbound(frame(
            start_sequence,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AgentStart,
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            start_sequence + 1,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: terminal_stop_event(),
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            start_sequence + 2,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AgentSettled,
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            start_sequence + 3,
            Some("prompt-001"),
            OutboundEvent::UsageSnapshot {
                usage: zero_usage(),
            },
        ))
        .unwrap();
        start_sequence + 4
    }

    #[test]
    fn malformed_raw_boundary_frames_seal_then_fence_before_another_prompt() {
        let mut inbound = setup_ready();
        assert_eq!(
            inbound.admit_inbound_jsonl("{"),
            Err(PeerError::Protocol(ProtocolError::InvalidJson))
        );
        assert_eq!(inbound.inbound_seals().len(), 1);
        assert_eq!(inbound.phase(), PeerPhase::Fatal);
        assert_eq!(inbound.admit_inbound(prompt_frame()), Err(PeerError::Fatal));

        let mut outbound = setup_ready();
        let duplicate = r#"{"protocolVersion":"society-pi-host/v4","sequence":4,"sequence":4,"sessionIdentity":"peer-session-001","event":"Fatal","failureCode":"protocol_decode_failed"}"#;
        assert_eq!(
            outbound.observe_outbound_jsonl(duplicate),
            Err(PeerError::Protocol(ProtocolError::DuplicateObjectKey))
        );
        assert_eq!(outbound.outbound_seals().len(), 1);
        assert_eq!(outbound.phase(), PeerPhase::Fatal);
        assert_eq!(
            outbound.admit_inbound(prompt_frame()),
            Err(PeerError::Fatal)
        );
    }

    #[test]
    fn raw_invalid_utf8_is_sealed_exactly_then_fences_the_future_command_path() {
        let mut peer = setup_ready();
        let bytes = [b'{', 0xff, b'}'];
        assert_eq!(
            peer.admit_inbound_jsonl_bytes(&bytes),
            Err(PeerError::Protocol(ProtocolError::InvalidUtf8))
        );
        assert_eq!(peer.inbound_seals(), &[SealedLine::of_bytes(&bytes)]);
        assert_eq!(peer.phase(), PeerPhase::Fatal);
        assert_eq!(peer.admit_inbound(prompt_frame()), Err(PeerError::Fatal));
    }

    #[test]
    fn raw_jsonl_delimiter_is_not_an_ambiguous_record_boundary() {
        let mut peer = setup_ready();
        assert_eq!(
            peer.observe_outbound_jsonl_bytes(b"{}\n"),
            Err(PeerError::Protocol(ProtocolError::InvalidJsonlLine))
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn adapter_ready_requires_the_supervised_child_pid() {
        let mut peer = BoundaryPeer::new(
            SessionIdentity::parse("peer-session-001").unwrap(),
            HostProcessId::parse(42).unwrap(),
            SpawnNonce::parse("peer-nonce-001").unwrap(),
            runtime(),
        )
        .unwrap();
        assert_eq!(
            peer.observe_outbound(frame(
                1,
                None,
                OutboundEvent::AdapterReady {
                    pid: HostProcessId::parse(43).unwrap(),
                    spawn_nonce: SpawnNonce::parse("peer-nonce-001").unwrap(),
                    runtime: runtime(),
                },
            )),
            Err(PeerError::RuntimeIdentity)
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn direct_typed_semantic_errors_always_fence_the_peer() {
        let mut session_ready = setup_ready();
        let payload = create();
        let configuration = EffectiveSessionConfiguration {
            session_kind: payload.session_kind,
            cwd: payload.cwd.clone(),
            session_directory: payload.session_directory.clone(),
            session_file: AbsolutePath::parse("/tmp/peer/sessions/receipt.jsonl").unwrap(),
            model: payload.model.clone(),
            model_catalog: payload.model_catalog.clone(),
            tool_profile: payload.tool_profile,
            tools: payload.tool_profile.tools().to_vec(),
            settings: payload.settings,
            forum_contract: payload.forum_contract,
        };
        assert_eq!(
            session_ready.observe_outbound(frame(
                4,
                Some("unknown-session-ready"),
                OutboundEvent::SessionReady { configuration },
            )),
            Err(PeerError::UnknownCorrelation)
        );
        assert_eq!(session_ready.phase(), PeerPhase::Fatal);

        let mut disposed = setup_ready();
        assert_eq!(
            disposed.observe_outbound(frame(
                4,
                Some("unknown-dispose"),
                OutboundEvent::Disposed {
                    transcript_flush_receipt: TranscriptFlushReceiptV1::UnmaterializedNoPrompt {
                        session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
                        session_file: AbsolutePath::parse("/tmp/peer/sessions/receipt.jsonl")
                            .unwrap(),
                    },
                },
            )),
            Err(PeerError::UnknownCorrelation)
        );
        assert_eq!(disposed.phase(), PeerPhase::Fatal);

        let mut invalid_profile = BoundaryPeer::new(
            SessionIdentity::parse("peer-session-001").unwrap(),
            HostProcessId::parse(42).unwrap(),
            SpawnNonce::parse("peer-nonce-001").unwrap(),
            runtime(),
        )
        .unwrap();
        invalid_profile
            .observe_outbound(frame(
                1,
                None,
                OutboundEvent::AdapterReady {
                    pid: HostProcessId::parse(42).unwrap(),
                    spawn_nonce: SpawnNonce::parse("peer-nonce-001").unwrap(),
                    runtime: runtime(),
                },
            ))
            .unwrap();
        let mut profile = create();
        profile.settings.retry.base_delay_milliseconds = NonNegativeInteger::parse(1).unwrap();
        assert!(matches!(
            invalid_profile.admit_inbound(InboundFrame {
                sequence: crate::protocol::BoundarySequence::parse(1).unwrap(),
                session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
                correlation_identity: CorrelationIdentity::parse("create-001").unwrap(),
                command: InboundCommand::CreateSession(Box::new(profile)),
            }),
            Err(PeerError::Protocol(ProtocolError::InvalidFrame(_)))
        ));
        assert_eq!(invalid_profile.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn oversize_raw_frame_fences_and_ready_eof_is_not_a_normal_terminal() {
        let mut peer = setup_ready();
        assert_eq!(
            peer.observe_outbound_jsonl(&"x".repeat(crate::protocol::MAX_JSONL_FRAME_BYTES + 1)),
            Err(PeerError::Protocol(ProtocolError::FrameTooLarge))
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
        assert_eq!(peer.admit_inbound(prompt_frame()), Err(PeerError::Fatal));

        let mut ready = setup_ready();
        assert_eq!(
            ready.observe_stdout_eof(),
            Err(PeerError::MissingTerminalEvidence)
        );
        assert_eq!(ready.phase(), PeerPhase::Fatal);
        assert_eq!(ready.admit_inbound(prompt_frame()), Err(PeerError::Fatal));
    }

    #[test]
    fn early_prompt_usage_cannot_certify_the_later_terminal_assistant_outcome() {
        let mut peer = setup_ready();
        admit_prompt(&mut peer);
        peer.observe_outbound(frame(
            5,
            Some("prompt-001"),
            OutboundEvent::UsageSnapshot {
                usage: zero_usage(),
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            6,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AgentStart,
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            7,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: terminal_stop_event(),
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            8,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AgentSettled,
            },
        ))
        .unwrap();
        assert_eq!(
            peer.observe_outbound(frame(
                9,
                Some("prompt-001"),
                OutboundEvent::Settled {
                    classification: SettledClassification::Completed,
                    final_assistant_outcome: FinalAssistantOutcome::Observed {
                        stop_reason: AssistantStopReason::Stop
                    },
                },
            )),
            Err(PeerError::MissingTerminalEvidence)
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn missing_or_control_correlated_usage_cannot_certify_an_active_prompt() {
        let mut missing = setup_ready();
        admit_prompt(&mut missing);
        assert_eq!(
            missing.observe_outbound(frame(
                5,
                None,
                OutboundEvent::UsageSnapshot {
                    usage: zero_usage()
                }
            )),
            Err(PeerError::UnknownCorrelation)
        );
        assert_eq!(missing.phase(), PeerPhase::Fatal);

        let mut control = setup_ready();
        admit_prompt(&mut control);
        control
            .admit_inbound(InboundFrame {
                sequence: crate::protocol::BoundarySequence::parse(3).unwrap(),
                session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
                correlation_identity: CorrelationIdentity::parse("state-001").unwrap(),
                command: InboundCommand::GetState,
            })
            .unwrap();
        control
            .observe_outbound(frame(
                5,
                Some("state-001"),
                OutboundEvent::CommandResult(CommandResult::Accepted {
                    command: CommandName::GetState,
                    detail: crate::protocol::CommandResultDetail::Acknowledged,
                }),
            ))
            .unwrap();
        assert!(matches!(
            control.observe_outbound(frame(
                6,
                Some("state-001"),
                OutboundEvent::UsageSnapshot {
                    usage: zero_usage()
                }
            )),
            Ok(Some(PeerObservation::Usage(_)))
        ));
        control
            .observe_outbound(frame(
                7,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: ProjectedAgentEvent::AgentStart,
                },
            ))
            .unwrap();
        control
            .observe_outbound(frame(
                8,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: terminal_stop_event(),
                },
            ))
            .unwrap();
        control
            .observe_outbound(frame(
                9,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: ProjectedAgentEvent::AgentSettled,
                },
            ))
            .unwrap();
        assert_eq!(
            control.observe_outbound(frame(
                10,
                Some("prompt-001"),
                OutboundEvent::Settled {
                    classification: SettledClassification::Completed,
                    final_assistant_outcome: FinalAssistantOutcome::Observed {
                        stop_reason: AssistantStopReason::Stop
                    }
                }
            )),
            Err(PeerError::MissingTerminalEvidence)
        );
    }

    #[test]
    fn failed_and_aborted_turns_follow_host_snapshot_correlation_ordering() {
        let mut failed = setup_ready();
        admit_prompt(&mut failed);
        failed
            .observe_outbound(frame(
                5,
                Some("prompt-001"),
                OutboundEvent::UsageSnapshot {
                    usage: zero_usage(),
                },
            ))
            .unwrap();
        assert!(matches!(
            failed.observe_outbound(frame(
                6,
                Some("prompt-001"),
                OutboundEvent::Settled {
                    classification: SettledClassification::Failed,
                    final_assistant_outcome: FinalAssistantOutcome::Unavailable {
                        reason:
                            crate::protocol::FinalAssistantUnavailableReason::SdkPromiseRejected,
                    },
                },
            )),
            Ok(Some(PeerObservation::TurnSettled(TurnReceipt {
                disposition: TurnDisposition::Failed,
                ..
            })))
        ));

        let mut aborted = setup_ready();
        admit_prompt(&mut aborted);
        aborted
            .observe_outbound(frame(
                5,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: ProjectedAgentEvent::AgentStart,
                },
            ))
            .unwrap();
        aborted
            .admit_inbound(InboundFrame {
                sequence: crate::protocol::BoundarySequence::parse(3).unwrap(),
                session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
                correlation_identity: CorrelationIdentity::parse("abort-001").unwrap(),
                command: InboundCommand::Abort(crate::protocol::AbortPayload {
                    reason: crate::protocol::AbortReason::GracefulCancellation,
                }),
            })
            .unwrap();
        aborted
            .observe_outbound(frame(
                6,
                Some("abort-001"),
                OutboundEvent::CommandResult(CommandResult::Accepted {
                    command: CommandName::Abort,
                    detail: crate::protocol::CommandResultDetail::Acknowledged,
                }),
            ))
            .unwrap();
        // The host's immediate Abort snapshot contributes cumulative usage but
        // cannot certify the active Prompt's later terminal outcome.
        aborted
            .observe_outbound(frame(
                7,
                Some("abort-001"),
                OutboundEvent::UsageSnapshot {
                    usage: zero_usage(),
                },
            ))
            .unwrap();
        aborted
            .observe_outbound(frame(
                8,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: terminal_stop_event(),
                },
            ))
            .unwrap();
        aborted
            .observe_outbound(frame(
                9,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: ProjectedAgentEvent::AgentSettled,
                },
            ))
            .unwrap();
        aborted
            .observe_outbound(frame(
                10,
                Some("prompt-001"),
                OutboundEvent::UsageSnapshot {
                    usage: zero_usage(),
                },
            ))
            .unwrap();
        assert!(matches!(
            aborted.observe_outbound(frame(
                11,
                Some("prompt-001"),
                OutboundEvent::Settled {
                    classification: SettledClassification::Aborted,
                    final_assistant_outcome: FinalAssistantOutcome::Observed {
                        stop_reason: AssistantStopReason::Stop,
                    },
                },
            )),
            Ok(Some(PeerObservation::TurnSettled(TurnReceipt {
                disposition: TurnDisposition::Aborted,
                ..
            })))
        ));
    }

    #[test]
    fn failed_prompt_needs_the_forced_snapshot_immediately_before_settlement() {
        let mut peer = setup_ready();
        admit_prompt(&mut peer);
        peer.observe_outbound(frame(
            5,
            Some("prompt-001"),
            OutboundEvent::UsageSnapshot {
                usage: zero_usage(),
            },
        ))
        .unwrap();
        // A resolved/rejected SDK operation can have emitted lifecycle evidence
        // after an earlier cumulative snapshot. Only the host's forced final
        // Prompt-correlated snapshot may certify `failed`.
        peer.observe_outbound(frame(
            6,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AgentStart,
            },
        ))
        .unwrap();
        assert_eq!(
            peer.observe_outbound(frame(
                7,
                Some("prompt-001"),
                OutboundEvent::Settled {
                    classification: SettledClassification::Failed,
                    final_assistant_outcome: FinalAssistantOutcome::Unavailable {
                        reason:
                            crate::protocol::FinalAssistantUnavailableReason::SdkPromiseRejected,
                    },
                },
            )),
            Err(PeerError::MissingTerminalEvidence)
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn protocol_failed_prompt_preserves_the_closed_terminal_before_fencing_session_authority() {
        let mut peer = setup_ready();
        admit_prompt(&mut peer);
        peer.observe_outbound(frame(
            5,
            Some("prompt-001"),
            OutboundEvent::UsageSnapshot {
                usage: zero_usage(),
            },
        ))
        .unwrap();
        assert!(matches!(
            peer.observe_outbound(frame(
                6,
                Some("prompt-001"),
                OutboundEvent::Settled {
                    classification: SettledClassification::ProtocolFailed,
                    final_assistant_outcome: FinalAssistantOutcome::Unavailable {
                        reason: crate::protocol::FinalAssistantUnavailableReason::MissingFinalAssistantOutcome,
                    },
                },
            )),
            Ok(Some(PeerObservation::TurnSettled(TurnReceipt {
                disposition: TurnDisposition::ProtocolFailed,
                ..
            })))
        ));
        assert_eq!(peer.phase(), PeerPhase::Fatal);
        assert_eq!(peer.settled_turns().len(), 1);
    }

    #[test]
    fn admitted_abort_intent_survives_result_after_prompt_settlement() {
        let mut peer = setup_ready();
        admit_prompt(&mut peer);
        // The host records this intent before it awaits session.abort(). Pi may
        // synchronously finish the Prompt while that await is still pending.
        peer.admit_inbound(InboundFrame {
            sequence: crate::protocol::BoundarySequence::parse(3).unwrap(),
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            correlation_identity: CorrelationIdentity::parse("abort-001").unwrap(),
            command: InboundCommand::Abort(crate::protocol::AbortPayload {
                reason: crate::protocol::AbortReason::GracefulCancellation,
            }),
        })
        .unwrap();
        let settled_sequence = materialized_prompt_sequence(&mut peer, 5);
        assert!(matches!(
            peer.observe_outbound(frame(
                settled_sequence,
                Some("prompt-001"),
                OutboundEvent::Settled {
                    classification: SettledClassification::Aborted,
                    final_assistant_outcome: FinalAssistantOutcome::Observed {
                        stop_reason: AssistantStopReason::Stop,
                    },
                },
            )),
            Ok(Some(PeerObservation::TurnSettled(TurnReceipt {
                disposition: TurnDisposition::Aborted,
                ..
            })))
        ));
        // The delayed Abort result and its control-correlated snapshot are
        // accepted as accounting evidence, but cannot change the receipt.
        peer.observe_outbound(frame(
            settled_sequence + 1,
            Some("abort-001"),
            OutboundEvent::CommandResult(CommandResult::Accepted {
                command: CommandName::Abort,
                detail: crate::protocol::CommandResultDetail::Acknowledged,
            }),
        ))
        .unwrap();
        peer.observe_outbound(frame(
            settled_sequence + 2,
            Some("abort-001"),
            OutboundEvent::UsageSnapshot {
                usage: zero_usage(),
            },
        ))
        .unwrap();
        assert_eq!(peer.phase(), PeerPhase::Ready);
    }

    #[test]
    fn rejected_but_admitted_abort_keeps_the_host_matched_abort_intent() {
        let mut peer = setup_ready();
        admit_prompt(&mut peer);
        peer.admit_inbound(InboundFrame {
            sequence: crate::protocol::BoundarySequence::parse(3).unwrap(),
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            correlation_identity: CorrelationIdentity::parse("abort-001").unwrap(),
            command: InboundCommand::Abort(crate::protocol::AbortPayload {
                reason: crate::protocol::AbortReason::GracefulCancellation,
            }),
        })
        .unwrap();
        // `PiSdkHost.abort()` sets intent before an SDK rejection can produce
        // this result, so rejecting it must not rewrite the host's settled
        // classification from aborted to completed.
        peer.observe_outbound(frame(
            5,
            Some("abort-001"),
            OutboundEvent::CommandResult(CommandResult::Rejected {
                command: CommandName::Abort,
                failure_code: AdapterFailureCode::SdkOperationFailed,
            }),
        ))
        .unwrap();
        let settled_sequence = materialized_prompt_sequence(&mut peer, 6);
        assert!(matches!(
            peer.observe_outbound(frame(
                settled_sequence,
                Some("prompt-001"),
                OutboundEvent::Settled {
                    classification: SettledClassification::Aborted,
                    final_assistant_outcome: FinalAssistantOutcome::Observed {
                        stop_reason: AssistantStopReason::Stop,
                    },
                },
            )),
            Ok(Some(PeerObservation::TurnSettled(TurnReceipt {
                disposition: TurnDisposition::Aborted,
                ..
            })))
        ));
    }

    #[test]
    fn get_state_or_abort_may_interleave_after_agent_settled_but_cannot_replace_final_prompt_usage()
    {
        for abort in [false, true] {
            let mut peer = setup_ready();
            admit_prompt(&mut peer);
            peer.observe_outbound(frame(
                5,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: ProjectedAgentEvent::AgentStart,
                },
            ))
            .unwrap();
            peer.observe_outbound(frame(
                6,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: terminal_stop_event(),
                },
            ))
            .unwrap();
            peer.observe_outbound(frame(
                7,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: ProjectedAgentEvent::AgentSettled,
                },
            ))
            .unwrap();
            let command_name = if abort {
                CommandName::Abort
            } else {
                CommandName::GetState
            };
            let correlation = if abort { "abort-001" } else { "state-001" };
            let command = if abort {
                InboundCommand::Abort(crate::protocol::AbortPayload {
                    reason: crate::protocol::AbortReason::GracefulCancellation,
                })
            } else {
                InboundCommand::GetState
            };
            peer.admit_inbound(InboundFrame {
                sequence: crate::protocol::BoundarySequence::parse(3).unwrap(),
                session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
                correlation_identity: CorrelationIdentity::parse(correlation).unwrap(),
                command,
            })
            .unwrap();
            peer.observe_outbound(frame(
                8,
                Some(correlation),
                OutboundEvent::CommandResult(CommandResult::Accepted {
                    command: command_name,
                    detail: crate::protocol::CommandResultDetail::Acknowledged,
                }),
            ))
            .unwrap();
            peer.observe_outbound(frame(
                9,
                Some(correlation),
                OutboundEvent::UsageSnapshot {
                    usage: zero_usage(),
                },
            ))
            .unwrap();
            peer.observe_outbound(frame(
                10,
                Some("prompt-001"),
                OutboundEvent::UsageSnapshot {
                    usage: zero_usage(),
                },
            ))
            .unwrap();
            assert!(matches!(
                peer.observe_outbound(frame(
                    11,
                    Some("prompt-001"),
                    OutboundEvent::Settled {
                        classification: SettledClassification::Completed,
                        final_assistant_outcome: FinalAssistantOutcome::Observed {
                            stop_reason: AssistantStopReason::Stop,
                        },
                    },
                )),
                Ok(Some(PeerObservation::TurnSettled(TurnReceipt {
                    disposition: TurnDisposition::Completed,
                    ..
                })))
            ));
        }
    }

    #[test]
    fn terminal_lifecycle_requires_start_then_final_end_then_settled_once() {
        let mut peer = setup_ready();
        admit_prompt(&mut peer);
        assert_eq!(
            peer.observe_outbound(frame(
                5,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: ProjectedAgentEvent::AgentSettled,
                },
            )),
            Err(PeerError::MissingTerminalEvidence)
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);

        let mut duplicate_start = setup_ready();
        admit_prompt(&mut duplicate_start);
        duplicate_start
            .observe_outbound(frame(
                5,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: ProjectedAgentEvent::AgentStart,
                },
            ))
            .unwrap();
        assert_eq!(
            duplicate_start.observe_outbound(frame(
                6,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: ProjectedAgentEvent::AgentStart,
                },
            )),
            Err(PeerError::MissingTerminalEvidence)
        );
        assert_eq!(duplicate_start.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn admitted_prompt_is_a_pending_execution_and_fences_second_prompt_or_dispose() {
        let mut double_prompt = setup_ready();
        double_prompt.admit_inbound(prompt_frame()).unwrap();
        let mut second = prompt_frame();
        second.sequence = crate::protocol::BoundarySequence::parse(3).unwrap();
        second.correlation_identity = CorrelationIdentity::parse("prompt-002").unwrap();
        assert_eq!(
            double_prompt.admit_inbound(second),
            Err(PeerError::InvalidTransition)
        );
        assert_eq!(double_prompt.phase(), PeerPhase::Fatal);

        let mut dispose = setup_ready();
        dispose.admit_inbound(prompt_frame()).unwrap();
        assert_eq!(
            dispose.admit_inbound(InboundFrame {
                sequence: crate::protocol::BoundarySequence::parse(3).unwrap(),
                session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
                correlation_identity: CorrelationIdentity::parse("dispose-001").unwrap(),
                command: InboundCommand::Dispose(crate::protocol::DisposePayload {
                    reason: crate::protocol::DisposeReason::ProcessRecovery,
                }),
            }),
            Err(PeerError::InvalidTransition)
        );
        assert_eq!(dispose.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn pending_dispose_fences_every_later_command_until_its_evidenced_terminal_receipt() {
        for command in [
            InboundCommand::Prompt(crate::protocol::PromptPayload {
                purpose: PromptPurpose::OfficeTurn,
                text: "too late".into(),
            }),
            InboundCommand::GetState,
            InboundCommand::Abort(crate::protocol::AbortPayload {
                reason: crate::protocol::AbortReason::GracefulCancellation,
            }),
            InboundCommand::Dispose(crate::protocol::DisposePayload {
                reason: crate::protocol::DisposeReason::ProcessRecovery,
            }),
        ] {
            let mut peer = setup_ready();
            peer.admit_inbound(InboundFrame {
                sequence: crate::protocol::BoundarySequence::parse(2).unwrap(),
                session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
                correlation_identity: CorrelationIdentity::parse("dispose-001").unwrap(),
                command: InboundCommand::Dispose(crate::protocol::DisposePayload {
                    reason: crate::protocol::DisposeReason::ProcessRecovery,
                }),
            })
            .unwrap();
            assert_eq!(
                peer.admit_inbound(InboundFrame {
                    sequence: crate::protocol::BoundarySequence::parse(3).unwrap(),
                    session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
                    correlation_identity: CorrelationIdentity::parse("later-001").unwrap(),
                    command,
                }),
                Err(PeerError::InvalidTransition)
            );
            assert_eq!(peer.phase(), PeerPhase::Fatal);
        }

        let mut rejected = setup_ready();
        rejected
            .admit_inbound(InboundFrame {
                sequence: crate::protocol::BoundarySequence::parse(2).unwrap(),
                session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
                correlation_identity: CorrelationIdentity::parse("dispose-001").unwrap(),
                command: InboundCommand::Dispose(crate::protocol::DisposePayload {
                    reason: crate::protocol::DisposeReason::ProcessRecovery,
                }),
            })
            .unwrap();
        assert_eq!(
            rejected.observe_outbound(frame(
                4,
                Some("dispose-001"),
                OutboundEvent::CommandResult(CommandResult::Rejected {
                    command: CommandName::Dispose,
                    failure_code: AdapterFailureCode::SdkOperationFailed,
                }),
            )),
            Err(PeerError::InvalidTransition)
        );
        assert_eq!(rejected.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn follow_up_and_steer_close_with_the_terminal_agent_lifecycle() {
        for after_agent_settled in [false, true] {
            let mut peer = setup_ready();
            admit_prompt(&mut peer);
            peer.observe_outbound(frame(
                5,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: ProjectedAgentEvent::AgentStart,
                },
            ))
            .unwrap();
            peer.observe_outbound(frame(
                6,
                Some("prompt-001"),
                OutboundEvent::AgentEvent {
                    agent_event: terminal_stop_event(),
                },
            ))
            .unwrap();
            if after_agent_settled {
                peer.observe_outbound(frame(
                    7,
                    Some("prompt-001"),
                    OutboundEvent::AgentEvent {
                        agent_event: ProjectedAgentEvent::AgentSettled,
                    },
                ))
                .unwrap();
            }
            let command = if after_agent_settled {
                InboundCommand::Steer(crate::protocol::SteerPayload {
                    reason: crate::protocol::SteerReason::UrgentUnsafePremise,
                    text: "too late".into(),
                })
            } else {
                InboundCommand::FollowUp(crate::protocol::FollowUpPayload {
                    notice_delivery_identity: CorrelationIdentity::parse("notice-001").unwrap(),
                    ledger_frontier: crate::protocol::LedgerFrontier::parse(0).unwrap(),
                    text: "too late".into(),
                })
            };
            assert_eq!(
                peer.admit_inbound(InboundFrame {
                    sequence: crate::protocol::BoundarySequence::parse(3).unwrap(),
                    session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
                    correlation_identity: CorrelationIdentity::parse("late-control-001").unwrap(),
                    command,
                }),
                Err(PeerError::InvalidTransition)
            );
            assert_eq!(peer.phase(), PeerPhase::Fatal);
        }
    }

    #[test]
    fn follow_up_waits_for_real_agent_start() {
        let mut peer = setup_ready();
        admit_prompt(&mut peer);
        let early = InboundFrame {
            sequence: crate::protocol::BoundarySequence::parse(3).unwrap(),
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            correlation_identity: CorrelationIdentity::parse("follow-001").unwrap(),
            command: InboundCommand::FollowUp(crate::protocol::FollowUpPayload {
                notice_delivery_identity: CorrelationIdentity::parse("notice-001").unwrap(),
                ledger_frontier: crate::protocol::LedgerFrontier::parse(0).unwrap(),
                text: "too early".into(),
            }),
        };
        assert_eq!(peer.admit_inbound(early), Err(PeerError::InvalidTransition));
        assert_eq!(peer.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn settled_requires_prompt_usage_and_terminal_events() {
        let mut peer = setup_ready();
        admit_prompt(&mut peer);
        peer.observe_outbound(frame(
            5,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AgentStart,
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            6,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AgentEnd {
                    messages: vec![assistant_message("stop")],
                    will_retry: false,
                },
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            7,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AgentSettled,
            },
        ))
        .unwrap();
        let settled = frame(
            8,
            Some("prompt-001"),
            OutboundEvent::Settled {
                classification: SettledClassification::Completed,
                final_assistant_outcome: FinalAssistantOutcome::Observed {
                    stop_reason: AssistantStopReason::Stop,
                },
            },
        );
        assert_eq!(
            peer.observe_outbound(settled),
            Err(PeerError::MissingTerminalEvidence)
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn fatal_is_a_caller_visible_terminal_observation() {
        let mut peer = setup_ready();
        let observation = peer
            .observe_outbound(frame(
                4,
                None,
                OutboundEvent::Fatal {
                    failure_code: AdapterFailureCode::ExecutionProfileDrift,
                },
            ))
            .unwrap();
        assert_eq!(
            observation,
            Some(PeerObservation::Fatal {
                failure_code: AdapterFailureCode::ExecutionProfileDrift
            })
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn materialized_header_only_transcript_cannot_claim_a_user_prompt() {
        let mut peer = setup_ready();
        peer.admit_inbound(InboundFrame {
            sequence: crate::protocol::BoundarySequence::parse(2).unwrap(),
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            correlation_identity: CorrelationIdentity::parse("dispose-001").unwrap(),
            command: InboundCommand::Dispose(crate::protocol::DisposePayload {
                reason: crate::protocol::DisposeReason::ProcessRecovery,
            }),
        })
        .unwrap();
        peer.observe_outbound(frame(
            4,
            Some("dispose-001"),
            OutboundEvent::CommandResult(CommandResult::Accepted {
                command: CommandName::Dispose,
                detail: crate::protocol::CommandResultDetail::Acknowledged,
            }),
        ))
        .unwrap();
        peer.observe_outbound(frame(
            5,
            Some("dispose-001"),
            OutboundEvent::UsageSnapshot {
                usage: zero_usage(),
            },
        ))
        .unwrap();
        let receipt = TranscriptFlushReceiptV1::Materialized {
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            session_file: AbsolutePath::parse("/tmp/peer/sessions/receipt.jsonl").unwrap(),
            session_file_blake3: digest_value(),
            header_cwd: AbsolutePath::parse("/tmp/peer/cwd").unwrap(),
            first_user_prompt: crate::protocol::FirstUserPromptReceipt::Verified {
                digest: digest_value(),
            },
        };
        assert_eq!(
            peer.observe_outbound(frame(
                6,
                Some("dispose-001"),
                OutboundEvent::Disposed {
                    transcript_flush_receipt: receipt
                }
            )),
            Err(PeerError::TranscriptReceipt)
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn materialized_header_only_transcript_with_absent_prompt_is_a_valid_dispose_receipt() {
        let mut peer = setup_ready();
        admit_dispose(&mut peer);
        peer.observe_outbound(frame(
            5,
            Some("dispose-001"),
            OutboundEvent::UsageSnapshot {
                usage: zero_usage(),
            },
        ))
        .unwrap();
        let receipt = TranscriptFlushReceiptV1::Materialized {
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            session_file: AbsolutePath::parse("/tmp/peer/sessions/receipt.jsonl").unwrap(),
            session_file_blake3: digest_value(),
            header_cwd: AbsolutePath::parse("/tmp/peer/cwd").unwrap(),
            first_user_prompt: crate::protocol::FirstUserPromptReceipt::Absent,
        };
        assert_eq!(
            peer.observe_outbound(frame(
                6,
                Some("dispose-001"),
                OutboundEvent::Disposed {
                    transcript_flush_receipt: receipt,
                },
            )),
            Ok(Some(PeerObservation::Disposed))
        );
        assert_eq!(peer.phase(), PeerPhase::Disposed);
    }

    #[test]
    fn disposed_without_the_forced_final_usage_snapshot_fences_the_peer() {
        let mut peer = setup_ready();
        admit_dispose(&mut peer);
        assert_eq!(
            peer.observe_outbound(frame(
                5,
                Some("dispose-001"),
                OutboundEvent::Disposed {
                    transcript_flush_receipt: unmaterialized_transcript_receipt(),
                },
            )),
            Err(PeerError::MissingTerminalEvidence)
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn dispose_usage_must_immediately_follow_its_accepted_result() {
        let mut peer = setup_ready();
        admit_dispose(&mut peer);
        // The transport sequence itself remains contiguous. This unrelated
        // schema-valid lifecycle frame is nevertheless forbidden between the
        // accepted Dispose result and its forced final usage snapshot.
        peer.observe_outbound(frame(
            5,
            None,
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AgentStart,
            },
        ))
        .unwrap();
        assert_eq!(
            peer.observe_outbound(frame(
                6,
                Some("dispose-001"),
                OutboundEvent::UsageSnapshot {
                    usage: zero_usage(),
                },
            )),
            Err(PeerError::MissingTerminalEvidence)
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn same_total_dispose_usage_is_required_terminal_evidence_even_without_a_delta() {
        let mut peer = setup_ready();
        admit_prompt(&mut peer);
        let settled_sequence = materialized_prompt_sequence(&mut peer, 5);
        peer.observe_outbound(frame(
            settled_sequence,
            Some("prompt-001"),
            OutboundEvent::Settled {
                classification: SettledClassification::Completed,
                final_assistant_outcome: FinalAssistantOutcome::Observed {
                    stop_reason: AssistantStopReason::Stop,
                },
            },
        ))
        .unwrap();
        peer.admit_inbound(InboundFrame {
            sequence: crate::protocol::BoundarySequence::parse(3).unwrap(),
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            correlation_identity: CorrelationIdentity::parse("dispose-001").unwrap(),
            command: InboundCommand::Dispose(crate::protocol::DisposePayload {
                reason: crate::protocol::DisposeReason::ProcessRecovery,
            }),
        })
        .unwrap();
        peer.observe_outbound(frame(
            settled_sequence + 1,
            Some("dispose-001"),
            OutboundEvent::CommandResult(CommandResult::Accepted {
                command: CommandName::Dispose,
                detail: crate::protocol::CommandResultDetail::Acknowledged,
            }),
        ))
        .unwrap();
        assert_eq!(
            peer.observe_outbound(frame(
                settled_sequence + 2,
                Some("dispose-001"),
                OutboundEvent::UsageSnapshot {
                    usage: zero_usage(),
                },
            )),
            Ok(Some(PeerObservation::Usage(UsageDelta {
                input_tokens: 0,
                output_tokens: 0,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
                total_tokens: 0,
                micro_usd: crate::cost::UsdMicros::ZERO,
                idempotent: true,
            })))
        );
        assert_eq!(
            peer.observe_outbound(frame(
                settled_sequence + 3,
                Some("dispose-001"),
                OutboundEvent::Disposed {
                    transcript_flush_receipt: materialized_transcript_receipt(),
                },
            )),
            Ok(Some(PeerObservation::Disposed))
        );
        assert_eq!(peer.phase(), PeerPhase::Disposed);
    }

    #[test]
    fn usage_after_prompt_produces_a_typed_normalized_delta() {
        let mut peer = setup_ready();
        admit_prompt(&mut peer);
        let usage = UsageObservation::Known(UsageTotals {
            input_tokens: NonNegativeInteger::parse(0).unwrap(),
            output_tokens: NonNegativeInteger::parse(0).unwrap(),
            cache_read_tokens: NonNegativeInteger::parse(0).unwrap(),
            cache_write_tokens: NonNegativeInteger::parse(0).unwrap(),
            total_tokens: NonNegativeInteger::parse(0).unwrap(),
            provider_cost: ProviderCostObservationV1 {
                binary64_big_endian_hex: Binary64BigEndianHex::parse("0000000000000000").unwrap(),
            },
        });
        let observation = peer
            .observe_outbound(frame(
                5,
                Some("prompt-001"),
                OutboundEvent::UsageSnapshot { usage },
            ))
            .unwrap();
        assert!(
            matches!(observation, Some(PeerObservation::Usage(delta)) if delta.micro_usd.value() == 0)
        );
    }

    #[test]
    fn unavailable_usage_is_visible_then_only_host_fatal_may_follow() {
        let mut peer = setup_ready();
        admit_prompt(&mut peer);
        let observation = peer
            .observe_outbound(frame(
                5,
                Some("prompt-001"),
                OutboundEvent::UsageSnapshot {
                    usage: UsageObservation::Unavailable(
                        crate::protocol::UsageUnavailableReason::InvalidSdkUsage,
                    ),
                },
            ))
            .unwrap();
        assert_eq!(
            observation,
            Some(PeerObservation::UsageUnavailable {
                reason: crate::protocol::UsageUnavailableReason::InvalidSdkUsage
            })
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
        let terminal = peer
            .observe_outbound(frame(
                6,
                None,
                OutboundEvent::Fatal {
                    failure_code: AdapterFailureCode::SdkOperationFailed,
                },
            ))
            .unwrap();
        assert_eq!(
            terminal,
            Some(PeerObservation::Fatal {
                failure_code: AdapterFailureCode::SdkOperationFailed
            })
        );
    }

    #[test]
    fn retry_and_compaction_fixture_settles_only_after_the_final_nonretry_end() {
        let mut peer = setup_ready();
        admit_prompt(&mut peer);
        peer.observe_outbound(frame(
            5,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AgentStart,
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            6,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AutoRetryStart {
                    attempt: NonNegativeInteger::parse(1).unwrap(),
                    max_attempts: NonNegativeInteger::parse(2).unwrap(),
                    delay_milliseconds: NonNegativeInteger::parse(2_000).unwrap(),
                    error_message: "temporary provider error".into(),
                },
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            7,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AgentEnd {
                    messages: vec![assistant_message("error")],
                    will_retry: true,
                },
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            8,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::CompactionStart {
                    reason: crate::protocol::CompactionReason::Threshold,
                },
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            9,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::CompactionEnd {
                    reason: crate::protocol::CompactionReason::Threshold,
                    result: None,
                    aborted: false,
                    will_retry: false,
                    error_message: None,
                },
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            10,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AgentEnd {
                    messages: vec![assistant_message("stop")],
                    will_retry: false,
                },
            },
        ))
        .unwrap();
        peer.observe_outbound(frame(
            11,
            Some("prompt-001"),
            OutboundEvent::AgentEvent {
                agent_event: ProjectedAgentEvent::AgentSettled,
            },
        ))
        .unwrap();
        let usage = UsageObservation::Known(UsageTotals {
            input_tokens: NonNegativeInteger::parse(0).unwrap(),
            output_tokens: NonNegativeInteger::parse(0).unwrap(),
            cache_read_tokens: NonNegativeInteger::parse(0).unwrap(),
            cache_write_tokens: NonNegativeInteger::parse(0).unwrap(),
            total_tokens: NonNegativeInteger::parse(0).unwrap(),
            provider_cost: ProviderCostObservationV1 {
                binary64_big_endian_hex: Binary64BigEndianHex::parse("0000000000000000").unwrap(),
            },
        });
        peer.observe_outbound(frame(
            12,
            Some("prompt-001"),
            OutboundEvent::UsageSnapshot { usage },
        ))
        .unwrap();
        let observation = peer
            .observe_outbound(frame(
                13,
                Some("prompt-001"),
                OutboundEvent::Settled {
                    classification: SettledClassification::Completed,
                    final_assistant_outcome: FinalAssistantOutcome::Observed {
                        stop_reason: AssistantStopReason::Stop,
                    },
                },
            ))
            .unwrap();
        assert!(matches!(
            observation,
            Some(PeerObservation::TurnSettled(TurnReceipt {
                disposition: TurnDisposition::Completed,
                ..
            }))
        ));
        assert_eq!(peer.settled_turns().len(), 1);
        assert_eq!(peer.phase(), PeerPhase::Ready);
    }

    #[test]
    fn correlation_and_sequence_mismatch_are_terminal() {
        let mut peer = setup_ready();
        let bad_correlation = frame(
            4,
            Some("unknown-correlation"),
            OutboundEvent::CommandResult(CommandResult::Accepted {
                command: CommandName::GetState,
                detail: crate::protocol::CommandResultDetail::Acknowledged,
            }),
        );
        assert_eq!(
            peer.observe_outbound(bad_correlation),
            Err(PeerError::UnknownCorrelation)
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
        let mut peer = setup_ready();
        let skipped_sequence = frame(
            5,
            None,
            OutboundEvent::Fatal {
                failure_code: AdapterFailureCode::ProtocolDecodeFailed,
            },
        );
        assert_eq!(
            peer.observe_outbound(skipped_sequence),
            Err(PeerError::OutboundSequence)
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
    }

    #[test]
    fn create_rejects_agent_escape_paths() {
        let mut peer = BoundaryPeer::new(
            SessionIdentity::parse("peer-session-001").unwrap(),
            HostProcessId::parse(42).unwrap(),
            SpawnNonce::parse("peer-nonce-001").unwrap(),
            runtime(),
        )
        .unwrap();
        peer.observe_outbound(frame(
            1,
            None,
            OutboundEvent::AdapterReady {
                pid: HostProcessId::parse(42).unwrap(),
                spawn_nonce: SpawnNonce::parse("peer-nonce-001").unwrap(),
                runtime: runtime(),
            },
        ))
        .unwrap();
        let mut escaped = create();
        escaped.auth_path = AbsolutePath::parse("/tmp/not-owned/auth.json").unwrap();
        let command = InboundFrame {
            sequence: crate::protocol::BoundarySequence::parse(1).unwrap(),
            session_identity: SessionIdentity::parse("peer-session-001").unwrap(),
            correlation_identity: CorrelationIdentity::parse("create-001").unwrap(),
            command: InboundCommand::CreateSession(Box::new(escaped)),
        };
        assert_eq!(
            peer.admit_inbound(command),
            Err(PeerError::ExecutionProfileDrift)
        );
        assert_eq!(peer.phase(), PeerPhase::Fatal);
    }
}
