/**
 * The Pi SDK host's only wire contract. JSON is allowed here because Rust and
 * TypeScript cannot otherwise exchange Pi SDK commands and events. Every tag
 * is nevertheless a closed discriminated union; generic envelopes and
 * metadata maps are deliberately absent.
 */

import { isAbsolute, normalize } from "node:path";

export const ADAPTER_PROTOCOL_VERSION = "society-pi-host/v1" as const;
export const ADAPTER_VERSION = "1" as const;
export const PINNED_PI_SDK_VERSION = "0.83.0" as const;
export const PINNED_PROVIDER = "openrouter" as const;
export const PINNED_MODEL = "deepseek/deepseek-v4-flash-0731" as const;
export const PINNED_CANONICAL_MODEL_SLUG = "deepseek/deepseek-v4-flash-20260731" as const;
export const PINNED_OPENROUTER_BASE_URL = "https://openrouter.ai/api/v1" as const;
export const PINNED_THINKING_LEVEL = "high" as const;
export const MAX_JSONL_FRAME_BYTES = 1024 * 1024;
/**
 * The duplicate-key pre-scan is intentionally stack-bounded. A deeply nested
 * sub-megabyte record is malformed v1 input, never an opportunity to exhaust
 * the host before it can produce its typed containment result.
 */
export const MAX_JSON_NESTING = 128;

export type AdapterProtocolVersion = typeof ADAPTER_PROTOCOL_VERSION;
export type AdapterVersion = typeof ADAPTER_VERSION;
export type PiSdkVersion = typeof PINNED_PI_SDK_VERSION;
export type ProviderId = typeof PINNED_PROVIDER;
export type ModelId = typeof PINNED_MODEL;
export type ThinkingLevel = typeof PINNED_THINKING_LEVEL;

declare const sessionIdentityBrand: unique symbol;
declare const correlationIdentityBrand: unique symbol;
declare const spawnNonceBrand: unique symbol;
declare const sha256DigestBrand: unique symbol;
declare const boundarySequenceBrand: unique symbol;
declare const absolutePathBrand: unique symbol;
declare const nonNegativeIntegerBrand: unique symbol;
declare const positiveIntegerBrand: unique symbol;
declare const ledgerFrontierBrand: unique symbol;
declare const binary64BigEndianHexBrand: unique symbol;
declare const nodeRuntimeVersionBrand: unique symbol;
declare const usdPerMillionDecimalBrand: unique symbol;

export type SessionIdentity = string & { readonly [sessionIdentityBrand]: "SessionIdentity" };
export type CorrelationIdentity = string & { readonly [correlationIdentityBrand]: "CorrelationIdentity" };
export type SpawnNonce = string & { readonly [spawnNonceBrand]: "SpawnNonce" };
export type Sha256Digest = string & { readonly [sha256DigestBrand]: "Sha256Digest" };
export type BoundarySequence = number & { readonly [boundarySequenceBrand]: "BoundarySequence" };
export type AbsolutePath = string & { readonly [absolutePathBrand]: "AbsolutePath" };
export type NonNegativeInteger = number & { readonly [nonNegativeIntegerBrand]: "NonNegativeInteger" };
export type PositiveInteger = number & { readonly [positiveIntegerBrand]: "PositiveInteger" };
export type LedgerFrontier = number & { readonly [ledgerFrontierBrand]: "LedgerFrontier" };
export type Binary64BigEndianHex = string & { readonly [binary64BigEndianHexBrand]: "Binary64BigEndianHex" };
export type NodeRuntimeVersion = string & { readonly [nodeRuntimeVersionBrand]: "NodeRuntimeVersion" };
/** Canonical nonzero base-10 USD amount, never a JavaScript display float. */
export type UsdPerMillionDecimal = string & { readonly [usdPerMillionDecimalBrand]: "UsdPerMillionDecimal" };

const ID_PATTERN = /^[A-Za-z0-9](?:[A-Za-z0-9._-]*[A-Za-z0-9])?$/u;
const SHA256_PATTERN = /^[a-f0-9]{64}$/u;
const USD_PER_MILLION_PATTERN = /^(?:0|[1-9][0-9]*)(?:\.[0-9]*[1-9])?$/u;

export function sessionIdentity(value: string): SessionIdentity {
	if (!ID_PATTERN.test(value)) throw new ProtocolDecodeError("invalid_session_identity");
	return value as SessionIdentity;
}

export function correlationIdentity(value: string): CorrelationIdentity {
	if (!ID_PATTERN.test(value)) throw new ProtocolDecodeError("invalid_correlation_identity");
	return value as CorrelationIdentity;
}

export function spawnNonce(value: string): SpawnNonce {
	if (!ID_PATTERN.test(value)) throw new ProtocolDecodeError("invalid_spawn_nonce");
	return value as SpawnNonce;
}

export function sha256Digest(value: string): Sha256Digest {
	if (!SHA256_PATTERN.test(value)) throw new ProtocolDecodeError("invalid_sha256_digest");
	return value as Sha256Digest;
}

export function boundarySequence(value: number): BoundarySequence {
	if (!Number.isSafeInteger(value) || value < 1) throw new ProtocolDecodeError("invalid_boundary_sequence");
	return value as BoundarySequence;
}

export function absolutePath(value: string): AbsolutePath {
	if (
		value.length < 2 ||
		value.includes("\0") ||
		!isAbsolute(value) ||
		value.endsWith("/") ||
		normalize(value) !== value ||
		value.split("/").slice(1).some((segment) => segment === "" || segment === "." || segment === "..")
	) {
		throw new ProtocolDecodeError("invalid_frame");
	}
	return value as AbsolutePath;
}

