//! The complete closed `society-pi-host/v4` wire schema.
//!
//! `miniserde::json::Value` is used only after [`reject_duplicate_object_keys`]
//! has ruled out JSON's last-key-wins ambiguity. The subsequent decoder checks
//! every object for exact keys and converts every discriminant into a Rust
//! enum. JSON-safe Pi evidence is intentionally represented by `Value`: it is
//! sealed forensic content, not a generic workflow payload.

use std::{collections::BTreeSet, fmt, path::Path};

use miniserde::json::{Array, Number, Object, Value};
use thiserror::Error;

use crate::forum::ForumSessionContractV1;

pub const ADAPTER_PROTOCOL_VERSION: &str = "society-pi-host/v4";
pub const ADAPTER_VERSION: &str = "1";
pub const PINNED_PI_SDK_VERSION: &str = "0.83.0";
pub const PINNED_PROVIDER: &str = "openrouter";
pub const PINNED_MODEL: &str = "deepseek/deepseek-v4-flash-0731";
pub const PINNED_CANONICAL_MODEL_SLUG: &str = "deepseek/deepseek-v4-flash-20260731";
pub const PINNED_OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
pub const PINNED_THINKING_LEVEL: &str = "high";
pub const MAX_JSONL_FRAME_BYTES: usize = 1024 * 1024;
/// The duplicate-key pre-scan is deliberately bounded before it descends.
///
/// JSON itself has no practical nesting limit, but the v1 boundary does. A
/// deeply nested sub-megabyte record must become a typed protocol failure, not
/// consume an unbounded Rust or Node stack before the exact frame is sealed.
pub const MAX_JSON_NESTING: usize = 128;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ProtocolError {
    #[error("JSONL frame exceeds the v1 byte limit")]
    FrameTooLarge,
    #[error("invalid JSON")]
    InvalidJson,
    #[error("JSON exceeds the v1 nesting limit")]
    NestingTooDeep,
    #[error("JSONL record contains a delimiter or carriage return")]
    InvalidJsonlLine,
    #[error("JSONL record is not valid UTF-8")]
    InvalidUtf8,
    #[error("duplicate JSON object key")]
    DuplicateObjectKey,
    #[error("noncanonical JSON negative zero")]
    NegativeZero,
    #[error("invalid closed protocol frame: {0}")]
    InvalidFrame(&'static str),
}

macro_rules! closed_string {
    ($name:ident, $what:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, ProtocolError> {
                let value = value.into();
                if !is_identifier(&value) {
                    return Err(ProtocolError::InvalidFrame($what));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

closed_string!(SessionIdentity, "session identity");
closed_string!(CorrelationIdentity, "correlation identity");
closed_string!(SpawnNonce, "spawn nonce");
closed_string!(ToolCallIdentity, "tool call identity");
closed_string!(BashExecutionIdentity, "bash execution identity");

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BoundarySequence(u64);
impl BoundarySequence {
    pub fn parse(value: u64) -> Result<Self, ProtocolError> {
        if value == 0 || value > MAX_SAFE_INTEGER {
            return Err(ProtocolError::InvalidFrame("boundary sequence"));
        }
        Ok(Self(value))
    }
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LedgerFrontier(u64);
impl LedgerFrontier {
    pub fn parse(value: u64) -> Result<Self, ProtocolError> {
        if value > MAX_SAFE_INTEGER {
            return Err(ProtocolError::InvalidFrame("ledger frontier"));
        }
        Ok(Self(value))
    }
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NonNegativeInteger(u64);
impl NonNegativeInteger {
    pub fn parse(value: u64) -> Result<Self, ProtocolError> {
        if value > MAX_SAFE_INTEGER {
            return Err(ProtocolError::InvalidFrame("nonnegative integer"));
        }
        Ok(Self(value))
    }
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PositiveInteger(u64);
impl PositiveInteger {
    pub fn parse(value: u64) -> Result<Self, ProtocolError> {
        if value == 0 || value > MAX_SAFE_INTEGER {
            return Err(ProtocolError::InvalidFrame("positive integer"));
        }
        Ok(Self(value))
    }
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// The exact supervised OS child identity echoed by `AdapterReady`. This is
/// intentionally distinct from generic positive protocol quantities: a PID
/// participates in process-ownership proof and may not be substituted with a
/// retry count, sequence, or SDK-provided number.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HostProcessId(PositiveInteger);
impl HostProcessId {
    pub fn parse(value: u64) -> Result<Self, ProtocolError> {
        Ok(Self(PositiveInteger::parse(value)?))
    }
    pub const fn value(self) -> u64 {
        self.0.value()
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Blake3Digest(String);
impl Blake3Digest {
    pub fn parse(value: impl Into<String>) -> Result<Self, ProtocolError> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ProtocolError::InvalidFrame("blake3 digest"));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AbsolutePath(String);
impl AbsolutePath {
    /// Matches the host's POSIX `isAbsolute(normalize(path) == path)` rule
    /// while also rejecting NULs and parent traversal before filesystem use.
    pub fn parse(value: impl Into<String>) -> Result<Self, ProtocolError> {
        let value = value.into();
        if value.len() < 2
            || !value.starts_with('/')
            || value.contains('\0')
            || value.ends_with('/')
        {
            return Err(ProtocolError::InvalidFrame("absolute path"));
        }
        if value
            .split('/')
            .skip(1)
            .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(ProtocolError::InvalidFrame("absolute path"));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn is_strict_descendant_of(&self, base: &Self) -> bool {
        let base = base.as_str();
        self.0.starts_with(base) && self.0.as_bytes().get(base.len()) == Some(&b'/')
    }
    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UsdPerMillionDecimal(String);
impl UsdPerMillionDecimal {
    pub fn parse(value: impl Into<String>) -> Result<Self, ProtocolError> {
        let value = value.into();
        let bytes = value.as_bytes();
        let nonzero = if let Some((whole, fraction)) = value.split_once('.') {
            !whole.is_empty()
                && !fraction.is_empty()
                && fraction.bytes().all(|byte| byte.is_ascii_digit())
                && fraction.bytes().any(|byte| byte != b'0')
        } else {
            value != "0"
        };
        let canonical_whole = match bytes.first() {
            Some(b'0') => value == "0" || value.starts_with("0."),
            Some(byte) if byte.is_ascii_digit() => value
                .bytes()
                .take_while(|byte| *byte != b'.')
                .all(|byte| byte.is_ascii_digit()),
            _ => false,
        };
        if !nonzero
            || !canonical_whole
            || value.starts_with('0') && !value.starts_with("0.") && value != "0"
            || value.chars().filter(|character| *character == '.').count() > 1
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'.')
        {
            return Err(ProtocolError::InvalidFrame("USD per million decimal"));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NodeRuntimeVersion(String);
impl NodeRuntimeVersion {
    pub fn parse(value: impl Into<String>) -> Result<Self, ProtocolError> {
        let value = value.into();
        let without_v = value.strip_prefix('v').unwrap_or(&value);
        let mut parts = without_v.split('.');
        let (Some(major), Some(minor), Some(patch), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(ProtocolError::InvalidFrame("Node runtime version"));
        };
        let parse = |part: &str| {
            part.parse::<u64>()
                .ok()
                .filter(|number| *number <= MAX_SAFE_INTEGER)
        };
        let (Some(major), Some(minor), Some(_patch)) = (parse(major), parse(minor), parse(patch))
        else {
            return Err(ProtocolError::InvalidFrame("Node runtime version"));
        };
        if major < 22 || major == 22 && minor < 19 {
            return Err(ProtocolError::InvalidFrame("Node runtime version"));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionKind {
    TaskAttempt,
    RootAuthorityOffice,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolProfile {
    ReadExecuteV1,
    ReadWriteV1,
    WorkspaceMutationV1,
    /// Pi's file tools are replaced with canonical operations rooted at cwd.
    /// This profile has no shell or subprocess-backed search tool.
    WorkspaceIsolatedV1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PiToolName {
    Read,
    Bash,
    Edit,
    Write,
    Grep,
    Find,
    Ls,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueMode {
    All,
    OneAtATime,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompactionMode {
    Enabled,
    Disabled,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromptPurpose {
    TaskAssignment,
    OfficeTurn,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SteerReason {
    UrgentStalePremise,
    UrgentUnsafePremise,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbortReason {
    GracefulCancellation,
    EmergencyStop,
    BudgetGuardrail,
    DaemonRecovery,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisposeReason {
    CycleReconciliation,
    ProcessRecovery,
    ProtocolFailure,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Provider {
    OpenRouter,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelId {
    DeepseekV4Flash0731,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalModelSlug {
    DeepseekV4Flash20260731,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpenRouterBaseUrl {
    ApiV1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelApi {
    OpenAiCompletions,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelInput {
    TextOnly,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Transport {
    Sse,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectTrust {
    Never,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disabled {
    Disabled,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Images {
    Blocked,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterVersion {
    V1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PiSdkVersion {
    V0830,
}

impl ToolProfile {
    pub const fn tools(self) -> &'static [PiToolName] {
        match self {
            Self::ReadExecuteV1 => &[
                PiToolName::Read,
                PiToolName::Bash,
                PiToolName::Grep,
                PiToolName::Find,
                PiToolName::Ls,
            ],
            Self::ReadWriteV1 => &[PiToolName::Read, PiToolName::Write],
            Self::WorkspaceMutationV1 => &[
                PiToolName::Read,
                PiToolName::Bash,
                PiToolName::Edit,
                PiToolName::Write,
                PiToolName::Grep,
                PiToolName::Find,
                PiToolName::Ls,
            ],
            Self::WorkspaceIsolatedV1 => &[
                PiToolName::Read,
                PiToolName::Edit,
                PiToolName::Write,
                PiToolName::Ls,
            ],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryPolicyV1 {
    pub max_retries: NonNegativeInteger,
    pub base_delay_milliseconds: NonNegativeInteger,
    pub provider_timeout_milliseconds: PositiveInteger,
    pub provider_max_retries: NonNegativeInteger,
    pub provider_max_retry_delay_milliseconds: PositiveInteger,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompactionPolicyV1 {
    pub mode: CompactionMode,
    pub reserve_tokens: NonNegativeInteger,
    pub keep_recent_tokens: NonNegativeInteger,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorModelPolicyV1 {
    pub retry: RetryPolicyV1,
    pub compaction: CompactionPolicyV1,
    pub steering_mode: QueueMode,
    pub follow_up_mode: QueueMode,
    pub transport: Transport,
    pub project_trust: ProjectTrust,
    pub install_telemetry: Disabled,
    pub analytics: Disabled,
    pub images: Images,
}
impl ActorModelPolicyV1 {
    /// Rejects any deviation from the current pinned Pi SDK actor policy.
    pub fn assert_pinned(&self) -> Result<(), ProtocolError> {
        let retry = &self.retry;
        let compaction = &self.compaction;
        if retry.max_retries.value() != 2
            || retry.base_delay_milliseconds.value() != 2_000
            || retry.provider_timeout_milliseconds.value() != 300_000
            || retry.provider_max_retries.value() != 1
            || retry.provider_max_retry_delay_milliseconds.value() != 30_000
            || compaction.mode != CompactionMode::Enabled
            || compaction.reserve_tokens.value() != 16_384
            || compaction.keep_recent_tokens.value() != 20_000
            || self.steering_mode != QueueMode::OneAtATime
            || self.follow_up_mode != QueueMode::OneAtATime
            || self.transport != Transport::Sse
            || self.project_trust != ProjectTrust::Never
            || self.install_telemetry != Disabled::Disabled
            || self.analytics != Disabled::Disabled
            || self.images != Images::Blocked
        {
            return Err(ProtocolError::InvalidFrame("pinned actor model policy"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelSelection {
    pub provider: Provider,
    pub model_id: ModelId,
    pub thinking_level: ThinkingLevel,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnownPerMillionRateV1 {
    pub usd_per_million: UsdPerMillionDecimal,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CacheWritePerMillionRateV1 {
    Known(KnownPerMillionRateV1),
    Absent,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveModelDescriptorV1 {
    pub provider: Provider,
    pub base_url: OpenRouterBaseUrl,
    pub api: ModelApi,
    pub model_id: ModelId,
    pub canonical_slug: CanonicalModelSlug,
    pub input: ModelInput,
    pub context_window: PositiveInteger,
    pub max_tokens: PositiveInteger,
    pub input_usd_per_million: KnownPerMillionRateV1,
    pub output_usd_per_million: KnownPerMillionRateV1,
    pub cache_read_usd_per_million: KnownPerMillionRateV1,
    pub cache_write_usd_per_million: CacheWritePerMillionRateV1,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelCatalogPolicyV1 {
    pub catalog_blake3: Blake3Digest,
    pub effective_model: EffectiveModelDescriptorV1,
}
impl ModelCatalogPolicyV1 {
    /// Rejects any deviation from the current pinned Pi SDK model catalog.
    pub fn assert_pinned(&self) -> Result<(), ProtocolError> {
        let model = &self.effective_model;
        if model.provider != Provider::OpenRouter
            || model.base_url != OpenRouterBaseUrl::ApiV1
            || model.api != ModelApi::OpenAiCompletions
            || model.model_id != ModelId::DeepseekV4Flash0731
            || model.canonical_slug != CanonicalModelSlug::DeepseekV4Flash20260731
            || model.input != ModelInput::TextOnly
            || model.context_window.value() != 1_048_576
            || model.max_tokens.value() != 384_000
            || model.input_usd_per_million.usd_per_million.as_str() != "0.09"
            || model.output_usd_per_million.usd_per_million.as_str() != "0.18"
            || model.cache_read_usd_per_million.usd_per_million.as_str() != "0.018"
            || model.cache_write_usd_per_million != CacheWritePerMillionRateV1::Absent
        {
            return Err(ProtocolError::InvalidFrame("pinned model catalog policy"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateSessionPayload {
    pub session_kind: SessionKind,
    pub cwd: AbsolutePath,
    pub agent_directory: AbsolutePath,
    pub auth_path: AbsolutePath,
    pub models_path: AbsolutePath,
    pub session_directory: AbsolutePath,
    pub system_prompt: String,
    pub system_prompt_digest: Blake3Digest,
    pub model: ModelSelection,
    pub model_catalog: ModelCatalogPolicyV1,
    pub tool_profile: ToolProfile,
    pub settings: ActorModelPolicyV1,
    pub forum_contract: ForumSessionContractV1,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptPayload {
    pub purpose: PromptPurpose,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FollowUpPayload {
    pub notice_delivery_identity: CorrelationIdentity,
    pub ledger_frontier: LedgerFrontier,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SteerPayload {
    pub reason: SteerReason,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbortPayload {
    pub reason: AbortReason,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisposePayload {
    pub reason: DisposeReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InboundCommand {
    CreateSession(Box<CreateSessionPayload>),
    Prompt(PromptPayload),
    FollowUp(FollowUpPayload),
    Steer(SteerPayload),
    Abort(AbortPayload),
    GetState,
    Dispose(DisposePayload),
}
impl InboundCommand {
    pub const fn name(&self) -> CommandName {
        match self {
            Self::CreateSession(_) => CommandName::CreateSession,
            Self::Prompt(_) => CommandName::Prompt,
            Self::FollowUp(_) => CommandName::FollowUp,
            Self::Steer(_) => CommandName::Steer,
            Self::Abort(_) => CommandName::Abort,
            Self::GetState => CommandName::GetState,
            Self::Dispose(_) => CommandName::Dispose,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandName {
    CreateSession,
    Prompt,
    FollowUp,
    Steer,
    Abort,
    GetState,
    Dispose,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InboundFrame {
    pub sequence: BoundarySequence,
    pub session_identity: SessionIdentity,
    pub correlation_identity: CorrelationIdentity,
    pub command: InboundCommand,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterPhase {
    Inert,
    Creating,
    Ready,
    Closing,
    Disposed,
    Fatal,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterFailureCode {
    InvalidCommand,
    InvalidState,
    SequenceGap,
    SessionIdentityMismatch,
    ExecutionProfileDrift,
    SdkOperationFailed,
    MissingAgentSettled,
    MissingFinalAssistantOutcome,
    ProtocolDecodeFailed,
    OutboundFrameTooLarge,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeIdentity {
    pub node_version: NodeRuntimeVersion,
    pub adapter_version: AdapterVersion,
    pub pi_sdk_version: PiSdkVersion,
    pub node_executable_blake3: Blake3Digest,
    pub lockfile_blake3: Blake3Digest,
    pub adapter_build_blake3: Blake3Digest,
    pub pi_transitive_package_set_blake3: Blake3Digest,
}
impl RuntimeIdentity {
    pub fn assert_v1(&self) -> Result<(), ProtocolError> {
        if self.adapter_version != AdapterVersion::V1 || self.pi_sdk_version != PiSdkVersion::V0830
        {
            return Err(ProtocolError::InvalidFrame("runtime identity"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveSessionConfiguration {
    pub session_kind: SessionKind,
    pub cwd: AbsolutePath,
    pub session_directory: AbsolutePath,
    pub session_file: AbsolutePath,
    pub model: ModelSelection,
    pub model_catalog: ModelCatalogPolicyV1,
    pub tool_profile: ToolProfile,
    pub tools: Vec<PiToolName>,
    pub settings: ActorModelPolicyV1,
    pub forum_contract: ForumSessionContractV1,
}
impl EffectiveSessionConfiguration {
    /// Rejects any configuration that does not use the current pinned Pi SDK
    /// model and actor policies.
    pub fn assert_pinned(&self) -> Result<(), ProtocolError> {
        self.model_catalog.assert_pinned()?;
        self.settings.assert_pinned()?;
        self.forum_contract.assert_pinned()?;
        if self.model.provider != Provider::OpenRouter
            || self.model.model_id != ModelId::DeepseekV4Flash0731
            || self.model.thinking_level != ThinkingLevel::High
            || self.tools.as_slice() != self.tool_profile.tools()
        {
            return Err(ProtocolError::InvalidFrame(
                "effective session configuration",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderCostObservationV1 {
    pub binary64_big_endian_hex: Binary64BigEndianHex,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binary64BigEndianHex(String);
impl Binary64BigEndianHex {
    pub fn parse(value: impl Into<String>) -> Result<Self, ProtocolError> {
        let value = value.into();
        if value.len() != 16
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ProtocolError::InvalidFrame("binary64 cost"));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsageTotals {
    pub input_tokens: NonNegativeInteger,
    pub output_tokens: NonNegativeInteger,
    pub cache_read_tokens: NonNegativeInteger,
    pub cache_write_tokens: NonNegativeInteger,
    pub total_tokens: NonNegativeInteger,
    pub provider_cost: ProviderCostObservationV1,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UsageObservation {
    Known(UsageTotals),
    Unavailable(UsageUnavailableReason),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UsageUnavailableReason {
    InvalidSdkUsage,
    UsageRegressed,
    UsageInconsistent,
}

#[derive(Clone, Debug)]
pub enum ProjectedAgentEvent {
    AgentStart,
    AgentEnd {
        messages: Vec<Value>,
        will_retry: bool,
    },
    AgentSettled,
    TurnStart,
    TurnEnd {
        message: Value,
        tool_results: Vec<Value>,
    },
    MessageStart {
        message: Value,
    },
    MessageUpdate {
        message: Value,
        assistant_message_event: Value,
    },
    MessageEnd {
        message: Value,
    },
    ToolExecutionStart {
        tool_call_identity: ToolCallIdentity,
        tool_name: PiToolName,
        args: Value,
    },
    ToolExecutionUpdate {
        tool_call_identity: ToolCallIdentity,
        tool_name: PiToolName,
        args: Value,
        partial_result: Value,
    },
    ToolExecutionEnd {
        tool_call_identity: ToolCallIdentity,
        tool_name: PiToolName,
        result: Value,
        is_error: bool,
    },
    QueueUpdate {
        steering: Vec<String>,
        follow_up: Vec<String>,
    },
    EntryAppended {
        entry: Value,
    },
    BashExecutionUpdate {
        execution_identity: Option<BashExecutionIdentity>,
        delta: String,
    },
    CompactionStart {
        reason: CompactionReason,
    },
    SessionInfoChanged {
        name: Option<String>,
    },
    ThinkingLevelChanged {
        level: ThinkingLevel,
    },
    CompactionEnd {
        reason: CompactionReason,
        result: Option<Value>,
        aborted: bool,
        will_retry: bool,
        error_message: Option<String>,
    },
    AutoRetryStart {
        attempt: NonNegativeInteger,
        max_attempts: NonNegativeInteger,
        delay_milliseconds: NonNegativeInteger,
        error_message: String,
    },
    AutoRetryEnd {
        success: bool,
        attempt: NonNegativeInteger,
        final_error: Option<String>,
    },
    SummarizationRetryScheduled {
        attempt: NonNegativeInteger,
        max_attempts: NonNegativeInteger,
        delay_milliseconds: NonNegativeInteger,
        error_message: String,
    },
    SummarizationRetryAttemptStart {
        source: SummarizationSource,
        reason: Option<CompactionReason>,
    },
    SummarizationRetryFinished,
}

fn json_value_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Number(left), Value::Number(right)) => match (left, right) {
            (Number::U64(left), Number::U64(right)) => left == right,
            (Number::I64(left), Number::I64(right)) => left == right,
            (Number::F64(left), Number::F64(right)) => left == right,
            _ => false,
        },
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| json_value_equal(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right
                        .get(key)
                        .is_some_and(|other| json_value_equal(value, other))
                })
        }
        _ => false,
    }
}

impl PartialEq for ProjectedAgentEvent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::AgentStart, Self::AgentStart)
            | (Self::AgentSettled, Self::AgentSettled)
            | (Self::TurnStart, Self::TurnStart)
            | (Self::SummarizationRetryFinished, Self::SummarizationRetryFinished) => true,
            (
                Self::AgentEnd {
                    messages: left_messages,
                    will_retry: left_retry,
                },
                Self::AgentEnd {
                    messages: right_messages,
                    will_retry: right_retry,
                },
            ) => {
                left_retry == right_retry
                    && left_messages.len() == right_messages.len()
                    && left_messages
                        .iter()
                        .zip(right_messages)
                        .all(|(left, right)| json_value_equal(left, right))
            }
            (
                Self::TurnEnd {
                    message: left,
                    tool_results: left_results,
                },
                Self::TurnEnd {
                    message: right,
                    tool_results: right_results,
                },
            ) => {
                json_value_equal(left, right)
                    && left_results.len() == right_results.len()
                    && left_results
                        .iter()
                        .zip(right_results)
                        .all(|(left, right)| json_value_equal(left, right))
            }
            (Self::MessageStart { message: left }, Self::MessageStart { message: right })
            | (Self::MessageEnd { message: left }, Self::MessageEnd { message: right })
            | (Self::EntryAppended { entry: left }, Self::EntryAppended { entry: right }) => {
                json_value_equal(left, right)
            }
            (
                Self::MessageUpdate {
                    message: left_message,
                    assistant_message_event: left_event,
                },
                Self::MessageUpdate {
                    message: right_message,
                    assistant_message_event: right_event,
                },
            ) => {
                json_value_equal(left_message, right_message)
                    && json_value_equal(left_event, right_event)
            }
            (
                Self::ToolExecutionStart {
                    tool_call_identity: left_id,
                    tool_name: left_name,
                    args: left_args,
                },
                Self::ToolExecutionStart {
                    tool_call_identity: right_id,
                    tool_name: right_name,
                    args: right_args,
                },
            ) => {
                left_id == right_id
                    && left_name == right_name
                    && json_value_equal(left_args, right_args)
            }
            (
                Self::ToolExecutionUpdate {
                    tool_call_identity: left_id,
                    tool_name: left_name,
                    args: left_args,
                    partial_result: left_result,
                },
                Self::ToolExecutionUpdate {
                    tool_call_identity: right_id,
                    tool_name: right_name,
                    args: right_args,
                    partial_result: right_result,
                },
            ) => {
                left_id == right_id
                    && left_name == right_name
                    && json_value_equal(left_args, right_args)
                    && json_value_equal(left_result, right_result)
            }
            (
                Self::ToolExecutionEnd {
                    tool_call_identity: left_id,
                    tool_name: left_name,
                    result: left_result,
                    is_error: left_error,
                },
                Self::ToolExecutionEnd {
                    tool_call_identity: right_id,
                    tool_name: right_name,
                    result: right_result,
                    is_error: right_error,
                },
            ) => {
                left_id == right_id
                    && left_name == right_name
                    && left_error == right_error
                    && json_value_equal(left_result, right_result)
            }
            (
                Self::QueueUpdate {
                    steering: left_steering,
                    follow_up: left_follow_up,
                },
                Self::QueueUpdate {
                    steering: right_steering,
                    follow_up: right_follow_up,
                },
            ) => left_steering == right_steering && left_follow_up == right_follow_up,
            (
                Self::BashExecutionUpdate {
                    execution_identity: left_id,
                    delta: left_delta,
                },
                Self::BashExecutionUpdate {
                    execution_identity: right_id,
                    delta: right_delta,
                },
            ) => left_id == right_id && left_delta == right_delta,
            (Self::CompactionStart { reason: left }, Self::CompactionStart { reason: right }) => {
                left == right
            }
            (
                Self::ThinkingLevelChanged { level: left },
                Self::ThinkingLevelChanged { level: right },
            ) => left == right,
            (Self::SessionInfoChanged { name: left }, Self::SessionInfoChanged { name: right }) => {
                left == right
            }
            (
                Self::CompactionEnd {
                    reason: left_reason,
                    result: left_result,
                    aborted: left_aborted,
                    will_retry: left_retry,
                    error_message: left_error,
                },
                Self::CompactionEnd {
                    reason: right_reason,
                    result: right_result,
                    aborted: right_aborted,
                    will_retry: right_retry,
                    error_message: right_error,
                },
            ) => {
                left_reason == right_reason
                    && left_aborted == right_aborted
                    && left_retry == right_retry
                    && left_error == right_error
                    && match (left_result.as_ref(), right_result.as_ref()) {
                        (Some(left), Some(right)) => json_value_equal(left, right),
                        (None, None) => true,
                        _ => false,
                    }
            }
            (
                Self::AutoRetryStart {
                    attempt: left_attempt,
                    max_attempts: left_max,
                    delay_milliseconds: left_delay,
                    error_message: left_error,
                },
                Self::AutoRetryStart {
                    attempt: right_attempt,
                    max_attempts: right_max,
                    delay_milliseconds: right_delay,
                    error_message: right_error,
                },
            ) => {
                left_attempt == right_attempt
                    && left_max == right_max
                    && left_delay == right_delay
                    && left_error == right_error
            }
            (
                Self::AutoRetryEnd {
                    success: left_success,
                    attempt: left_attempt,
                    final_error: left_error,
                },
                Self::AutoRetryEnd {
                    success: right_success,
                    attempt: right_attempt,
                    final_error: right_error,
                },
            ) => {
                left_success == right_success
                    && left_attempt == right_attempt
                    && left_error == right_error
            }
            (
                Self::SummarizationRetryScheduled {
                    attempt: left_attempt,
                    max_attempts: left_max,
                    delay_milliseconds: left_delay,
                    error_message: left_error,
                },
                Self::SummarizationRetryScheduled {
                    attempt: right_attempt,
                    max_attempts: right_max,
                    delay_milliseconds: right_delay,
                    error_message: right_error,
                },
            ) => {
                left_attempt == right_attempt
                    && left_max == right_max
                    && left_delay == right_delay
                    && left_error == right_error
            }
            (
                Self::SummarizationRetryAttemptStart {
                    source: left_source,
                    reason: left_reason,
                },
                Self::SummarizationRetryAttemptStart {
                    source: right_source,
                    reason: right_reason,
                },
            ) => left_source == right_source && left_reason == right_reason,
            _ => false,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompactionReason {
    Manual,
    Threshold,
    Overflow,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SummarizationSource {
    BranchSummary,
    Compaction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionLiveness {
    Inert,
    Creating,
    Idle,
    Active,
    Closing,
    Disposed,
    Fatal,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandResultDetail {
    Acknowledged,
    State {
        phase: AdapterPhase,
        liveness: SessionLiveness,
        session_file: Option<AbsolutePath>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandResult {
    Accepted {
        command: CommandName,
        detail: CommandResultDetail,
    },
    Rejected {
        command: CommandName,
        failure_code: AdapterFailureCode,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettledClassification {
    Completed,
    Length,
    Error,
    Aborted,
    Failed,
    ProtocolFailed,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FinalAssistantOutcome {
    Observed {
        stop_reason: AssistantStopReason,
    },
    Unavailable {
        reason: FinalAssistantUnavailableReason,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssistantStopReason {
    Stop,
    Length,
    Error,
    Aborted,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FinalAssistantUnavailableReason {
    SdkPromiseRejected,
    MissingFinalAssistantOutcome,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FirstUserPromptReceipt {
    Absent,
    Verified { digest: Blake3Digest },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TranscriptFlushReceiptV1 {
    Materialized {
        session_identity: SessionIdentity,
        session_file: AbsolutePath,
        session_file_blake3: Blake3Digest,
        header_cwd: AbsolutePath,
        first_user_prompt: FirstUserPromptReceipt,
    },
    UnmaterializedNoPrompt {
        session_identity: SessionIdentity,
        session_file: AbsolutePath,
    },
}
// SessionReady deliberately carries the complete typed effective profile in
// one boundary event; boxing it would make the public event shape less direct.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum OutboundEvent {
    AdapterReady {
        pid: HostProcessId,
        spawn_nonce: SpawnNonce,
        runtime: RuntimeIdentity,
    },
    SessionReady {
        configuration: EffectiveSessionConfiguration,
    },
    CommandResult(CommandResult),
    AgentEvent {
        agent_event: ProjectedAgentEvent,
    },
    UsageSnapshot {
        usage: UsageObservation,
    },
    Settled {
        classification: SettledClassification,
        final_assistant_outcome: FinalAssistantOutcome,
    },
    Disposed {
        transcript_flush_receipt: TranscriptFlushReceiptV1,
    },
    Fatal {
        failure_code: AdapterFailureCode,
    },
}

impl PartialEq for OutboundEvent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::AdapterReady {
                    pid: left_pid,
                    spawn_nonce: left_nonce,
                    runtime: left_runtime,
                },
                Self::AdapterReady {
                    pid: right_pid,
                    spawn_nonce: right_nonce,
                    runtime: right_runtime,
                },
            ) => {
                left_pid == right_pid && left_nonce == right_nonce && left_runtime == right_runtime
            }
            (
                Self::SessionReady {
                    configuration: left,
                },
                Self::SessionReady {
                    configuration: right,
                },
            ) => left == right,
            (Self::CommandResult(left), Self::CommandResult(right)) => left == right,
            (Self::AgentEvent { agent_event: left }, Self::AgentEvent { agent_event: right }) => {
                left == right
            }
            (Self::UsageSnapshot { usage: left }, Self::UsageSnapshot { usage: right }) => {
                match (left, right) {
                    (UsageObservation::Unavailable(left), UsageObservation::Unavailable(right)) => {
                        left == right
                    }
                    (UsageObservation::Known(left), UsageObservation::Known(right)) => {
                        left == right
                    }
                    _ => false,
                }
            }
            (
                Self::Settled {
                    classification: left_classification,
                    final_assistant_outcome: left_outcome,
                },
                Self::Settled {
                    classification: right_classification,
                    final_assistant_outcome: right_outcome,
                },
            ) => left_classification == right_classification && left_outcome == right_outcome,
            (
                Self::Disposed {
                    transcript_flush_receipt: left,
                },
                Self::Disposed {
                    transcript_flush_receipt: right,
                },
            ) => left == right,
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
#[derive(Clone, Debug, PartialEq)]
pub struct OutboundFrame {
    pub sequence: BoundarySequence,
    pub session_identity: SessionIdentity,
    pub correlation_identity: Option<CorrelationIdentity>,
    pub event: OutboundEvent,
}

// `miniserde::json::Object` is deliberately a closed, typed boundary here.
// These tiny constructors keep the wire writers explicit without bringing a
// general-purpose serialization framework back into the Pi protocol.
macro_rules! json_object {
    () => {
        Value::Object(Object::new())
    };
    ($($key:literal => $value:expr),* $(,)?) => {{
        let mut object = Object::new();
        $(object.insert($key.to_owned(), $value);)*
        Value::Object(object)
    }};
}
fn json_string(value: &str) -> Value {
    Value::String(value.to_owned())
}
fn json_u64(value: u64) -> Value {
    Value::Number(Number::U64(value))
}
fn json_bool(value: bool) -> Value {
    Value::Bool(value)
}

pub fn decode_inbound_jsonl(line: &str) -> Result<InboundFrame, ProtocolError> {
    let root = decode_frame_value(line)?;
    let frame = object(&root)?;
    exact_keys(
        frame,
        &[
            "protocolVersion",
            "sequence",
            "sessionIdentity",
            "correlationIdentity",
            "command",
            "payload",
        ],
    )?;
    literal(frame, "protocolVersion", ADAPTER_PROTOCOL_VERSION)?;
    let sequence = boundary_sequence(frame, "sequence")?;
    let session_identity = SessionIdentity::parse(string(frame, "sessionIdentity")?)?;
    let correlation_identity = CorrelationIdentity::parse(string(frame, "correlationIdentity")?)?;
    let name = command_name(string(frame, "command")?)?;
    let payload = object(required(frame, "payload")?)?;
    let command = match name {
        CommandName::CreateSession => {
            InboundCommand::CreateSession(Box::new(decode_create_session(payload)?))
        }
        CommandName::Prompt => InboundCommand::Prompt(decode_prompt(payload)?),
        CommandName::FollowUp => InboundCommand::FollowUp(decode_follow_up(payload)?),
        CommandName::Steer => InboundCommand::Steer(decode_steer(payload)?),
        CommandName::Abort => InboundCommand::Abort(decode_abort(payload)?),
        CommandName::GetState => {
            exact_keys(payload, &[])?;
            InboundCommand::GetState
        }
        CommandName::Dispose => InboundCommand::Dispose(decode_dispose(payload)?),
    };
    Ok(InboundFrame {
        sequence,
        session_identity,
        correlation_identity,
        command,
    })
}

/// Serializes only a closed Rust command. Re-decoding the produced JSONL makes
/// the writer subject to the same exact-key and pinned Pi SDK admission checks as a
/// received command, so hand-constructed Rust values cannot bypass the wire
/// contract before they reach the host's stdin.
pub fn encode_inbound_jsonl(frame: &InboundFrame) -> Result<String, ProtocolError> {
    let payload = match &frame.command {
        InboundCommand::CreateSession(payload) => encode_create_session(payload),
        InboundCommand::Prompt(payload) => {
            json_object!("purpose" => json_string(prompt_purpose_wire(payload.purpose)), "text" => json_string(&payload.text))
        }
        InboundCommand::FollowUp(payload) => {
            json_object!("noticeDeliveryIdentity" => json_string(payload.notice_delivery_identity.as_str()), "ledgerFrontier" => json_u64(payload.ledger_frontier.value()), "text" => json_string(&payload.text))
        }
        InboundCommand::Steer(payload) => {
            json_object!("reason" => json_string(steer_reason_wire(payload.reason)), "text" => json_string(&payload.text))
        }
        InboundCommand::Abort(payload) => {
            json_object!("reason" => json_string(abort_reason_wire(payload.reason)))
        }
        InboundCommand::GetState => json_object!(),
        InboundCommand::Dispose(payload) => {
            json_object!("reason" => json_string(dispose_reason_wire(payload.reason)))
        }
    };
    let line = miniserde::json::to_string(&json_object!(
        "protocolVersion" => json_string(ADAPTER_PROTOCOL_VERSION),
        "sequence" => json_u64(frame.sequence.value()),
        "sessionIdentity" => json_string(frame.session_identity.as_str()),
        "correlationIdentity" => json_string(frame.correlation_identity.as_str()),
        "command" => json_string(command_name_wire(frame.command.name())),
        "payload" => payload,
    ));
    if line.len() > MAX_JSONL_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }
    let _ = decode_inbound_jsonl(&line)?;
    Ok(line)
}

fn encode_create_session(payload: &CreateSessionPayload) -> Value {
    json_object!(
        "sessionKind" => json_string(session_kind_wire(payload.session_kind)),
        "cwd" => json_string(payload.cwd.as_str()),
        "agentDirectory" => json_string(payload.agent_directory.as_str()),
        "authPath" => json_string(payload.auth_path.as_str()),
        "modelsPath" => json_string(payload.models_path.as_str()),
        "sessionDirectory" => json_string(payload.session_directory.as_str()),
        "systemPrompt" => json_string(&payload.system_prompt),
        "systemPromptDigest" => json_string(payload.system_prompt_digest.as_str()),
        "model" => encode_model_selection(&payload.model),
        "modelCatalog" => encode_model_catalog(&payload.model_catalog),
        "toolProfile" => json_string(tool_profile_wire(payload.tool_profile)),
        "settings" => encode_settings(&payload.settings),
        "forumContract" => encode_forum_session_contract(&payload.forum_contract),
    )
}
fn encode_forum_session_contract(value: &ForumSessionContractV1) -> Value {
    match value {
        ForumSessionContractV1::ForumEnabledV1 {
            awareness_blake3,
            tool_contract_blake3,
        } => json_object!(
            "kind" => json_string("forum_enabled_v1"),
            "awarenessBlake3" => json_string(awareness_blake3.as_str()),
            "toolContractBlake3" => json_string(tool_contract_blake3.as_str()),
        ),
        ForumSessionContractV1::SequesteredV1 => {
            json_object!("kind" => json_string("sequestered_v1"))
        }
    }
}
fn decode_forum_session_contract(value: &Object) -> Result<ForumSessionContractV1, ProtocolError> {
    match string(value, "kind")? {
        "forum_enabled_v1" => {
            exact_keys(value, &["kind", "awarenessBlake3", "toolContractBlake3"])?;
            let contract = ForumSessionContractV1::ForumEnabledV1 {
                awareness_blake3: Blake3Digest::parse(string(value, "awarenessBlake3")?)?,
                tool_contract_blake3: Blake3Digest::parse(string(value, "toolContractBlake3")?)?,
            };
            contract.assert_pinned()?;
            Ok(contract)
        }
        "sequestered_v1" => {
            exact_keys(value, &["kind"])?;
            Ok(ForumSessionContractV1::SequesteredV1)
        }
        _ => Err(ProtocolError::InvalidFrame("Forum session contract")),
    }
}
fn encode_model_selection(value: &ModelSelection) -> Value {
    json_object!("provider" => json_string(provider_wire(value.provider)), "modelId" => json_string(model_id_wire(value.model_id)), "thinkingLevel" => json_string(thinking_level_wire(value.thinking_level)))
}
fn encode_model_catalog(value: &ModelCatalogPolicyV1) -> Value {
    let model = &value.effective_model;
    json_object!(
        "catalogBlake3" => json_string(value.catalog_blake3.as_str()),
        "effectiveModel" => json_object!(
            "provider" => json_string(provider_wire(model.provider)),
            "baseUrl" => json_string(base_url_wire(model.base_url)),
            "api" => json_string(model_api_wire(model.api)),
            "modelId" => json_string(model_id_wire(model.model_id)),
            "canonicalSlug" => json_string(canonical_slug_wire(model.canonical_slug)),
            "input" => json_string(model_input_wire(model.input)),
            "contextWindow" => json_u64(model.context_window.value()),
            "maxTokens" => json_u64(model.max_tokens.value()),
            "inputUsdPerMillion" => encode_rate(&model.input_usd_per_million),
            "outputUsdPerMillion" => encode_rate(&model.output_usd_per_million),
            "cacheReadUsdPerMillion" => encode_rate(&model.cache_read_usd_per_million),
            "cacheWriteUsdPerMillion" => encode_cache_write_rate(&model.cache_write_usd_per_million),
        ),
    )
}
fn encode_rate(value: &KnownPerMillionRateV1) -> Value {
    json_object!("kind" => json_string("Known"), "usdPerMillion" => json_string(value.usd_per_million.as_str()))
}
fn encode_cache_write_rate(value: &CacheWritePerMillionRateV1) -> Value {
    match value {
        CacheWritePerMillionRateV1::Known(value) => encode_rate(value),
        CacheWritePerMillionRateV1::Absent => json_object!("kind" => json_string("Absent")),
    }
}
fn encode_settings(value: &ActorModelPolicyV1) -> Value {
    json_object!(
        "retry" => json_object!(
            "maxRetries" => json_u64(value.retry.max_retries.value()),
            "baseDelayMilliseconds" => json_u64(value.retry.base_delay_milliseconds.value()),
            "providerTimeoutMilliseconds" => json_u64(value.retry.provider_timeout_milliseconds.value()),
            "providerMaxRetries" => json_u64(value.retry.provider_max_retries.value()),
            "providerMaxRetryDelayMilliseconds" => json_u64(value.retry.provider_max_retry_delay_milliseconds.value()),
        ),
        "compaction" => json_object!(
            "mode" => json_string(compaction_mode_wire(value.compaction.mode)),
            "reserveTokens" => json_u64(value.compaction.reserve_tokens.value()),
            "keepRecentTokens" => json_u64(value.compaction.keep_recent_tokens.value()),
        ),
        "steeringMode" => json_string(queue_mode_wire(value.steering_mode)),
        "followUpMode" => json_string(queue_mode_wire(value.follow_up_mode)),
        "transport" => json_string(transport_wire(value.transport)),
        "projectTrust" => json_string(project_trust_wire(value.project_trust)),
        "installTelemetryEnabled" => json_bool(false),
        "analyticsEnabled" => json_bool(false),
        "images" => json_string(images_wire(value.images)),
    )
}

macro_rules! wire { ($name:ident, $enum:ident, {$($variant:ident => $text:literal),+ $(,)?}) => { fn $name(value: $enum) -> &'static str { match value { $($enum::$variant => $text),+ } } }; }
wire!(session_kind_wire, SessionKind, { TaskAttempt => "TaskAttempt", RootAuthorityOffice => "RootAuthorityOffice" });
wire!(tool_profile_wire, ToolProfile, { ReadExecuteV1 => "read_execute_v1", ReadWriteV1 => "read_write_v1", WorkspaceMutationV1 => "workspace_mutation_v1", WorkspaceIsolatedV1 => "workspace_isolated_v1" });
wire!(queue_mode_wire, QueueMode, { All => "all", OneAtATime => "one-at-a-time" });
wire!(compaction_mode_wire, CompactionMode, { Enabled => "enabled", Disabled => "disabled" });
wire!(prompt_purpose_wire, PromptPurpose, { TaskAssignment => "TaskAssignment", OfficeTurn => "OfficeTurn" });
wire!(steer_reason_wire, SteerReason, { UrgentStalePremise => "UrgentStalePremise", UrgentUnsafePremise => "UrgentUnsafePremise" });
wire!(abort_reason_wire, AbortReason, { GracefulCancellation => "GracefulCancellation", EmergencyStop => "EmergencyStop", BudgetGuardrail => "BudgetGuardrail", DaemonRecovery => "DaemonRecovery" });
wire!(dispose_reason_wire, DisposeReason, { CycleReconciliation => "CycleReconciliation", ProcessRecovery => "ProcessRecovery", ProtocolFailure => "ProtocolFailure" });
wire!(command_name_wire, CommandName, { CreateSession => "CreateSession", Prompt => "Prompt", FollowUp => "FollowUp", Steer => "Steer", Abort => "Abort", GetState => "GetState", Dispose => "Dispose" });
wire!(provider_wire, Provider, { OpenRouter => "openrouter" });
wire!(model_id_wire, ModelId, { DeepseekV4Flash0731 => "deepseek/deepseek-v4-flash-0731" });
wire!(thinking_level_wire, ThinkingLevel, { Off => "off", Minimal => "minimal", Low => "low", Medium => "medium", High => "high", Xhigh => "xhigh", Max => "max" });
wire!(base_url_wire, OpenRouterBaseUrl, { ApiV1 => "https://openrouter.ai/api/v1" });
wire!(model_api_wire, ModelApi, { OpenAiCompletions => "openai-completions" });
wire!(canonical_slug_wire, CanonicalModelSlug, { DeepseekV4Flash20260731 => "deepseek/deepseek-v4-flash-20260731" });
wire!(model_input_wire, ModelInput, { TextOnly => "text_only" });
wire!(transport_wire, Transport, { Sse => "sse" });
wire!(project_trust_wire, ProjectTrust, { Never => "never" });
wire!(images_wire, Images, { Blocked => "blocked" });

pub fn decode_outbound_jsonl(line: &str) -> Result<OutboundFrame, ProtocolError> {
    let root = decode_frame_value(line)?;
    let frame = object(&root)?;
    let event_name = string(frame, "event")?;
    let base_keys = ["protocolVersion", "sequence", "sessionIdentity", "event"];
    literal(frame, "protocolVersion", ADAPTER_PROTOCOL_VERSION)?;
    let sequence = boundary_sequence(frame, "sequence")?;
    let session_identity = SessionIdentity::parse(string(frame, "sessionIdentity")?)?;
    let (correlation_identity, event) = match event_name {
        "AdapterReady" => {
            exact_keys(
                frame,
                &[
                    base_keys[0],
                    base_keys[1],
                    base_keys[2],
                    base_keys[3],
                    "pid",
                    "spawnNonce",
                    "runtime",
                ],
            )?;
            (
                None,
                OutboundEvent::AdapterReady {
                    pid: HostProcessId::parse(positive(frame, "pid")?.value())?,
                    spawn_nonce: SpawnNonce::parse(string(frame, "spawnNonce")?)?,
                    runtime: decode_runtime(object(required(frame, "runtime")?)?)?,
                },
            )
        }
        "SessionReady" => {
            exact_keys(
                frame,
                &[
                    base_keys[0],
                    base_keys[1],
                    base_keys[2],
                    base_keys[3],
                    "correlationIdentity",
                    "configuration",
                ],
            )?;
            (
                Some(CorrelationIdentity::parse(string(
                    frame,
                    "correlationIdentity",
                )?)?),
                OutboundEvent::SessionReady {
                    configuration: decode_effective_configuration(object(required(
                        frame,
                        "configuration",
                    )?)?)?,
                },
            )
        }
        "CommandResult" => {
            let accepted = boolean(frame, "accepted")?;
            if accepted {
                exact_keys(
                    frame,
                    &[
                        base_keys[0],
                        base_keys[1],
                        base_keys[2],
                        base_keys[3],
                        "correlationIdentity",
                        "command",
                        "accepted",
                        "detail",
                    ],
                )?;
                (
                    Some(CorrelationIdentity::parse(string(
                        frame,
                        "correlationIdentity",
                    )?)?),
                    OutboundEvent::CommandResult(CommandResult::Accepted {
                        command: command_name(string(frame, "command")?)?,
                        detail: decode_command_result_detail(object(required(frame, "detail")?)?)?,
                    }),
                )
            } else {
                exact_keys(
                    frame,
                    &[
                        base_keys[0],
                        base_keys[1],
                        base_keys[2],
                        base_keys[3],
                        "correlationIdentity",
                        "command",
                        "accepted",
                        "detail",
                        "failureCode",
                    ],
                )?;
                let detail = object(required(frame, "detail")?)?;
                exact_keys(detail, &["kind"])?;
                literal(detail, "kind", "rejected")?;
                (
                    Some(CorrelationIdentity::parse(string(
                        frame,
                        "correlationIdentity",
                    )?)?),
                    OutboundEvent::CommandResult(CommandResult::Rejected {
                        command: command_name(string(frame, "command")?)?,
                        failure_code: failure_code(string(frame, "failureCode")?)?,
                    }),
                )
            }
        }
        "AgentEvent" => {
            let has_corr = frame.contains_key("correlationIdentity");
            let expected = if has_corr {
                vec![
                    base_keys[0],
                    base_keys[1],
                    base_keys[2],
                    base_keys[3],
                    "correlationIdentity",
                    "agentEvent",
                ]
            } else {
                vec![
                    base_keys[0],
                    base_keys[1],
                    base_keys[2],
                    base_keys[3],
                    "agentEvent",
                ]
            };
            exact_keys(frame, &expected)?;
            (
                optional_correlation(frame)?,
                OutboundEvent::AgentEvent {
                    agent_event: decode_agent_event(object(required(frame, "agentEvent")?)?)?,
                },
            )
        }
        "UsageSnapshot" => {
            let has_corr = frame.contains_key("correlationIdentity");
            let expected = if has_corr {
                vec![
                    base_keys[0],
                    base_keys[1],
                    base_keys[2],
                    base_keys[3],
                    "correlationIdentity",
                    "usage",
                ]
            } else {
                vec![
                    base_keys[0],
                    base_keys[1],
                    base_keys[2],
                    base_keys[3],
                    "usage",
                ]
            };
            exact_keys(frame, &expected)?;
            (
                optional_correlation(frame)?,
                OutboundEvent::UsageSnapshot {
                    usage: decode_usage(object(required(frame, "usage")?)?)?,
                },
            )
        }
        "Settled" => {
            exact_keys(
                frame,
                &[
                    base_keys[0],
                    base_keys[1],
                    base_keys[2],
                    base_keys[3],
                    "correlationIdentity",
                    "classification",
                    "finalAssistantOutcome",
                ],
            )?;
            (
                Some(CorrelationIdentity::parse(string(
                    frame,
                    "correlationIdentity",
                )?)?),
                OutboundEvent::Settled {
                    classification: settled_classification(string(frame, "classification")?)?,
                    final_assistant_outcome: decode_final_outcome(object(required(
                        frame,
                        "finalAssistantOutcome",
                    )?)?)?,
                },
            )
        }
        "Disposed" => {
            exact_keys(
                frame,
                &[
                    base_keys[0],
                    base_keys[1],
                    base_keys[2],
                    base_keys[3],
                    "correlationIdentity",
                    "transcriptFlushReceipt",
                ],
            )?;
            (
                Some(CorrelationIdentity::parse(string(
                    frame,
                    "correlationIdentity",
                )?)?),
                OutboundEvent::Disposed {
                    transcript_flush_receipt: decode_transcript_receipt(object(required(
                        frame,
                        "transcriptFlushReceipt",
                    )?)?)?,
                },
            )
        }
        "Fatal" => {
            exact_keys(
                frame,
                &[
                    base_keys[0],
                    base_keys[1],
                    base_keys[2],
                    base_keys[3],
                    "failureCode",
                ],
            )?;
            (
                None,
                OutboundEvent::Fatal {
                    failure_code: failure_code(string(frame, "failureCode")?)?,
                },
            )
        }
        _ => return Err(ProtocolError::InvalidFrame("outbound event")),
    };
    Ok(OutboundFrame {
        sequence,
        session_identity,
        correlation_identity,
        event,
    })
}

fn decode_frame_value(line: &str) -> Result<Value, ProtocolError> {
    if line.len() > MAX_JSONL_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }
    reject_duplicate_object_keys(line)?;
    miniserde::json::from_str(line).map_err(|_| ProtocolError::InvalidJson)
}

fn decode_create_session(value: &Object) -> Result<CreateSessionPayload, ProtocolError> {
    exact_keys(
        value,
        &[
            "sessionKind",
            "cwd",
            "agentDirectory",
            "authPath",
            "modelsPath",
            "sessionDirectory",
            "systemPrompt",
            "systemPromptDigest",
            "model",
            "modelCatalog",
            "toolProfile",
            "settings",
            "forumContract",
        ],
    )?;
    let model = object(required(value, "model")?)?;
    exact_keys(model, &["provider", "modelId", "thinkingLevel"])?;
    literal(model, "provider", PINNED_PROVIDER)?;
    literal(model, "modelId", PINNED_MODEL)?;
    literal(model, "thinkingLevel", PINNED_THINKING_LEVEL)?;
    let model_selection = decode_model_selection(model)?;
    if model_selection.provider != Provider::OpenRouter
        || model_selection.model_id != ModelId::DeepseekV4Flash0731
        || model_selection.thinking_level != ThinkingLevel::High
    {
        return Err(ProtocolError::InvalidFrame("pinned model selection"));
    }
    let decoded = CreateSessionPayload {
        session_kind: session_kind(string(value, "sessionKind")?)?,
        cwd: path(value, "cwd")?,
        agent_directory: path(value, "agentDirectory")?,
        auth_path: path(value, "authPath")?,
        models_path: path(value, "modelsPath")?,
        session_directory: path(value, "sessionDirectory")?,
        system_prompt: nonempty(value, "systemPrompt")?,
        system_prompt_digest: Blake3Digest::parse(string(value, "systemPromptDigest")?)?,
        model: model_selection,
        model_catalog: decode_model_catalog(object(required(value, "modelCatalog")?)?)?,
        tool_profile: tool_profile(string(value, "toolProfile")?)?,
        settings: decode_settings(object(required(value, "settings")?)?)?,
        forum_contract: decode_forum_session_contract(object(required(value, "forumContract")?)?)?,
    };
    if decoded.tool_profile == ToolProfile::WorkspaceIsolatedV1
        && !matches!(&decoded.forum_contract, ForumSessionContractV1::SequesteredV1)
    {
        return Err(ProtocolError::InvalidFrame(
            "workspace-isolated Forum pairing",
        ));
    }
    decoded.model_catalog.assert_pinned()?;
    decoded.settings.assert_pinned()?;
    decoded.forum_contract.assert_pinned()?;
    Ok(decoded)
}
fn decode_prompt(value: &Object) -> Result<PromptPayload, ProtocolError> {
    exact_keys(value, &["purpose", "text"])?;
    Ok(PromptPayload {
        purpose: prompt_purpose(string(value, "purpose")?)?,
        text: nonempty(value, "text")?,
    })
}
fn decode_follow_up(value: &Object) -> Result<FollowUpPayload, ProtocolError> {
    exact_keys(value, &["noticeDeliveryIdentity", "ledgerFrontier", "text"])?;
    Ok(FollowUpPayload {
        notice_delivery_identity: CorrelationIdentity::parse(string(
            value,
            "noticeDeliveryIdentity",
        )?)?,
        ledger_frontier: LedgerFrontier::parse(unsigned(value, "ledgerFrontier")?)?,
        text: nonempty(value, "text")?,
    })
}
fn decode_steer(value: &Object) -> Result<SteerPayload, ProtocolError> {
    exact_keys(value, &["reason", "text"])?;
    Ok(SteerPayload {
        reason: steer_reason(string(value, "reason")?)?,
        text: nonempty(value, "text")?,
    })
}
fn decode_abort(value: &Object) -> Result<AbortPayload, ProtocolError> {
    exact_keys(value, &["reason"])?;
    Ok(AbortPayload {
        reason: abort_reason(string(value, "reason")?)?,
    })
}
fn decode_dispose(value: &Object) -> Result<DisposePayload, ProtocolError> {
    exact_keys(value, &["reason"])?;
    Ok(DisposePayload {
        reason: dispose_reason(string(value, "reason")?)?,
    })
}

fn decode_model_catalog(value: &Object) -> Result<ModelCatalogPolicyV1, ProtocolError> {
    exact_keys(value, &["catalogBlake3", "effectiveModel"])?;
    let model = object(required(value, "effectiveModel")?)?;
    exact_keys(
        model,
        &[
            "provider",
            "baseUrl",
            "api",
            "modelId",
            "canonicalSlug",
            "input",
            "contextWindow",
            "maxTokens",
            "inputUsdPerMillion",
            "outputUsdPerMillion",
            "cacheReadUsdPerMillion",
            "cacheWriteUsdPerMillion",
        ],
    )?;
    Ok(ModelCatalogPolicyV1 {
        catalog_blake3: Blake3Digest::parse(string(value, "catalogBlake3")?)?,
        effective_model: EffectiveModelDescriptorV1 {
            provider: provider(string(model, "provider")?)?,
            base_url: open_router_base_url(string(model, "baseUrl")?)?,
            api: model_api(string(model, "api")?)?,
            model_id: model_id(string(model, "modelId")?)?,
            canonical_slug: canonical_model_slug(string(model, "canonicalSlug")?)?,
            input: model_input(string(model, "input")?)?,
            context_window: positive(model, "contextWindow")?,
            max_tokens: positive(model, "maxTokens")?,
            input_usd_per_million: decode_known_rate(object(required(
                model,
                "inputUsdPerMillion",
            )?)?)?,
            output_usd_per_million: decode_known_rate(object(required(
                model,
                "outputUsdPerMillion",
            )?)?)?,
            cache_read_usd_per_million: decode_known_rate(object(required(
                model,
                "cacheReadUsdPerMillion",
            )?)?)?,
            cache_write_usd_per_million: decode_cache_write_rate(object(required(
                model,
                "cacheWriteUsdPerMillion",
            )?)?)?,
        },
    })
}
fn decode_known_rate(value: &Object) -> Result<KnownPerMillionRateV1, ProtocolError> {
    exact_keys(value, &["kind", "usdPerMillion"])?;
    literal(value, "kind", "Known")?;
    Ok(KnownPerMillionRateV1 {
        usd_per_million: UsdPerMillionDecimal::parse(string(value, "usdPerMillion")?)?,
    })
}
fn decode_cache_write_rate(value: &Object) -> Result<CacheWritePerMillionRateV1, ProtocolError> {
    match string(value, "kind")? {
        "Absent" => {
            exact_keys(value, &["kind"])?;
            Ok(CacheWritePerMillionRateV1::Absent)
        }
        "Known" => Ok(CacheWritePerMillionRateV1::Known(decode_known_rate(value)?)),
        _ => Err(ProtocolError::InvalidFrame("cache write rate")),
    }
}
fn decode_settings(value: &Object) -> Result<ActorModelPolicyV1, ProtocolError> {
    exact_keys(
        value,
        &[
            "retry",
            "compaction",
            "steeringMode",
            "followUpMode",
            "transport",
            "projectTrust",
            "installTelemetryEnabled",
            "analyticsEnabled",
            "images",
        ],
    )?;
    let retry = object(required(value, "retry")?)?;
    exact_keys(
        retry,
        &[
            "maxRetries",
            "baseDelayMilliseconds",
            "providerTimeoutMilliseconds",
            "providerMaxRetries",
            "providerMaxRetryDelayMilliseconds",
        ],
    )?;
    let compaction = object(required(value, "compaction")?)?;
    exact_keys(compaction, &["mode", "reserveTokens", "keepRecentTokens"])?;
    Ok(ActorModelPolicyV1 {
        retry: RetryPolicyV1 {
            max_retries: nonnegative(retry, "maxRetries")?,
            base_delay_milliseconds: nonnegative(retry, "baseDelayMilliseconds")?,
            provider_timeout_milliseconds: positive(retry, "providerTimeoutMilliseconds")?,
            provider_max_retries: nonnegative(retry, "providerMaxRetries")?,
            provider_max_retry_delay_milliseconds: positive(
                retry,
                "providerMaxRetryDelayMilliseconds",
            )?,
        },
        compaction: CompactionPolicyV1 {
            mode: compaction_mode(string(compaction, "mode")?)?,
            reserve_tokens: nonnegative(compaction, "reserveTokens")?,
            keep_recent_tokens: nonnegative(compaction, "keepRecentTokens")?,
        },
        steering_mode: queue_mode(string(value, "steeringMode")?)?,
        follow_up_mode: queue_mode(string(value, "followUpMode")?)?,
        transport: transport(string(value, "transport")?)?,
        project_trust: project_trust(string(value, "projectTrust")?)?,
        install_telemetry: disabled(boolean(value, "installTelemetryEnabled")?)?,
        analytics: disabled(boolean(value, "analyticsEnabled")?)?,
        images: images(string(value, "images")?)?,
    })
}

fn decode_runtime(value: &Object) -> Result<RuntimeIdentity, ProtocolError> {
    exact_keys(
        value,
        &[
            "nodeVersion",
            "adapterVersion",
            "piSdkVersion",
            "nodeExecutableBlake3",
            "lockfileBlake3",
            "adapterBuildBlake3",
            "piTransitivePackageSetBlake3",
        ],
    )?;
    let runtime = RuntimeIdentity {
        node_version: NodeRuntimeVersion::parse(string(value, "nodeVersion")?)?,
        adapter_version: adapter_version(string(value, "adapterVersion")?)?,
        pi_sdk_version: pi_sdk_version(string(value, "piSdkVersion")?)?,
        node_executable_blake3: Blake3Digest::parse(string(value, "nodeExecutableBlake3")?)?,
        lockfile_blake3: Blake3Digest::parse(string(value, "lockfileBlake3")?)?,
        adapter_build_blake3: Blake3Digest::parse(string(value, "adapterBuildBlake3")?)?,
        pi_transitive_package_set_blake3: Blake3Digest::parse(string(
            value,
            "piTransitivePackageSetBlake3",
        )?)?,
    };
    runtime.assert_v1()?;
    Ok(runtime)
}
fn decode_effective_configuration(
    value: &Object,
) -> Result<EffectiveSessionConfiguration, ProtocolError> {
    exact_keys(
        value,
        &[
            "sessionKind",
            "cwd",
            "sessionDirectory",
            "sessionFile",
            "model",
            "modelCatalog",
            "toolProfile",
            "tools",
            "settings",
            "forumContract",
        ],
    )?;
    let model = object(required(value, "model")?)?;
    exact_keys(model, &["provider", "modelId", "thinkingLevel"])?;
    let profile = tool_profile(string(value, "toolProfile")?)?;
    let tools = array(required(value, "tools")?)?
        .iter()
        .map(|item| pi_tool_name(value_string(item)?))
        .collect::<Result<Vec<_>, _>>()?;
    let config = EffectiveSessionConfiguration {
        session_kind: session_kind(string(value, "sessionKind")?)?,
        cwd: path(value, "cwd")?,
        session_directory: path(value, "sessionDirectory")?,
        session_file: path(value, "sessionFile")?,
        model: decode_model_selection(model)?,
        model_catalog: decode_model_catalog(object(required(value, "modelCatalog")?)?)?,
        tool_profile: profile,
        tools,
        settings: decode_settings(object(required(value, "settings")?)?)?,
        forum_contract: decode_forum_session_contract(object(required(value, "forumContract")?)?)?,
    };
    config.assert_pinned()?;
    Ok(config)
}
fn decode_command_result_detail(value: &Object) -> Result<CommandResultDetail, ProtocolError> {
    match string(value, "kind")? {
        "acknowledged" => {
            exact_keys(value, &["kind"])?;
            Ok(CommandResultDetail::Acknowledged)
        }
        "state" => {
            let has = value.contains_key("sessionFile");
            if has {
                exact_keys(value, &["kind", "phase", "liveness", "sessionFile"])?
            } else {
                exact_keys(value, &["kind", "phase", "liveness"])?
            };
            Ok(CommandResultDetail::State {
                phase: adapter_phase(string(value, "phase")?)?,
                liveness: session_liveness(string(value, "liveness")?)?,
                session_file: if has {
                    Some(path(value, "sessionFile")?)
                } else {
                    None
                },
            })
        }
        _ => Err(ProtocolError::InvalidFrame("CommandResult detail")),
    }
}
fn decode_usage(value: &Object) -> Result<UsageObservation, ProtocolError> {
    match string(value, "kind")? {
        "Known" => {
            exact_keys(value, &["kind", "totals"])?;
            let totals = object(required(value, "totals")?)?;
            exact_keys(
                totals,
                &[
                    "inputTokens",
                    "outputTokens",
                    "cacheReadTokens",
                    "cacheWriteTokens",
                    "totalTokens",
                    "providerCost",
                ],
            )?;
            let cost = object(required(totals, "providerCost")?)?;
            exact_keys(cost, &["encoding", "binary64BigEndianHex", "rounding"])?;
            literal(cost, "encoding", "ieee754_binary64_be_hex_v1")?;
            literal(cost, "rounding", "ceil_to_micro_usd")?;
            Ok(UsageObservation::Known(UsageTotals {
                input_tokens: nonnegative(totals, "inputTokens")?,
                output_tokens: nonnegative(totals, "outputTokens")?,
                cache_read_tokens: nonnegative(totals, "cacheReadTokens")?,
                cache_write_tokens: nonnegative(totals, "cacheWriteTokens")?,
                total_tokens: nonnegative(totals, "totalTokens")?,
                provider_cost: ProviderCostObservationV1 {
                    binary64_big_endian_hex: Binary64BigEndianHex::parse(string(
                        cost,
                        "binary64BigEndianHex",
                    )?)?,
                },
            }))
        }
        "Unavailable" => {
            exact_keys(value, &["kind", "reason"])?;
            Ok(UsageObservation::Unavailable(usage_unavailable_reason(
                string(value, "reason")?,
            )?))
        }
        _ => Err(ProtocolError::InvalidFrame("usage observation")),
    }
}
fn decode_final_outcome(value: &Object) -> Result<FinalAssistantOutcome, ProtocolError> {
    match string(value, "kind")? {
        "Observed" => {
            exact_keys(value, &["kind", "stopReason"])?;
            Ok(FinalAssistantOutcome::Observed {
                stop_reason: assistant_stop_reason(string(value, "stopReason")?)?,
            })
        }
        "Unavailable" => {
            exact_keys(value, &["kind", "reason"])?;
            Ok(FinalAssistantOutcome::Unavailable {
                reason: final_unavailable_reason(string(value, "reason")?)?,
            })
        }
        _ => Err(ProtocolError::InvalidFrame("final assistant outcome")),
    }
}
fn decode_transcript_receipt(value: &Object) -> Result<TranscriptFlushReceiptV1, ProtocolError> {
    literal(value, "format", "pi_session_manager_jsonl_v3")?;
    let materialization = string(value, "materialization")?;
    match materialization {
        "observed" => {
            exact_keys(
                value,
                &[
                    "format",
                    "sessionIdentity",
                    "sessionFile",
                    "materialization",
                    "sessionFileBlake3",
                    "headerCwd",
                    "firstUserPrompt",
                ],
            )?;
            Ok(TranscriptFlushReceiptV1::Materialized {
                session_identity: SessionIdentity::parse(string(value, "sessionIdentity")?)?,
                session_file: path(value, "sessionFile")?,
                session_file_blake3: Blake3Digest::parse(string(value, "sessionFileBlake3")?)?,
                header_cwd: path(value, "headerCwd")?,
                first_user_prompt: decode_first_user_prompt(object(required(
                    value,
                    "firstUserPrompt",
                )?)?)?,
            })
        }
        "unmaterialized_no_prompt" => {
            exact_keys(
                value,
                &[
                    "format",
                    "sessionIdentity",
                    "sessionFile",
                    "materialization",
                    "firstUserPrompt",
                ],
            )?;
            let prompt = object(required(value, "firstUserPrompt")?)?;
            exact_keys(prompt, &["kind"])?;
            literal(prompt, "kind", "absent")?;
            Ok(TranscriptFlushReceiptV1::UnmaterializedNoPrompt {
                session_identity: SessionIdentity::parse(string(value, "sessionIdentity")?)?,
                session_file: path(value, "sessionFile")?,
            })
        }
        _ => Err(ProtocolError::InvalidFrame("transcript materialization")),
    }
}
fn decode_first_user_prompt(value: &Object) -> Result<FirstUserPromptReceipt, ProtocolError> {
    match string(value, "kind")? {
        "absent" => {
            exact_keys(value, &["kind"])?;
            Ok(FirstUserPromptReceipt::Absent)
        }
        "verified" => {
            exact_keys(value, &["kind", "digest"])?;
            Ok(FirstUserPromptReceipt::Verified {
                digest: Blake3Digest::parse(string(value, "digest")?)?,
            })
        }
        _ => Err(ProtocolError::InvalidFrame("first user prompt receipt")),
    }
}

fn decode_agent_event(value: &Object) -> Result<ProjectedAgentEvent, ProtocolError> {
    let kind = string(value, "type")?;
    macro_rules! keys { ($($key:expr),* $(,)?) => { exact_keys(value, &["type", $($key),*])? }; }
    match kind {
        "agent_start" => {
            keys!();
            Ok(ProjectedAgentEvent::AgentStart)
        }
        "agent_end" => {
            keys!("messages", "willRetry");
            Ok(ProjectedAgentEvent::AgentEnd {
                messages: json_array(value, "messages")?,
                will_retry: boolean(value, "willRetry")?,
            })
        }
        "agent_settled" => {
            keys!();
            Ok(ProjectedAgentEvent::AgentSettled)
        }
        "turn_start" => {
            keys!();
            Ok(ProjectedAgentEvent::TurnStart)
        }
        "turn_end" => {
            keys!("message", "toolResults");
            Ok(ProjectedAgentEvent::TurnEnd {
                message: required(value, "message")?.clone(),
                tool_results: json_array(value, "toolResults")?,
            })
        }
        "message_start" => {
            keys!("message");
            Ok(ProjectedAgentEvent::MessageStart {
                message: required(value, "message")?.clone(),
            })
        }
        "message_update" => {
            keys!("message", "assistantMessageEvent");
            Ok(ProjectedAgentEvent::MessageUpdate {
                message: required(value, "message")?.clone(),
                assistant_message_event: required(value, "assistantMessageEvent")?.clone(),
            })
        }
        "message_end" => {
            keys!("message");
            Ok(ProjectedAgentEvent::MessageEnd {
                message: required(value, "message")?.clone(),
            })
        }
        "tool_execution_start" => {
            keys!("toolCallIdentity", "toolName", "args");
            Ok(ProjectedAgentEvent::ToolExecutionStart {
                tool_call_identity: ToolCallIdentity::parse(string(value, "toolCallIdentity")?)?,
                tool_name: pi_tool_name(string(value, "toolName")?)?,
                args: required(value, "args")?.clone(),
            })
        }
        "tool_execution_update" => {
            keys!("toolCallIdentity", "toolName", "args", "partialResult");
            Ok(ProjectedAgentEvent::ToolExecutionUpdate {
                tool_call_identity: ToolCallIdentity::parse(string(value, "toolCallIdentity")?)?,
                tool_name: pi_tool_name(string(value, "toolName")?)?,
                args: required(value, "args")?.clone(),
                partial_result: required(value, "partialResult")?.clone(),
            })
        }
        "tool_execution_end" => {
            keys!("toolCallIdentity", "toolName", "result", "isError");
            Ok(ProjectedAgentEvent::ToolExecutionEnd {
                tool_call_identity: ToolCallIdentity::parse(string(value, "toolCallIdentity")?)?,
                tool_name: pi_tool_name(string(value, "toolName")?)?,
                result: required(value, "result")?.clone(),
                is_error: boolean(value, "isError")?,
            })
        }
        "queue_update" => {
            keys!("steering", "followUp");
            Ok(ProjectedAgentEvent::QueueUpdate {
                steering: string_array(value, "steering")?,
                follow_up: string_array(value, "followUp")?,
            })
        }
        "entry_appended" => {
            keys!("entry");
            Ok(ProjectedAgentEvent::EntryAppended {
                entry: required(value, "entry")?.clone(),
            })
        }
        "bash_execution_update" => {
            let has_identity = value.contains_key("executionIdentity");
            if has_identity {
                keys!("executionIdentity", "delta")
            } else {
                keys!("delta")
            };
            Ok(ProjectedAgentEvent::BashExecutionUpdate {
                execution_identity: if has_identity {
                    Some(BashExecutionIdentity::parse(string(
                        value,
                        "executionIdentity",
                    )?)?)
                } else {
                    None
                },
                delta: string(value, "delta")?.into(),
            })
        }
        "compaction_start" => {
            keys!("reason");
            Ok(ProjectedAgentEvent::CompactionStart {
                reason: compaction_reason(string(value, "reason")?)?,
            })
        }
        "session_info_changed" => {
            let has_name = value.contains_key("name");
            if has_name {
                keys!("name")
            } else {
                keys!()
            };
            Ok(ProjectedAgentEvent::SessionInfoChanged {
                name: if has_name {
                    Some(string(value, "name")?.into())
                } else {
                    None
                },
            })
        }
        "thinking_level_changed" => {
            keys!("level");
            Ok(ProjectedAgentEvent::ThinkingLevelChanged {
                level: thinking_level(string(value, "level")?)?,
            })
        }
        "compaction_end" => {
            let result = value.contains_key("result");
            let error = value.contains_key("errorMessage");
            match (result, error) {
                (false, false) => keys!("reason", "aborted", "willRetry"),
                (true, false) => keys!("reason", "result", "aborted", "willRetry"),
                (false, true) => keys!("reason", "aborted", "willRetry", "errorMessage"),
                (true, true) => keys!("reason", "result", "aborted", "willRetry", "errorMessage"),
            };
            Ok(ProjectedAgentEvent::CompactionEnd {
                reason: compaction_reason(string(value, "reason")?)?,
                result: result.then(|| required(value, "result").expect("key checked").clone()),
                aborted: boolean(value, "aborted")?,
                will_retry: boolean(value, "willRetry")?,
                error_message: if error {
                    Some(string(value, "errorMessage")?.into())
                } else {
                    None
                },
            })
        }
        "auto_retry_start" => {
            keys!(
                "attempt",
                "maxAttempts",
                "delayMilliseconds",
                "errorMessage"
            );
            Ok(ProjectedAgentEvent::AutoRetryStart {
                attempt: nonnegative(value, "attempt")?,
                max_attempts: nonnegative(value, "maxAttempts")?,
                delay_milliseconds: nonnegative(value, "delayMilliseconds")?,
                error_message: string(value, "errorMessage")?.into(),
            })
        }
        "auto_retry_end" => {
            let error = value.contains_key("finalError");
            if error {
                keys!("success", "attempt", "finalError")
            } else {
                keys!("success", "attempt")
            };
            Ok(ProjectedAgentEvent::AutoRetryEnd {
                success: boolean(value, "success")?,
                attempt: nonnegative(value, "attempt")?,
                final_error: if error {
                    Some(string(value, "finalError")?.into())
                } else {
                    None
                },
            })
        }
        "summarization_retry_scheduled" => {
            keys!(
                "attempt",
                "maxAttempts",
                "delayMilliseconds",
                "errorMessage"
            );
            Ok(ProjectedAgentEvent::SummarizationRetryScheduled {
                attempt: nonnegative(value, "attempt")?,
                max_attempts: nonnegative(value, "maxAttempts")?,
                delay_milliseconds: nonnegative(value, "delayMilliseconds")?,
                error_message: string(value, "errorMessage")?.into(),
            })
        }
        "summarization_retry_attempt_start" => {
            let source = summarization_source(string(value, "source")?)?;
            match source {
                SummarizationSource::BranchSummary => {
                    keys!("source");
                    Ok(ProjectedAgentEvent::SummarizationRetryAttemptStart {
                        source,
                        reason: None,
                    })
                }
                SummarizationSource::Compaction => {
                    keys!("source", "reason");
                    Ok(ProjectedAgentEvent::SummarizationRetryAttemptStart {
                        source,
                        reason: Some(compaction_reason(string(value, "reason")?)?),
                    })
                }
            }
        }
        "summarization_retry_finished" => {
            keys!();
            Ok(ProjectedAgentEvent::SummarizationRetryFinished)
        }
        _ => Err(ProtocolError::InvalidFrame("AgentSession event variant")),
    }
}

fn object(value: &Value) -> Result<&Object, ProtocolError> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(ProtocolError::InvalidFrame("object")),
    }
}
fn array(value: &Value) -> Result<&Array, ProtocolError> {
    match value {
        Value::Array(array) => Ok(array),
        _ => Err(ProtocolError::InvalidFrame("array")),
    }
}
fn required<'a>(object: &'a Object, key: &str) -> Result<&'a Value, ProtocolError> {
    object
        .get(key)
        .ok_or(ProtocolError::InvalidFrame("missing field"))
}
fn string<'a>(object: &'a Object, key: &str) -> Result<&'a str, ProtocolError> {
    match required(object, key)? {
        Value::String(value) => Ok(value),
        _ => Err(ProtocolError::InvalidFrame("string field")),
    }
}
fn value_string(value: &Value) -> Result<&str, ProtocolError> {
    match value {
        Value::String(value) => Ok(value),
        _ => Err(ProtocolError::InvalidFrame("string array member")),
    }
}
fn nonempty(object: &Object, key: &str) -> Result<String, ProtocolError> {
    let value = string(object, key)?;
    if value.is_empty() {
        Err(ProtocolError::InvalidFrame("nonempty string"))
    } else {
        Ok(value.into())
    }
}
fn boolean(object: &Object, key: &str) -> Result<bool, ProtocolError> {
    match required(object, key)? {
        Value::Bool(value) => Ok(*value),
        _ => Err(ProtocolError::InvalidFrame("boolean field")),
    }
}
fn unsigned(object: &Object, key: &str) -> Result<u64, ProtocolError> {
    match required(object, key)? {
        Value::Number(Number::U64(value)) if *value <= MAX_SAFE_INTEGER => Ok(*value),
        _ => Err(ProtocolError::InvalidFrame("safe unsigned integer")),
    }
}
fn boundary_sequence(object: &Object, key: &str) -> Result<BoundarySequence, ProtocolError> {
    BoundarySequence::parse(unsigned(object, key)?)
}
fn nonnegative(object: &Object, key: &str) -> Result<NonNegativeInteger, ProtocolError> {
    NonNegativeInteger::parse(unsigned(object, key)?)
}
fn positive(object: &Object, key: &str) -> Result<PositiveInteger, ProtocolError> {
    PositiveInteger::parse(unsigned(object, key)?)
}
fn path(object: &Object, key: &str) -> Result<AbsolutePath, ProtocolError> {
    AbsolutePath::parse(string(object, key)?)
}
fn optional_correlation(object: &Object) -> Result<Option<CorrelationIdentity>, ProtocolError> {
    object
        .get("correlationIdentity")
        .map(|value| value_string(value).and_then(CorrelationIdentity::parse))
        .transpose()
}
fn json_array(object: &Object, key: &str) -> Result<Vec<Value>, ProtocolError> {
    Ok(array(required(object, key)?)?.iter().cloned().collect())
}
fn string_array(object: &Object, key: &str) -> Result<Vec<String>, ProtocolError> {
    array(required(object, key)?)?
        .iter()
        .map(|value| value_string(value).map(Into::into))
        .collect()
}
fn exact_keys(object: &Object, expected: &[&str]) -> Result<(), ProtocolError> {
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        Err(ProtocolError::InvalidFrame("exact object keys"))
    } else {
        Ok(())
    }
}
fn literal(object: &Object, key: &str, expected: &str) -> Result<(), ProtocolError> {
    if string(object, key)? == expected {
        Ok(())
    } else {
        Err(ProtocolError::InvalidFrame("literal field"))
    }
}

macro_rules! closed_enum { ($function:ident, $enum:ident, {$($text:literal => $variant:ident),+ $(,)?}) => { fn $function(value:&str)->Result<$enum,ProtocolError>{match value{$($text=>Ok($enum::$variant),)+_=>Err(ProtocolError::InvalidFrame(stringify!($enum)))}} }; }
closed_enum!(session_kind,SessionKind,{"TaskAttempt"=>TaskAttempt,"RootAuthorityOffice"=>RootAuthorityOffice});
closed_enum!(tool_profile,ToolProfile,{"read_execute_v1"=>ReadExecuteV1,"read_write_v1"=>ReadWriteV1,"workspace_mutation_v1"=>WorkspaceMutationV1,"workspace_isolated_v1"=>WorkspaceIsolatedV1});
closed_enum!(pi_tool_name,PiToolName,{"read"=>Read,"bash"=>Bash,"edit"=>Edit,"write"=>Write,"grep"=>Grep,"find"=>Find,"ls"=>Ls});
closed_enum!(queue_mode,QueueMode,{"all"=>All,"one-at-a-time"=>OneAtATime});
closed_enum!(compaction_mode,CompactionMode,{"enabled"=>Enabled,"disabled"=>Disabled});
closed_enum!(prompt_purpose,PromptPurpose,{"TaskAssignment"=>TaskAssignment,"OfficeTurn"=>OfficeTurn});
closed_enum!(steer_reason,SteerReason,{"UrgentStalePremise"=>UrgentStalePremise,"UrgentUnsafePremise"=>UrgentUnsafePremise});
closed_enum!(abort_reason,AbortReason,{"GracefulCancellation"=>GracefulCancellation,"EmergencyStop"=>EmergencyStop,"BudgetGuardrail"=>BudgetGuardrail,"DaemonRecovery"=>DaemonRecovery});
closed_enum!(dispose_reason,DisposeReason,{"CycleReconciliation"=>CycleReconciliation,"ProcessRecovery"=>ProcessRecovery,"ProtocolFailure"=>ProtocolFailure});
closed_enum!(provider,Provider,{"openrouter"=>OpenRouter});
closed_enum!(model_id,ModelId,{"deepseek/deepseek-v4-flash-0731"=>DeepseekV4Flash0731});
closed_enum!(canonical_model_slug,CanonicalModelSlug,{"deepseek/deepseek-v4-flash-20260731"=>DeepseekV4Flash20260731});
closed_enum!(open_router_base_url,OpenRouterBaseUrl,{"https://openrouter.ai/api/v1"=>ApiV1});
closed_enum!(model_api,ModelApi,{"openai-completions"=>OpenAiCompletions});
closed_enum!(model_input,ModelInput,{"text_only"=>TextOnly});
closed_enum!(transport,Transport,{"sse"=>Sse});
closed_enum!(project_trust,ProjectTrust,{"never"=>Never});
closed_enum!(images,Images,{"blocked"=>Blocked});
closed_enum!(adapter_version,AdapterVersion,{"1"=>V1});
closed_enum!(pi_sdk_version,PiSdkVersion,{"0.83.0"=>V0830});
closed_enum!(command_name,CommandName,{"CreateSession"=>CreateSession,"Prompt"=>Prompt,"FollowUp"=>FollowUp,"Steer"=>Steer,"Abort"=>Abort,"GetState"=>GetState,"Dispose"=>Dispose});
closed_enum!(adapter_phase,AdapterPhase,{"Inert"=>Inert,"Creating"=>Creating,"Ready"=>Ready,"Closing"=>Closing,"Disposed"=>Disposed,"Fatal"=>Fatal});
closed_enum!(session_liveness,SessionLiveness,{"inert"=>Inert,"creating"=>Creating,"idle"=>Idle,"active"=>Active,"closing"=>Closing,"disposed"=>Disposed,"fatal"=>Fatal});
closed_enum!(failure_code,AdapterFailureCode,{"invalid_command"=>InvalidCommand,"invalid_state"=>InvalidState,"sequence_gap"=>SequenceGap,"session_identity_mismatch"=>SessionIdentityMismatch,"execution_profile_drift"=>ExecutionProfileDrift,"sdk_operation_failed"=>SdkOperationFailed,"missing_agent_settled"=>MissingAgentSettled,"missing_final_assistant_outcome"=>MissingFinalAssistantOutcome,"protocol_decode_failed"=>ProtocolDecodeFailed,"outbound_frame_too_large"=>OutboundFrameTooLarge});
closed_enum!(usage_unavailable_reason,UsageUnavailableReason,{"invalid_sdk_usage"=>InvalidSdkUsage,"usage_regressed"=>UsageRegressed,"usage_inconsistent"=>UsageInconsistent});
closed_enum!(compaction_reason,CompactionReason,{"manual"=>Manual,"threshold"=>Threshold,"overflow"=>Overflow});
closed_enum!(thinking_level,ThinkingLevel,{"off"=>Off,"minimal"=>Minimal,"low"=>Low,"medium"=>Medium,"high"=>High,"xhigh"=>Xhigh,"max"=>Max});
closed_enum!(summarization_source,SummarizationSource,{"branchSummary"=>BranchSummary,"compaction"=>Compaction});
closed_enum!(settled_classification,SettledClassification,{"completed"=>Completed,"length"=>Length,"error"=>Error,"aborted"=>Aborted,"failed"=>Failed,"protocol_failed"=>ProtocolFailed});
closed_enum!(assistant_stop_reason,AssistantStopReason,{"stop"=>Stop,"length"=>Length,"error"=>Error,"aborted"=>Aborted});
closed_enum!(final_unavailable_reason,FinalAssistantUnavailableReason,{"sdk_promise_rejected"=>SdkPromiseRejected,"missing_final_assistant_outcome"=>MissingFinalAssistantOutcome});

fn disabled(value: bool) -> Result<Disabled, ProtocolError> {
    if value {
        Err(ProtocolError::InvalidFrame("disabled setting"))
    } else {
        Ok(Disabled::Disabled)
    }
}

fn decode_model_selection(value: &Object) -> Result<ModelSelection, ProtocolError> {
    Ok(ModelSelection {
        provider: provider(string(value, "provider")?)?,
        model_id: model_id(string(value, "modelId")?)?,
        thinking_level: thinking_level(string(value, "thinkingLevel")?)?,
    })
}

fn is_identifier(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || !identifier_edge(bytes[0])
        || !identifier_edge(*bytes.last().expect("checked"))
    {
        return false;
    }
    bytes
        .iter()
        .all(|byte| identifier_edge(*byte) || matches!(*byte, b'.' | b'_' | b'-'))
}
fn identifier_edge(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
}

/// Detect duplicate decoded object keys before miniserde's object insertion.
fn reject_duplicate_object_keys(input: &str) -> Result<(), ProtocolError> {
    struct Scanner<'a> {
        bytes: &'a [u8],
        at: usize,
    }
    impl Scanner<'_> {
        fn whitespace(&mut self) {
            while matches!(self.bytes.get(self.at), Some(b' ' | b'\n' | b'\r' | b'\t')) {
                self.at += 1;
            }
        }
        fn string(&mut self) -> Result<String, ProtocolError> {
            let start = self.at;
            if self.bytes.get(self.at) != Some(&b'"') {
                return Err(ProtocolError::InvalidJson);
            }
            self.at += 1;
            while let Some(byte) = self.bytes.get(self.at) {
                match *byte {
                    b'"' => {
                        self.at += 1;
                        let string = std::str::from_utf8(&self.bytes[start..self.at])
                            .map_err(|_| ProtocolError::InvalidJson)?;
                        return match miniserde::json::from_str::<Value>(string)
                            .map_err(|_| ProtocolError::InvalidJson)?
                        {
                            Value::String(value) => Ok(value),
                            _ => Err(ProtocolError::InvalidJson),
                        };
                    }
                    b'\\' => {
                        self.at += 1;
                        if self.bytes.get(self.at).is_none() {
                            return Err(ProtocolError::InvalidJson);
                        }
                        self.at += 1;
                    }
                    0..=31 => return Err(ProtocolError::InvalidJson),
                    _ => self.at += 1,
                }
            }
            Err(ProtocolError::InvalidJson)
        }
        fn value(&mut self, depth: usize) -> Result<(), ProtocolError> {
            if depth > MAX_JSON_NESTING {
                return Err(ProtocolError::NestingTooDeep);
            }
            self.whitespace();
            match self.bytes.get(self.at) {
                Some(b'"') => {
                    self.string()?;
                    Ok(())
                }
                Some(b'{') => self.object(depth + 1),
                Some(b'[') => self.array(depth + 1),
                Some(_) => self.primitive(),
                None => Err(ProtocolError::InvalidJson),
            }
        }
        fn object(&mut self, depth: usize) -> Result<(), ProtocolError> {
            if depth > MAX_JSON_NESTING {
                return Err(ProtocolError::NestingTooDeep);
            }
            self.at += 1;
            self.whitespace();
            let mut keys = BTreeSet::new();
            if self.bytes.get(self.at) == Some(&b'}') {
                self.at += 1;
                return Ok(());
            }
            loop {
                self.whitespace();
                let key = self.string()?;
                if !keys.insert(key) {
                    return Err(ProtocolError::DuplicateObjectKey);
                }
                self.whitespace();
                if self.bytes.get(self.at) != Some(&b':') {
                    return Err(ProtocolError::InvalidJson);
                }
                self.at += 1;
                self.value(depth)?;
                self.whitespace();
                match self.bytes.get(self.at) {
                    Some(b'}') => {
                        self.at += 1;
                        return Ok(());
                    }
                    Some(b',') => self.at += 1,
                    _ => return Err(ProtocolError::InvalidJson),
                }
            }
        }
        fn array(&mut self, depth: usize) -> Result<(), ProtocolError> {
            if depth > MAX_JSON_NESTING {
                return Err(ProtocolError::NestingTooDeep);
            }
            self.at += 1;
            self.whitespace();
            if self.bytes.get(self.at) == Some(&b']') {
                self.at += 1;
                return Ok(());
            }
            loop {
                self.value(depth)?;
                self.whitespace();
                match self.bytes.get(self.at) {
                    Some(b']') => {
                        self.at += 1;
                        return Ok(());
                    }
                    Some(b',') => self.at += 1,
                    _ => return Err(ProtocolError::InvalidJson),
                }
            }
        }
        fn primitive(&mut self) -> Result<(), ProtocolError> {
            let start = self.at;
            while let Some(byte) = self.bytes.get(self.at) {
                if matches!(*byte, b',' | b']' | b'}' | b' ' | b'\n' | b'\r' | b'\t') {
                    break;
                }
                self.at += 1;
            }
            if start == self.at {
                return Err(ProtocolError::InvalidJson);
            }
            let primitive = &self.bytes[start..self.at];
            // The TS evidence projector rejects negative zero because
            // JSON.stringify would rewrite it as `0`; mirror that guarantee
            // before an opaque Pi evidence value reaches the peer.
            if is_negative_zero_number(primitive) {
                return Err(ProtocolError::NegativeZero);
            }
            Ok(())
        }
    }
    let mut scanner = Scanner {
        bytes: input.as_bytes(),
        at: 0,
    };
    scanner.value(0)?;
    scanner.whitespace();
    if scanner.at == input.len() {
        Ok(())
    } else {
        Err(ProtocolError::InvalidJson)
    }
}

fn is_negative_zero_number(value: &[u8]) -> bool {
    if value.first() != Some(&b'-') {
        return false;
    }
    let mut at = 1;
    let mut integer_digits = 0;
    let mut integer_zero = true;
    while let Some(byte) = value.get(at).copied() {
        if !byte.is_ascii_digit() {
            break;
        }
        integer_digits += 1;
        integer_zero &= byte == b'0';
        at += 1;
    }
    if integer_digits == 0 {
        return false;
    }
    let mut fraction_digits = 0;
    let mut fraction_zero = true;
    if value.get(at) == Some(&b'.') {
        at += 1;
        while let Some(byte) = value.get(at).copied() {
            if !byte.is_ascii_digit() {
                break;
            }
            fraction_digits += 1;
            fraction_zero &= byte == b'0';
            at += 1;
        }
        if fraction_digits == 0 {
            return false;
        }
    }
    if matches!(value.get(at), Some(b'e' | b'E')) {
        at += 1;
        if matches!(value.get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        let exponent_start = at;
        while value.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if exponent_start == at {
            return false;
        }
    }
    at == value.len() && integer_zero && fraction_zero
}

#[cfg(test)]
mod protocol_tests {
    // These are closed fixture constructors and assertion boundaries; panicking
    // keeps invalid test data local and legible without weakening production code.
    #![allow(clippy::unwrap_used)]

    use crate::forum::{FORUM_F0_AWARENESS_BLAKE3, FORUM_F0_TOOL_CONTRACT_BLAKE3};

    use super::*;

    fn json_value(input: &str) -> Value {
        miniserde::json::from_str(input).unwrap()
    }

    fn create_session_fixture(forum_contract: Value) -> String {
        let frame = json_object!(
            "protocolVersion" => json_string(ADAPTER_PROTOCOL_VERSION),
            "sequence" => json_u64(1),
            "sessionIdentity" => json_string("session-001"),
            "correlationIdentity" => json_string("create-001"),
            "command" => json_string("CreateSession"),
            "payload" => json_object!(
                "sessionKind" => json_string("TaskAttempt"),
                "cwd" => json_string("/tmp/cwd"),
                "agentDirectory" => json_string("/tmp/agent"),
                "authPath" => json_string("/tmp/agent/auth.json"),
                "modelsPath" => json_string("/tmp/agent/models.json"),
                "sessionDirectory" => json_string("/tmp/sessions"),
                "systemPrompt" => json_string("mission"),
                "systemPromptDigest" => json_string("a".repeat(64).as_str()),
                "model" => json_object!(
                    "provider" => json_string("openrouter"),
                    "modelId" => json_string(PINNED_MODEL),
                    "thinkingLevel" => json_string(PINNED_THINKING_LEVEL),
                ),
                "modelCatalog" => json_object!(
                    "catalogBlake3" => json_string("b".repeat(64).as_str()),
                    "effectiveModel" => json_object!(
                        "provider" => json_string("openrouter"),
                        "baseUrl" => json_string(PINNED_OPENROUTER_BASE_URL),
                        "api" => json_string("openai-completions"),
                        "modelId" => json_string(PINNED_MODEL),
                        "canonicalSlug" => json_string(PINNED_CANONICAL_MODEL_SLUG),
                        "input" => json_string("text_only"),
                        "contextWindow" => json_u64(1_048_576),
                        "maxTokens" => json_u64(384_000),
                        "inputUsdPerMillion" => json_object!("kind" => json_string("Known"), "usdPerMillion" => json_string("0.09")),
                        "outputUsdPerMillion" => json_object!("kind" => json_string("Known"), "usdPerMillion" => json_string("0.18")),
                        "cacheReadUsdPerMillion" => json_object!("kind" => json_string("Known"), "usdPerMillion" => json_string("0.018")),
                        "cacheWriteUsdPerMillion" => json_object!("kind" => json_string("Absent")),
                    ),
                ),
                "toolProfile" => json_string("read_execute_v1"),
                "settings" => json_object!(
                    "retry" => json_object!(
                        "maxRetries" => json_u64(2),
                        "baseDelayMilliseconds" => json_u64(2_000),
                        "providerTimeoutMilliseconds" => json_u64(300_000),
                        "providerMaxRetries" => json_u64(1),
                        "providerMaxRetryDelayMilliseconds" => json_u64(30_000),
                    ),
                    "compaction" => json_object!(
                        "mode" => json_string("enabled"),
                        "reserveTokens" => json_u64(16_384),
                        "keepRecentTokens" => json_u64(20_000),
                    ),
                    "steeringMode" => json_string("one-at-a-time"),
                    "followUpMode" => json_string("one-at-a-time"),
                    "transport" => json_string("sse"),
                    "projectTrust" => json_string("never"),
                    "installTelemetryEnabled" => json_bool(false),
                    "analyticsEnabled" => json_bool(false),
                    "images" => json_string("blocked"),
                ),
                "forumContract" => forum_contract,
            ),
        );
        miniserde::json::to_string(&frame)
    }

    #[test]
    fn workspace_isolated_tool_profile_is_closed_and_has_no_shell_or_search_tools() {
        let profile = tool_profile("workspace_isolated_v1").unwrap();
        assert_eq!(
            profile.tools(),
            &[
                PiToolName::Read,
                PiToolName::Edit,
                PiToolName::Write,
                PiToolName::Ls,
            ]
        );
        assert_eq!(tool_profile_wire(profile), "workspace_isolated_v1");
    }

    #[test]
    fn rejects_duplicate_nested_keys_and_negative_zero() {
        assert_eq!(
            decode_inbound_jsonl(
                r#"{"protocolVersion":"society-pi-host/v4","sequence":1,"sequence":2,"sessionIdentity":"session-1","correlationIdentity":"correlation-1","command":"GetState","payload":{}}"#
            ),
            Err(ProtocolError::DuplicateObjectKey)
        );
        assert_eq!(
            decode_inbound_jsonl(
                r#"{"protocolVersion":"society-pi-host/v4","se\u0071uence":1,"sequence":2,"sessionIdentity":"session-1","correlationIdentity":"correlation-1","command":"GetState","payload":{}}"#
            ),
            Err(ProtocolError::DuplicateObjectKey)
        );
        assert_eq!(
            decode_inbound_jsonl(
                r#"{"protocolVersion":"society-pi-host/v4","sequence":-0,"sessionIdentity":"session-1","correlationIdentity":"correlation-1","command":"GetState","payload":{}}"#
            ),
            Err(ProtocolError::NegativeZero)
        );
    }

    #[test]
    fn bounded_stack_rejects_deep_submegabyte_json_before_recursion_can_escape() {
        let deeply_nested = format!("{}{}", "[".repeat(10_000), "]".repeat(10_000));
        assert!(deeply_nested.len() < MAX_JSONL_FRAME_BYTES);
        let thread = std::thread::Builder::new()
            .name("bounded-jsonl-scan".into())
            .stack_size(512 * 1024)
            .spawn(move || decode_inbound_jsonl(&deeply_nested))
            .unwrap();
        assert_eq!(thread.join().unwrap(), Err(ProtocolError::NestingTooDeep));
    }

    #[test]
    fn rejects_every_json_negative_zero_spelling_in_opaque_agent_evidence() {
        for spelling in ["-0", "-0.0", "-0e0", "-0E+10"] {
            let line = format!(
                r#"{{"protocolVersion":"society-pi-host/v4","sequence":1,"sessionIdentity":"session-001","event":"AgentEvent","agentEvent":{{"type":"message_end","message":{{"value":{spelling}}}}}}}"#
            );
            assert_eq!(
                decode_outbound_jsonl(&line),
                Err(ProtocolError::NegativeZero),
                "{spelling}"
            );
        }
        let nonzero = r#"{"protocolVersion":"society-pi-host/v4","sequence":1,"sessionIdentity":"session-001","event":"AgentEvent","agentEvent":{"type":"message_end","message":{"value":-1}}}"#;
        assert!(decode_outbound_jsonl(nonzero).is_ok());
    }

    #[test]
    fn strict_get_state_and_path() {
        assert!(decode_inbound_jsonl(r#"{"protocolVersion":"society-pi-host/v4","sequence":1,"sessionIdentity":"session-1","correlationIdentity":"correlation-1","command":"GetState","payload":{"x":1}}"#).is_err());
        assert!(AbsolutePath::parse("/tmp/../secret").is_err());
        assert!(AbsolutePath::parse("/tmp//secret").is_err());
    }

    #[test]
    fn create_session_forum_contract_is_digest_bound_and_closed() {
        let valid = json_object!(
            "kind" => json_string("forum_enabled_v1"),
            "awarenessBlake3" => json_string(FORUM_F0_AWARENESS_BLAKE3),
            "toolContractBlake3" => json_string(FORUM_F0_TOOL_CONTRACT_BLAKE3),
        );
        let frame = decode_inbound_jsonl(&create_session_fixture(valid)).unwrap();
        let InboundCommand::CreateSession(payload) = &frame.command else {
            panic!("expected create session");
        };
        assert_eq!(
            payload.forum_contract,
            ForumSessionContractV1::forum_enabled_v1().unwrap()
        );
        assert!(
            encode_inbound_jsonl(&frame)
                .unwrap()
                .contains("forumContract")
        );

        let drifted = json_object!(
            "kind" => json_string("forum_enabled_v1"),
            "awarenessBlake3" => json_string(&"0".repeat(64)),
            "toolContractBlake3" => json_string(FORUM_F0_TOOL_CONTRACT_BLAKE3),
        );
        assert!(decode_inbound_jsonl(&create_session_fixture(drifted)).is_err());

        let sequestered_with_digest = json_object!(
            "kind" => json_string("sequestered_v1"),
            "awarenessBlake3" => json_string(FORUM_F0_AWARENESS_BLAKE3),
        );
        assert!(decode_inbound_jsonl(&create_session_fixture(sequestered_with_digest)).is_err());
    }
    #[test]
    fn encode_reenters_the_same_closed_inbound_decoder() {
        let frame = InboundFrame {
            sequence: BoundarySequence::parse(1).unwrap(),
            session_identity: SessionIdentity::parse("session-001").unwrap(),
            correlation_identity: CorrelationIdentity::parse("correlation-001").unwrap(),
            command: InboundCommand::GetState,
        };
        let line = encode_inbound_jsonl(&frame).unwrap();
        assert_eq!(decode_inbound_jsonl(&line).unwrap(), frame);
    }
    #[test]
    fn pinned_agent_event_fixture_covers_every_host_v1_variant() {
        let events = vec![
            json_value(r#"{"type":"agent_start"}"#),
            json_value(
                r#"{"type":"agent_end","messages":[{"type":"assistant"}],"willRetry":true}"#,
            ),
            json_value(r#"{"type":"agent_settled"}"#),
            json_value(r#"{"type":"turn_start"}"#),
            json_value(r#"{"type":"turn_end","message":{"type":"assistant"},"toolResults":[]}"#),
            json_value(r#"{"type":"message_start","message":{"role":"assistant"}}"#),
            json_value(
                r#"{"type":"message_update","message":{"role":"assistant"},"assistantMessageEvent":{"type":"text_delta","delta":"x"}}"#,
            ),
            json_value(r#"{"type":"message_end","message":{"role":"assistant"}}"#),
            json_value(
                r#"{"type":"tool_execution_start","toolCallIdentity":"call-001","toolName":"read","args":{"path":"society"}}"#,
            ),
            json_value(
                r#"{"type":"tool_execution_update","toolCallIdentity":"call-001","toolName":"read","args":{"path":"society"},"partialResult":"part"}"#,
            ),
            json_value(
                r#"{"type":"tool_execution_end","toolCallIdentity":"call-001","toolName":"read","result":"done","isError":false}"#,
            ),
            json_value(r#"{"type":"queue_update","steering":["urgent"],"followUp":["notice"]}"#),
            json_value(r#"{"type":"entry_appended","entry":{"type":"message"}}"#),
            json_value(
                r#"{"type":"bash_execution_update","executionIdentity":"bash-001","delta":"output"}"#,
            ),
            json_value(r#"{"type":"compaction_start","reason":"threshold"}"#),
            json_value(r#"{"type":"session_info_changed","name":"session"}"#),
            json_value(r#"{"type":"thinking_level_changed","level":"high"}"#),
            json_value(
                r#"{"type":"compaction_end","reason":"overflow","result":{"summary":"ok"},"aborted":false,"willRetry":true,"errorMessage":"retry"}"#,
            ),
            json_value(
                r#"{"type":"auto_retry_start","attempt":1,"maxAttempts":2,"delayMilliseconds":2000,"errorMessage":"network"}"#,
            ),
            json_value(r#"{"type":"auto_retry_end","success":true,"attempt":1}"#),
            json_value(
                r#"{"type":"summarization_retry_scheduled","attempt":1,"maxAttempts":2,"delayMilliseconds":2000,"errorMessage":"summary"}"#,
            ),
            json_value(r#"{"type":"summarization_retry_attempt_start","source":"branchSummary"}"#),
            json_value(
                r#"{"type":"summarization_retry_attempt_start","source":"compaction","reason":"manual"}"#,
            ),
            json_value(r#"{"type":"summarization_retry_finished"}"#),
        ];
        for (sequence, agent_event) in events.into_iter().enumerate() {
            let frame = json_object!(
                "protocolVersion" => json_string(ADAPTER_PROTOCOL_VERSION),
                "sequence" => json_u64(sequence as u64 + 1),
                "sessionIdentity" => json_string("session-001"),
                "event" => json_string("AgentEvent"),
                "agentEvent" => agent_event,
            );
            assert!(matches!(
                decode_outbound_jsonl(&miniserde::json::to_string(&frame)),
                Ok(OutboundFrame {
                    event: OutboundEvent::AgentEvent { .. },
                    ..
                })
            ));
        }
    }
    #[test]
    fn unknown_event_and_oversize_frame_are_not_fallback_data() {
        let unknown = json_object!(
            "protocolVersion" => json_string(ADAPTER_PROTOCOL_VERSION),
            "sequence" => json_u64(1),
            "sessionIdentity" => json_string("session-001"),
            "event" => json_string("AgentEvent"),
            "agentEvent" => json_value(r#"{"type":"future_sdk_event"}"#),
        );
        assert!(decode_outbound_jsonl(&miniserde::json::to_string(&unknown)).is_err());
        assert_eq!(
            decode_outbound_jsonl(&"x".repeat(MAX_JSONL_FRAME_BYTES + 1)),
            Err(ProtocolError::FrameTooLarge)
        );
    }
}