export function nonNegativeInteger(value: number): NonNegativeInteger {
	if (!Number.isSafeInteger(value) || value < 0) throw new ProtocolDecodeError("invalid_frame");
	return value as NonNegativeInteger;
}

export function positiveInteger(value: number): PositiveInteger {
	if (!Number.isSafeInteger(value) || value < 1) throw new ProtocolDecodeError("invalid_frame");
	return value as PositiveInteger;
}

export function ledgerFrontier(value: number): LedgerFrontier {
	return nonNegativeInteger(value) as unknown as LedgerFrontier;
}

/**
 * Pi exposes provider cost as binary64. The host preserves that exact observed
 * value rather than rounding it down to a display decimal. Rust is charging
 * authority: it decodes this value exactly and applies `ceil(cost * 1e6)`.
 */
export function binary64BigEndianHex(value: number): Binary64BigEndianHex {
	if (!Number.isFinite(value) || value < 0) throw new ProtocolDecodeError("invalid_frame");
	const normalized = Object.is(value, -0) ? 0 : value;
	const buffer = new ArrayBuffer(8);
	const view = new DataView(buffer);
	view.setFloat64(0, normalized, false);
	return view.getBigUint64(0, false).toString(16).padStart(16, "0") as Binary64BigEndianHex;
}

export interface ProviderCostObservationV1 {
	readonly encoding: "ieee754_binary64_be_hex_v1";
	readonly binary64BigEndianHex: Binary64BigEndianHex;
	readonly rounding: "ceil_to_micro_usd";
}

export function providerCostObservation(value: number): ProviderCostObservationV1 {
	return {
		encoding: "ieee754_binary64_be_hex_v1",
		binary64BigEndianHex: binary64BigEndianHex(value),
		rounding: "ceil_to_micro_usd",
	};
}

export function usdPerMillionDecimal(value: string): UsdPerMillionDecimal {
	if (!USD_PER_MILLION_PATTERN.test(value) || Number(value) <= 0 || !Number.isFinite(Number(value))) {
		throw new ProtocolDecodeError("invalid_frame");
	}
	return value as UsdPerMillionDecimal;
}

export function nodeRuntimeVersion(value: string): NodeRuntimeVersion {
	const match = /^v?(\d+)\.(\d+)\.(\d+)$/u.exec(value);
	if (match === null) throw new ProtocolDecodeError("invalid_frame");
	const major = Number(match[1]);
	const minor = Number(match[2]);
	if (!Number.isSafeInteger(major) || !Number.isSafeInteger(minor) || major < 22 || (major === 22 && minor < 19)) {
		throw new ProtocolDecodeError("invalid_frame");
	}
	return value as NodeRuntimeVersion;
}

export type SessionKind = "TaskAttempt" | "GrandArchitectOffice";
export type ToolProfile = "read_source_v1" | "curator_v1" | "product_builder_v1" | "task_actor_v1";
export type PiToolName = "read" | "bash" | "edit" | "write" | "grep" | "find" | "ls";
export type QueueMode = "all" | "one-at-a-time";
export type CompactionMode = "enabled" | "disabled";
export type PromptPurpose = "TaskAssignment" | "OfficeTurn";
export type SteerReason = "UrgentStalePremise" | "UrgentUnsafePremise";
export type AbortReason = "GracefulCancellation" | "EmergencyStop" | "BudgetGuardrail" | "DaemonRecovery";
export type DisposeReason = "CycleReconciliation" | "ProcessRecovery" | "ProtocolFailure";

const TOOL_PROFILE_TOOLS: Readonly<Record<ToolProfile, readonly PiToolName[]>> = {
	read_source_v1: ["read", "bash", "grep", "find", "ls"],
	curator_v1: ["read", "write"],
	product_builder_v1: ["read", "bash", "edit", "write", "grep", "find", "ls"],
	task_actor_v1: ["read", "bash", "edit", "write", "grep", "find", "ls"],
};

export function toolsForProfile(profile: ToolProfile): readonly PiToolName[] {
	return TOOL_PROFILE_TOOLS[profile];
}

export interface RetryPolicyV1 {
	readonly maxRetries: NonNegativeInteger;
	readonly baseDelayMilliseconds: NonNegativeInteger;
	readonly providerTimeoutMilliseconds: PositiveInteger;
	readonly providerMaxRetries: NonNegativeInteger;
	readonly providerMaxRetryDelayMilliseconds: PositiveInteger;
}

export interface CompactionPolicyV1 {
	readonly mode: CompactionMode;
	readonly reserveTokens: NonNegativeInteger;
	readonly keepRecentTokens: NonNegativeInteger;
}

export interface ActorModelPolicyV1 {
	readonly retry: RetryPolicyV1;
	readonly compaction: CompactionPolicyV1;
	readonly steeringMode: QueueMode;
	readonly followUpMode: QueueMode;
	readonly transport: "sse";
	readonly projectTrust: "never";
	readonly installTelemetryEnabled: false;
	readonly analyticsEnabled: false;
	readonly images: "blocked";
}

/**
 * The current pinned Pi SDK profile has one treatment, not an adjustable
 * family of settings. Keeping the registered numbers here makes an
 * alternative retry/queue configuration impossible to smuggle through the
 * otherwise typed CreateSession boundary.
 */
export const PINNED_ACTOR_MODEL_POLICY_V1: ActorModelPolicyV1 = {
	retry: {
		maxRetries: nonNegativeInteger(2),
		baseDelayMilliseconds: nonNegativeInteger(2_000),
		providerTimeoutMilliseconds: positiveInteger(300_000),
		providerMaxRetries: nonNegativeInteger(1),
		providerMaxRetryDelayMilliseconds: positiveInteger(30_000),
	},
	compaction: {
		mode: "enabled",
		reserveTokens: nonNegativeInteger(16_384),
		keepRecentTokens: nonNegativeInteger(20_000),
	},
	steeringMode: "one-at-a-time",
	followUpMode: "one-at-a-time",
	transport: "sse",
	projectTrust: "never",
	installTelemetryEnabled: false,
	analyticsEnabled: false,
	images: "blocked",
};

export function assertPinnedActorModelPolicy(policy: ActorModelPolicyV1): ActorModelPolicyV1 {
	const expected = PINNED_ACTOR_MODEL_POLICY_V1;
	if (
		policy.retry.maxRetries !== expected.retry.maxRetries ||
		policy.retry.baseDelayMilliseconds !== expected.retry.baseDelayMilliseconds ||
		policy.retry.providerTimeoutMilliseconds !== expected.retry.providerTimeoutMilliseconds ||
		policy.retry.providerMaxRetries !== expected.retry.providerMaxRetries ||
		policy.retry.providerMaxRetryDelayMilliseconds !== expected.retry.providerMaxRetryDelayMilliseconds ||
		policy.compaction.mode !== expected.compaction.mode ||
		policy.compaction.reserveTokens !== expected.compaction.reserveTokens ||
		policy.compaction.keepRecentTokens !== expected.compaction.keepRecentTokens ||
		policy.steeringMode !== expected.steeringMode ||
		policy.followUpMode !== expected.followUpMode ||
		policy.transport !== expected.transport ||
		policy.projectTrust !== expected.projectTrust ||
		policy.installTelemetryEnabled !== expected.installTelemetryEnabled ||
		policy.analyticsEnabled !== expected.analyticsEnabled ||
		policy.images !== expected.images
	) {
		throw new ProtocolDecodeError("invalid_frame");
	}
	return policy;
}

export interface ModelSelection {
	readonly provider: ProviderId;
	readonly modelId: ModelId;
	readonly thinkingLevel: ThinkingLevel;
}

export interface KnownPerMillionRateV1 {
	readonly kind: "Known";
	readonly usdPerMillion: UsdPerMillionDecimal;
}

/** Pi stores cache-write cost numerically; absent catalog pricing normalizes to numeric zero only in Pi. */
export type CacheWritePerMillionRateV1 = KnownPerMillionRateV1 | { readonly kind: "Absent" };

export interface EffectiveModelDescriptorV1 {
	readonly provider: ProviderId;
	readonly baseUrl: typeof PINNED_OPENROUTER_BASE_URL;
	readonly api: "openai-completions";
	readonly modelId: ModelId;
	readonly canonicalSlug: typeof PINNED_CANONICAL_MODEL_SLUG;
	readonly input: "text_only";
	readonly contextWindow: PositiveInteger;
	readonly maxTokens: PositiveInteger;
	readonly inputUsdPerMillion: KnownPerMillionRateV1;
	readonly outputUsdPerMillion: KnownPerMillionRateV1;
	readonly cacheReadUsdPerMillion: KnownPerMillionRateV1;
	readonly cacheWriteUsdPerMillion: CacheWritePerMillionRateV1;
}

/** The Rust-admitted raw models.json bytes and its complete billing treatment. */
export interface ModelCatalogPolicyV1 {
	readonly catalogSha256: Sha256Digest;
	readonly effectiveModel: EffectiveModelDescriptorV1;
}

export const PINNED_EFFECTIVE_MODEL_DESCRIPTOR_V1: Omit<EffectiveModelDescriptorV1, "provider"> = {
	baseUrl: PINNED_OPENROUTER_BASE_URL,
	api: "openai-completions",
	modelId: PINNED_MODEL,
	canonicalSlug: PINNED_CANONICAL_MODEL_SLUG,
	input: "text_only",
	contextWindow: positiveInteger(1_048_576),
	maxTokens: positiveInteger(384_000),
	inputUsdPerMillion: { kind: "Known", usdPerMillion: usdPerMillionDecimal("0.09") },
	outputUsdPerMillion: { kind: "Known", usdPerMillion: usdPerMillionDecimal("0.18") },
	cacheReadUsdPerMillion: { kind: "Known", usdPerMillion: usdPerMillionDecimal("0.018") },
	cacheWriteUsdPerMillion: { kind: "Absent" },
};

export function assertPinnedModelCatalogPolicy(policy: ModelCatalogPolicyV1): ModelCatalogPolicyV1 {
	const model = policy.effectiveModel;
	const expected = PINNED_EFFECTIVE_MODEL_DESCRIPTOR_V1;
	if (
		model.provider !== PINNED_PROVIDER || model.baseUrl !== expected.baseUrl || model.api !== expected.api ||
		model.modelId !== expected.modelId || model.canonicalSlug !== expected.canonicalSlug || model.input !== expected.input ||
		model.contextWindow !== expected.contextWindow || model.maxTokens !== expected.maxTokens ||
		!sameKnownRate(model.inputUsdPerMillion, expected.inputUsdPerMillion) ||
		!sameKnownRate(model.outputUsdPerMillion, expected.outputUsdPerMillion) ||
		!sameKnownRate(model.cacheReadUsdPerMillion, expected.cacheReadUsdPerMillion) ||
		model.cacheWriteUsdPerMillion.kind !== "Absent"
	) throw new ProtocolDecodeError("invalid_frame");
	return policy;
}

function sameKnownRate(left: KnownPerMillionRateV1, right: KnownPerMillionRateV1): boolean {
	return left.kind === right.kind && left.usdPerMillion === right.usdPerMillion;
}

/** All fields are supplied by Rust from the admitted execution profile. */
export interface CreateSessionPayload {
	readonly sessionKind: SessionKind;
	readonly cwd: AbsolutePath;
	readonly agentDirectory: AbsolutePath;
	readonly authPath: AbsolutePath;
	readonly modelsPath: AbsolutePath;
	readonly sessionDirectory: AbsolutePath;
	readonly systemPrompt: string;
	readonly systemPromptDigest: Sha256Digest;
	readonly model: ModelSelection;
	readonly modelCatalog: ModelCatalogPolicyV1;
	readonly toolProfile: ToolProfile;
	readonly settings: ActorModelPolicyV1;
}

export interface PromptPayload {
	readonly purpose: PromptPurpose;
	readonly text: string;
}

export interface FollowUpPayload {
	readonly noticeDeliveryIdentity: CorrelationIdentity;
	readonly ledgerFrontier: LedgerFrontier;
	readonly text: string;
}

export interface SteerPayload {
	readonly reason: SteerReason;
	readonly text: string;
}

export interface AbortPayload {
	readonly reason: AbortReason;
}

export interface DisposePayload {
	readonly reason: DisposeReason;
}

interface InboundFrameBase {
	readonly protocolVersion: AdapterProtocolVersion;
	readonly sequence: BoundarySequence;
	readonly sessionIdentity: SessionIdentity;
	readonly correlationIdentity: CorrelationIdentity;
}

export interface CreateSessionCommand extends InboundFrameBase {
	readonly command: "CreateSession";
	readonly payload: CreateSessionPayload;
}

export interface PromptCommand extends InboundFrameBase {
	readonly command: "Prompt";
	readonly payload: PromptPayload;
}

export interface FollowUpCommand extends InboundFrameBase {
	readonly command: "FollowUp";
	readonly payload: FollowUpPayload;
}

export interface SteerCommand extends InboundFrameBase {
	readonly command: "Steer";
	readonly payload: SteerPayload;
}

export interface AbortCommand extends InboundFrameBase {
	readonly command: "Abort";
	readonly payload: AbortPayload;
}

export interface GetStateCommand extends InboundFrameBase {
	readonly command: "GetState";
	readonly payload: Record<never, never>;
}

export interface DisposeCommand extends InboundFrameBase {
	readonly command: "Dispose";
	readonly payload: DisposePayload;
}

export type InboundFrame =
	| CreateSessionCommand
	| PromptCommand
	| FollowUpCommand
	| SteerCommand
	| AbortCommand
	| GetStateCommand
	| DisposeCommand;

export type AdapterPhase = "Inert" | "Creating" | "Ready" | "Closing" | "Disposed" | "Fatal";
export type AdapterFailureCode =
	| "invalid_command"
	| "invalid_state"
	| "sequence_gap"
	| "session_identity_mismatch"
	| "execution_profile_drift"
	| "sdk_operation_failed"
	| "missing_agent_settled"
	| "missing_final_assistant_outcome"
	| "protocol_decode_failed"
	| "outbound_frame_too_large";

export interface RuntimeIdentity {
	readonly nodeVersion: NodeRuntimeVersion;
	readonly adapterVersion: AdapterVersion;
	readonly piSdkVersion: PiSdkVersion;
	/** Supervisor-bound evidence; the host validates shape and emits it intact. */
	readonly nodeExecutableSha256: Sha256Digest;
	readonly lockfileSha256: Sha256Digest;
	readonly adapterBuildSha256: Sha256Digest;
	readonly piTransitivePackageSetSha256: Sha256Digest;
}

export interface EffectiveSessionConfiguration {
	readonly sessionKind: SessionKind;
	readonly cwd: AbsolutePath;
	readonly sessionDirectory: AbsolutePath;
	readonly sessionFile: AbsolutePath;
	readonly model: ModelSelection;
	/** Observed after ModelRuntime construction against the raw catalog digest. */
	readonly modelCatalog: ModelCatalogPolicyV1;
	readonly toolProfile: ToolProfile;
	readonly tools: readonly PiToolName[];
	readonly settings: ActorModelPolicyV1;
}

export interface UsageTotals {
	readonly inputTokens: NonNegativeInteger;
	readonly outputTokens: NonNegativeInteger;
	readonly cacheReadTokens: NonNegativeInteger;
	readonly cacheWriteTokens: NonNegativeInteger;
	readonly totalTokens: NonNegativeInteger;
	readonly providerCost: ProviderCostObservationV1;
}

export type UsageObservation =
	| { readonly kind: "Known"; readonly totals: UsageTotals }
	| {
			readonly kind: "Unavailable";
			readonly reason: "invalid_sdk_usage" | "usage_regressed" | "usage_inconsistent";
	  };

/** JSON-safe SDK evidence is intentionally lossless at this boundary. */
export type SdkJsonValue = null | boolean | number | string | readonly SdkJsonValue[] | { readonly [key: string]: SdkJsonValue };

export type ProjectedAgentEvent =
	| { readonly type: "agent_start" }
	| { readonly type: "agent_end"; readonly messages: readonly SdkJsonValue[]; readonly willRetry: boolean }
	| { readonly type: "agent_settled" }
	| { readonly type: "turn_start" }
	| { readonly type: "turn_end"; readonly message: SdkJsonValue; readonly toolResults: readonly SdkJsonValue[] }
	| { readonly type: "message_start"; readonly message: SdkJsonValue }
	| { readonly type: "message_update"; readonly message: SdkJsonValue; readonly assistantMessageEvent: SdkJsonValue }
	| { readonly type: "message_end"; readonly message: SdkJsonValue }
	| { readonly type: "tool_execution_start"; readonly toolCallIdentity: string; readonly toolName: string; readonly args: SdkJsonValue }
	| {
			readonly type: "tool_execution_update";
			readonly toolCallIdentity: string;
			readonly toolName: string;
			readonly args: SdkJsonValue;
			readonly partialResult: SdkJsonValue;
	  }
	| { readonly type: "tool_execution_end"; readonly toolCallIdentity: string; readonly toolName: string; readonly result: SdkJsonValue; readonly isError: boolean }
	| { readonly type: "queue_update"; readonly steering: readonly string[]; readonly followUp: readonly string[] }
	| { readonly type: "entry_appended"; readonly entry: SdkJsonValue }
	| { readonly type: "bash_execution_update"; readonly executionIdentity?: string; readonly delta: string }
	| { readonly type: "compaction_start"; readonly reason: "manual" | "threshold" | "overflow" }
	| { readonly type: "session_info_changed"; readonly name?: string }
	| { readonly type: "thinking_level_changed"; readonly level: "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" }
	| {
			readonly type: "compaction_end";
			readonly reason: "manual" | "threshold" | "overflow";
			readonly result?: SdkJsonValue;
			readonly aborted: boolean;
			readonly willRetry: boolean;
			readonly errorMessage?: string;
	  }
	| { readonly type: "auto_retry_start"; readonly attempt: number; readonly maxAttempts: number; readonly delayMilliseconds: number; readonly errorMessage: string }
	| { readonly type: "auto_retry_end"; readonly success: boolean; readonly attempt: number; readonly finalError?: string }
	| {
			readonly type: "summarization_retry_scheduled";
			readonly attempt: number;
			readonly maxAttempts: number;
			readonly delayMilliseconds: number;
			readonly errorMessage: string;
	  }
	| { readonly type: "summarization_retry_attempt_start"; readonly source: "branchSummary" | "compaction"; readonly reason?: "manual" | "threshold" | "overflow" }
	| { readonly type: "summarization_retry_finished" };

interface OutboundFrameBase {
	readonly protocolVersion: AdapterProtocolVersion;
	readonly sequence: BoundarySequence;
	readonly sessionIdentity: SessionIdentity;
}

export interface AdapterReadyFrame extends OutboundFrameBase {
	readonly event: "AdapterReady";
	readonly pid: number;
	readonly spawnNonce: SpawnNonce;
	readonly runtime: RuntimeIdentity;
}

export interface SessionReadyFrame extends OutboundFrameBase {
	readonly event: "SessionReady";
	readonly correlationIdentity: CorrelationIdentity;
	readonly configuration: EffectiveSessionConfiguration;
}

export type SessionLiveness = "inert" | "creating" | "idle" | "active" | "closing" | "disposed" | "fatal";

export type CommandResultDetail =
	| { readonly kind: "acknowledged" }
	| { readonly kind: "state"; readonly phase: AdapterPhase; readonly liveness: SessionLiveness; readonly sessionFile?: AbsolutePath };

export interface AcceptedCommandResultFrame extends OutboundFrameBase {
	readonly event: "CommandResult";
	readonly correlationIdentity: CorrelationIdentity;
	readonly command: InboundFrame["command"];
	readonly accepted: true;
	readonly detail: CommandResultDetail;
}

export interface RejectedCommandResultFrame extends OutboundFrameBase {
	readonly event: "CommandResult";
	readonly correlationIdentity: CorrelationIdentity;
	readonly command: InboundFrame["command"];
	readonly accepted: false;
	readonly detail: { readonly kind: "rejected" };
	readonly failureCode: AdapterFailureCode;
}

export type CommandResultFrame = AcceptedCommandResultFrame | RejectedCommandResultFrame;

export interface AgentEventFrame extends OutboundFrameBase {
	readonly event: "AgentEvent";
	readonly correlationIdentity?: CorrelationIdentity;
	readonly agentEvent: ProjectedAgentEvent;
}

export interface UsageSnapshotFrame extends OutboundFrameBase {
	readonly event: "UsageSnapshot";
	readonly correlationIdentity?: CorrelationIdentity;
	readonly usage: UsageObservation;
}

export interface SettledFrame extends OutboundFrameBase {
	readonly event: "Settled";
	readonly correlationIdentity: CorrelationIdentity;
	readonly classification: "completed" | "length" | "error" | "aborted" | "failed" | "protocol_failed";
	/** The last non-retried assistant outcome is not inferred from process exit. */
	readonly finalAssistantOutcome: FinalAssistantOutcome;
}

export type FinalAssistantOutcome =
	| { readonly kind: "Observed"; readonly stopReason: "stop" | "length" | "error" | "aborted" }
	| { readonly kind: "Unavailable"; readonly reason: "sdk_promise_rejected" | "missing_final_assistant_outcome" };

export interface DisposedFrame extends OutboundFrameBase {
	readonly event: "Disposed";
	readonly correlationIdentity: CorrelationIdentity;
	readonly transcriptFlushReceipt: TranscriptFlushReceiptV1;
}

/** What the synchronous Pi dispose was followed by and physically observed. */
export type TranscriptFlushReceiptV1 = MaterializedTranscriptFlushReceiptV1 | UnmaterializedTranscriptFlushReceiptV1;

export interface MaterializedTranscriptFlushReceiptV1 {
	readonly format: "pi_session_manager_jsonl_v3";
	readonly sessionIdentity: SessionIdentity;
	readonly sessionFile: AbsolutePath;
	readonly materialization: "observed";
	readonly sessionFileSha256: Sha256Digest;
	readonly headerCwd: AbsolutePath;
	readonly firstUserPrompt: FirstUserPromptReceipt;
}

/** Pi 0.83 lazily creates a session JSONL only after durable message activity. */
export interface UnmaterializedTranscriptFlushReceiptV1 {
	readonly format: "pi_session_manager_jsonl_v3";
	readonly sessionIdentity: SessionIdentity;
	readonly sessionFile: AbsolutePath;
	readonly materialization: "unmaterialized_no_prompt";
	readonly firstUserPrompt: { readonly kind: "absent" };
}

export type FirstUserPromptReceipt =
	| { readonly kind: "absent" }
	| { readonly kind: "verified"; readonly digest: Sha256Digest };

export interface FatalFrame extends OutboundFrameBase {
	readonly event: "Fatal";
	readonly failureCode: AdapterFailureCode;
}

export type OutboundFrame =
	| AdapterReadyFrame
	| SessionReadyFrame
	| CommandResultFrame
	| AgentEventFrame
	| UsageSnapshotFrame
	| SettledFrame
	| DisposedFrame
	| FatalFrame;

export class ProtocolDecodeError extends Error {
	constructor(readonly code: "invalid_session_identity" | "invalid_correlation_identity" | "invalid_spawn_nonce" | "invalid_sha256_digest" | "invalid_boundary_sequence" | "invalid_frame") {
		super(code);
		this.name = "ProtocolDecodeError";
	}
}

export function decodeInboundJsonl(line: string): InboundFrame {
	if (Buffer.byteLength(line, "utf8") > MAX_JSONL_FRAME_BYTES) throw new ProtocolDecodeError("invalid_frame");
	let parsed: unknown;
	try {
		assertNoDuplicateObjectKeys(line);
		parsed = JSON.parse(line) as unknown;
	} catch {
		throw new ProtocolDecodeError("invalid_frame");
	}
	const frame = requiredRecord(parsed);
	requireExactKeys(frame, ["protocolVersion", "sequence", "sessionIdentity", "correlationIdentity", "command", "payload"]);
	if (requiredLiteral(frame, "protocolVersion", ADAPTER_PROTOCOL_VERSION) !== ADAPTER_PROTOCOL_VERSION) {
		throw new ProtocolDecodeError("invalid_frame");
	}
	const base = {
		protocolVersion: ADAPTER_PROTOCOL_VERSION,
		sequence: boundarySequence(requiredSafeInteger(frame, "sequence")),
		sessionIdentity: sessionIdentity(requiredString(frame, "sessionIdentity")),
		correlationIdentity: correlationIdentity(requiredString(frame, "correlationIdentity")),
	} as const;
	const command = requiredString(frame, "command");
	const payload = requiredRecord(frame.payload);

	switch (command) {
		case "CreateSession":
			return { ...base, command, payload: decodeCreateSessionPayload(payload) };
		case "Prompt":
			return { ...base, command, payload: decodePromptPayload(payload) };
		case "FollowUp":
			return { ...base, command, payload: decodeFollowUpPayload(payload) };
		case "Steer":
			return { ...base, command, payload: decodeSteerPayload(payload) };
		case "Abort":
			return { ...base, command, payload: decodeAbortPayload(payload) };
		case "GetState":
			requireExactKeys(payload, []);
			return { ...base, command, payload: {} };
		case "Dispose":
			return { ...base, command, payload: decodeDisposePayload(payload) };
		default:
			throw new ProtocolDecodeError("invalid_frame");
	}
}

/**
 * JSON.parse deliberately gives the last duplicate object member precedence.
 * The control protocol cannot allow that ambiguity: Rust and Node must admit
 * exactly the same command, so duplicate keys are rejected before decoding.
 */
function assertNoDuplicateObjectKeys(input: string): void {
	let cursor = 0;
	const whitespace = (): void => {
		while (/\s/u.test(input[cursor] ?? "")) cursor += 1;
	};
	const string = (): string => {
		if (input[cursor] !== '"') throw new Error("invalid_json");
		const begin = cursor;
		cursor += 1;
		while (cursor < input.length) {
			const character = input[cursor++];
			if (character === "\\") {
				const escaped = input[cursor++];
				if (escaped === "u") cursor += 4;
				continue;
			}
			if (character === '"') return JSON.parse(input.slice(begin, cursor)) as string;
			if (character === undefined || character < " ") throw new Error("invalid_json");
		}
		throw new Error("invalid_json");
	};
	const primitive = (): void => {
		const match = /^(?:true|false|null|-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)/u.exec(input.slice(cursor));
		if (match === null) throw new Error("invalid_json");
		// JSON.parse preserves the sign bit for -0 spellings, but later numeric
		// operations and JSON.stringify erase it. Rust rejects every such lexical
		// representation before decoding; so must this peer.
		if (match[0].startsWith("-") && Object.is(Number(match[0]), -0)) {
			throw new Error("negative_zero");
		}
		cursor += match[0].length;
	};
	const value = (depth = 0): void => {
		if (depth > MAX_JSON_NESTING) throw new Error("json_nesting_limit");
		whitespace();
		if (input[cursor] === '"') { string(); return; }
		if (input[cursor] === "{") {
			cursor += 1;
			const keys = new Set<string>();
			whitespace();
			if (input[cursor] === "}") { cursor += 1; return; }
			for (;;) {
				whitespace();
				const key = string();
				if (keys.has(key)) throw new Error("duplicate_json_object_key");
				keys.add(key);
				whitespace();
				if (input[cursor++] !== ":") throw new Error("invalid_json");
				value(depth + 1);
				whitespace();
				if (input[cursor] === "}") { cursor += 1; return; }
				if (input[cursor++] !== ",") throw new Error("invalid_json");
			}
		}
		if (input[cursor] === "[") {
			cursor += 1;
			whitespace();
			if (input[cursor] === "]") { cursor += 1; return; }
			for (;;) {
				value(depth + 1);
				whitespace();
				if (input[cursor] === "]") { cursor += 1; return; }
				if (input[cursor++] !== ",") throw new Error("invalid_json");
			}
		}
		primitive();
	};
	value();
	whitespace();
	if (cursor !== input.length) throw new Error("invalid_json");
}

function decodeCreateSessionPayload(value: Record<string, unknown>): CreateSessionPayload {
	requireExactKeys(value, [
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
	]);
	const model = requiredRecord(value.model);
	requireExactKeys(model, ["provider", "modelId", "thinkingLevel"]);
	const settings = requiredRecord(value.settings);
	const modelCatalog = requiredRecord(value.modelCatalog);
	return {
		sessionKind: requiredOneOf(value, "sessionKind", ["TaskAttempt", "GrandArchitectOffice"] as const),
		cwd: requiredAbsolutePath(value, "cwd"),
		agentDirectory: requiredAbsolutePath(value, "agentDirectory"),
		authPath: requiredAbsolutePath(value, "authPath"),
		modelsPath: requiredAbsolutePath(value, "modelsPath"),
		sessionDirectory: requiredAbsolutePath(value, "sessionDirectory"),
		systemPrompt: requiredNonEmptyString(value, "systemPrompt"),
		systemPromptDigest: sha256Digest(requiredString(value, "systemPromptDigest")),
		model: {
			provider: requiredLiteral(model, "provider", PINNED_PROVIDER),
			modelId: requiredLiteral(model, "modelId", PINNED_MODEL),
			thinkingLevel: requiredLiteral(model, "thinkingLevel", PINNED_THINKING_LEVEL),
		},
		modelCatalog: assertPinnedModelCatalogPolicy(decodeModelCatalogPolicy(modelCatalog)),
		toolProfile: requiredOneOf(value, "toolProfile", [
			"read_source_v1",
			"curator_v1",
			"product_builder_v1",
			"task_actor_v1",
		] as const),
		settings: assertPinnedActorModelPolicy(decodeSettings(settings)),
	};
}

function decodeModelCatalogPolicy(value: Record<string, unknown>): ModelCatalogPolicyV1 {
	requireExactKeys(value, ["catalogSha256", "effectiveModel"]);
	const model = requiredRecord(value.effectiveModel);
	requireExactKeys(model, [
		"provider", "baseUrl", "api", "modelId", "canonicalSlug", "input", "contextWindow", "maxTokens",
		"inputUsdPerMillion", "outputUsdPerMillion", "cacheReadUsdPerMillion", "cacheWriteUsdPerMillion",
	]);
	return {
		catalogSha256: sha256Digest(requiredString(value, "catalogSha256")),
		effectiveModel: {
			provider: requiredLiteral(model, "provider", PINNED_PROVIDER),
			baseUrl: requiredLiteral(model, "baseUrl", PINNED_OPENROUTER_BASE_URL),
			api: requiredLiteral(model, "api", "openai-completions"),
			modelId: requiredLiteral(model, "modelId", PINNED_MODEL),
			canonicalSlug: requiredLiteral(model, "canonicalSlug", PINNED_CANONICAL_MODEL_SLUG),
			input: requiredLiteral(model, "input", "text_only"),
			contextWindow: positiveInteger(requiredSafeInteger(model, "contextWindow")),
			maxTokens: positiveInteger(requiredSafeInteger(model, "maxTokens")),
			inputUsdPerMillion: decodeKnownRate(requiredRecord(model.inputUsdPerMillion)),
			outputUsdPerMillion: decodeKnownRate(requiredRecord(model.outputUsdPerMillion)),
			cacheReadUsdPerMillion: decodeKnownRate(requiredRecord(model.cacheReadUsdPerMillion)),
			cacheWriteUsdPerMillion: decodeCacheWriteRate(requiredRecord(model.cacheWriteUsdPerMillion)),
		},
	};
}

function decodeKnownRate(value: Record<string, unknown>): KnownPerMillionRateV1 {
	requireExactKeys(value, ["kind", "usdPerMillion"]);
	return { kind: requiredLiteral(value, "kind", "Known"), usdPerMillion: usdPerMillionDecimal(requiredString(value, "usdPerMillion")) };
}

function decodeCacheWriteRate(value: Record<string, unknown>): CacheWritePerMillionRateV1 {
	const kind = requiredString(value, "kind");
	switch (kind) {
		case "Absent":
			requireExactKeys(value, ["kind"]);
			return { kind };
		case "Known":
			return decodeKnownRate(value);
		default:
			throw new ProtocolDecodeError("invalid_frame");
	}
}

function decodeSettings(value: Record<string, unknown>): ActorModelPolicyV1 {
	requireExactKeys(value, [
		"retry",
		"compaction",
		"steeringMode",
		"followUpMode",
		"transport",
		"projectTrust",
		"installTelemetryEnabled",
		"analyticsEnabled",
		"images",
	]);
	const retry = requiredRecord(value.retry);
	requireExactKeys(retry, [
		"maxRetries",
		"baseDelayMilliseconds",
		"providerTimeoutMilliseconds",
		"providerMaxRetries",
		"providerMaxRetryDelayMilliseconds",
	]);
	const compaction = requiredRecord(value.compaction);
	requireExactKeys(compaction, ["mode", "reserveTokens", "keepRecentTokens"]);
	return {
		retry: {
			maxRetries: requiredNonNegativeInteger(retry, "maxRetries"),
			baseDelayMilliseconds: requiredNonNegativeInteger(retry, "baseDelayMilliseconds"),
			providerTimeoutMilliseconds: requiredPositiveInteger(retry, "providerTimeoutMilliseconds"),
			providerMaxRetries: requiredNonNegativeInteger(retry, "providerMaxRetries"),
			providerMaxRetryDelayMilliseconds: requiredPositiveInteger(retry, "providerMaxRetryDelayMilliseconds"),
		},
		compaction: {
			mode: requiredOneOf(compaction, "mode", ["enabled", "disabled"] as const),
			reserveTokens: requiredNonNegativeInteger(compaction, "reserveTokens"),
			keepRecentTokens: requiredNonNegativeInteger(compaction, "keepRecentTokens"),
		},
		steeringMode: requiredOneOf(value, "steeringMode", ["all", "one-at-a-time"] as const),
		followUpMode: requiredOneOf(value, "followUpMode", ["all", "one-at-a-time"] as const),
		transport: requiredLiteral(value, "transport", "sse"),
		projectTrust: requiredLiteral(value, "projectTrust", "never"),
		installTelemetryEnabled: requiredLiteral(value, "installTelemetryEnabled", false),
		analyticsEnabled: requiredLiteral(value, "analyticsEnabled", false),
		images: requiredLiteral(value, "images", "blocked"),
	};
}

function decodePromptPayload(value: Record<string, unknown>): PromptPayload {
	requireExactKeys(value, ["purpose", "text"]);
	return {
		purpose: requiredOneOf(value, "purpose", ["TaskAssignment", "OfficeTurn"] as const),
		text: requiredNonEmptyString(value, "text"),
	};
}

function decodeFollowUpPayload(value: Record<string, unknown>): FollowUpPayload {
	requireExactKeys(value, ["noticeDeliveryIdentity", "ledgerFrontier", "text"]);
	return {
		noticeDeliveryIdentity: correlationIdentity(requiredString(value, "noticeDeliveryIdentity")),
		ledgerFrontier: ledgerFrontier(requiredSafeInteger(value, "ledgerFrontier")),
		text: requiredNonEmptyString(value, "text"),
	};
}

function decodeSteerPayload(value: Record<string, unknown>): SteerPayload {
	requireExactKeys(value, ["reason", "text"]);
	return {
		reason: requiredOneOf(value, "reason", ["UrgentStalePremise", "UrgentUnsafePremise"] as const),
		text: requiredNonEmptyString(value, "text"),
	};
}

function decodeAbortPayload(value: Record<string, unknown>): AbortPayload {
	requireExactKeys(value, ["reason"]);
	return {
		reason: requiredOneOf(value, "reason", [
			"GracefulCancellation",
			"EmergencyStop",
			"BudgetGuardrail",
			"DaemonRecovery",
		] as const),
	};
}

function decodeDisposePayload(value: Record<string, unknown>): DisposePayload {
	requireExactKeys(value, ["reason"]);
	return {
		reason: requiredOneOf(value, "reason", ["CycleReconciliation", "ProcessRecovery", "ProtocolFailure"] as const),
	};
}

function requiredRecord(value: unknown): Record<string, unknown> {
	if (typeof value !== "object" || value === null || Array.isArray(value)) throw new ProtocolDecodeError("invalid_frame");
	return value as Record<string, unknown>;
}

function requireExactKeys(value: Record<string, unknown>, expected: readonly string[]): void {
	const actual = Object.keys(value);
	if (actual.length !== expected.length || actual.some((key) => !expected.includes(key))) {
		throw new ProtocolDecodeError("invalid_frame");
	}
}

function requiredString(value: Record<string, unknown>, key: string): string {
	const candidate = value[key];
	if (typeof candidate !== "string") throw new ProtocolDecodeError("invalid_frame");
	return candidate;
}

function requiredNonEmptyString(value: Record<string, unknown>, key: string): string {
	const candidate = requiredString(value, key);
	if (candidate.length === 0) throw new ProtocolDecodeError("invalid_frame");
	return candidate;
}

function requiredSafeInteger(value: Record<string, unknown>, key: string): number {
	const candidate = value[key];
	if (typeof candidate !== "number" || !Number.isSafeInteger(candidate)) throw new ProtocolDecodeError("invalid_frame");
	return candidate;
}

function requiredNonNegativeInteger(value: Record<string, unknown>, key: string): NonNegativeInteger {
	return nonNegativeInteger(requiredSafeInteger(value, key));
}

function requiredPositiveInteger(value: Record<string, unknown>, key: string): PositiveInteger {
	return positiveInteger(requiredSafeInteger(value, key));
}

function requiredAbsolutePath(value: Record<string, unknown>, key: string): AbsolutePath {
	return absolutePath(requiredNonEmptyString(value, key));
}

function requiredLiteral<const T extends string | boolean>(value: Record<string, unknown>, key: string, expected: T): T {
	if (value[key] !== expected) throw new ProtocolDecodeError("invalid_frame");
	return expected;
}

function requiredOneOf<const T extends readonly string[]>(value: Record<string, unknown>, key: string, allowed: T): T[number] {
	const candidate = requiredString(value, key);
	if (!allowed.includes(candidate)) throw new ProtocolDecodeError("invalid_frame");
	return candidate as T[number];
}
