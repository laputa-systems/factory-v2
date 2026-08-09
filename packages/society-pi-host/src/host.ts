/**
 * Stateful, one-session adapter. It owns neither authority nor a durable
 * workflow: Rust supplies the session identity, records the streams, and
 * decides whether each command is admissible. The host only enforces its
 * local protocol/lifecycle before calling the pinned SDK.
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { AgentSessionEvent } from "@earendil-works/pi-coding-agent";

import { isExecutionProfileMutation, projectAgentSessionEvent } from "./event-projection.js";
import {
	ADAPTER_PROTOCOL_VERSION,
	ADAPTER_VERSION,
	PINNED_PI_SDK_VERSION,
	MAX_JSONL_FRAME_BYTES,
	boundarySequence,
	nodeRuntimeVersion,
	sha256Digest,
	type AdapterFailureCode,
	type AdapterPhase,
	type AdapterReadyFrame,
	type CommandResultDetail,
	type CreateSessionCommand,
	type EffectiveSessionConfiguration,
	type FinalAssistantOutcome,
	type InboundFrame,
	type OutboundFrame,
	type RuntimeIdentity,
	type SessionIdentity,
	type SpawnNonce,
	type UsageTotals,
	toolsForProfile,
} from "./protocol.js";
import { SdkConstructionError, assertSdkEventExecutionProfile, type SdkRuntime, type SdkSession } from "./sdk.js";

export interface HostIdentity {
	readonly sessionIdentity: SessionIdentity;
	readonly spawnNonce: SpawnNonce;
	readonly pid: number;
	readonly runtime: RuntimeIdentity;
}

/** Digests are supplied by the Rust execution-profile admission, not guessed by Node. */
export interface SupervisorRuntimeEvidence {
	readonly nodeExecutableSha256: RuntimeIdentity["nodeExecutableSha256"];
	readonly lockfileSha256: RuntimeIdentity["lockfileSha256"];
	readonly adapterBuildSha256: RuntimeIdentity["adapterBuildSha256"];
	readonly piTransitivePackageSetSha256: RuntimeIdentity["piTransitivePackageSetSha256"];
}

export type FrameSink = (frame: OutboundFrame) => void;
type OutboundFrameDraft = WithoutEnvelope<OutboundFrame>;
type WithoutEnvelope<T> = T extends unknown ? Omit<T, "protocolVersion" | "sequence" | "sessionIdentity"> : never;

interface ActiveTurn {
	readonly correlationIdentity: InboundFrame["correlationIdentity"];
	readonly promptPurpose: "TaskAssignment" | "OfficeTurn";
	observedAgentSettled: boolean;
	observedAgentStarted: boolean;
	abortRequested: boolean;
	/** Only the `agent_end` which will not retry may determine final success. */
	finalAssistantOutcome: Extract<FinalAssistantOutcome, { kind: "Observed" }> | undefined;
}

/**
 * The host reports `AdapterReady` from its constructor and remains inert until
 * `CreateSession`. A control-pipe EOF in that state cannot create a Pi session.
 */
export class PiSdkHost {
	private phase: AdapterPhase = "Inert";
	private expectedInboundSequence = 1;
	private nextOutboundSequence = 1;
	private session: SdkSession | undefined;
	private unsubscribe: (() => void) | undefined;
	private activeTurn: ActiveTurn | undefined;
	private commandTail: Promise<void> = Promise.resolve();
	private taskAttemptPromptAdmitted = false;
	private lastUsage: UsageTotals | undefined;
	private sdkDisposeInvoked = false;
	private emittingTerminalFrame = false;
	private outboundTransportClosed = false;

	constructor(
		private readonly identity: HostIdentity,
		private readonly runtime: SdkRuntime,
		private readonly sink: FrameSink,
	) {
		validateHostIdentity(identity);
		this.emit(this.adapterReadyFrame());
	}

	get currentPhase(): AdapterPhase {
		return this.phase;
	}

	accept(command: InboundFrame): Promise<void> {
		const scheduled = this.commandTail.then(() => this.acceptOne(command));
		// Prompt completion intentionally runs outside this lane so that a later
		// FollowUp, Steer, or Abort can reach the active SDK turn. All command
		// admissions themselves remain FIFO and sequence-checked.
		this.commandTail = scheduled.catch(() => {});
		return scheduled;
	}

	private async acceptOne(command: InboundFrame): Promise<void> {
		if (this.phase === "Fatal") return;
		if (!this.matchesExpectedCommand(command)) return;
		if (command.sequence !== this.expectedInboundSequence) {
			this.reject(command, "sequence_gap");
			this.fatal("sequence_gap");
			return;
		}
		this.expectedInboundSequence += 1;
		try {
			switch (command.command) {
				case "CreateSession":
					await this.createSession(command);
					return;
				case "Prompt":
					await this.prompt(command);
					return;
				case "FollowUp":
					await this.followUp(command);
					return;
				case "Steer":
					await this.steer(command);
					return;
				case "Abort":
					await this.abort(command);
					return;
				case "GetState":
					this.getState(command);
					return;
				case "Dispose":
					await this.dispose(command);
					return;
				default:
					return assertNever(command);
			}
		} catch (error) {
			this.reject(command, errorToFailureCode(error));
			if (command.command === "CreateSession" || command.command === "Dispose") {
				this.fatal(errorToFailureCode(error));
			}
		}
	}

	/** Invoked by the stdio runner when Rust closes the control pipe. */
	async onControlPipeEof(): Promise<void> {
		// EOF can immediately follow a final Dispose frame. Waiting for admitted
		// work prevents a control-pipe race from replacing its flush receipt with
		// a false failure.
		await this.commandTail;
		if (this.phase === "Inert" || this.phase === "Disposed" || this.phase === "Fatal") return;
		this.fatal("sdk_operation_failed");
	}

	/** Invalid JSON and unknown protocol revisions cannot fall through as text. */
	protocolDecodeFailed(): void {
		this.fatal("protocol_decode_failed");
	}

	/**
	 * The stdout JSONL pipe is the only observation route. Once it cannot accept
	 * a frame, continuing to run Pi would create an unobserved paid execution.
	 * This hook therefore fences all subsequent inbound commands and best-effort
	 * disposes the owned SDK session without trying to recursively write Fatal.
	 */
	outboundTransportFailed(): void {
		if (this.outboundTransportClosed) return;
		this.outboundTransportClosed = true;
		this.fatal("sdk_operation_failed");
	}

	private matchesExpectedCommand(command: InboundFrame): boolean {
		if (command.sessionIdentity === this.identity.sessionIdentity) return true;
		this.reject(command, "session_identity_mismatch");
		this.fatal("session_identity_mismatch");
		return false;
	}

	private async createSession(command: CreateSessionCommand): Promise<void> {
		if (this.phase !== "Inert") return this.reject(command, "invalid_state");
		this.phase = "Creating";
		const session = await this.runtime.create(this.identity.sessionIdentity, command.payload);
		if (this.phase !== "Creating") {
			session.dispose();
			return;
		}
		this.session = session;
		this.unsubscribe = session.subscribe((event) => this.onSdkEvent(event));
		this.phase = "Ready";
		this.accepted(command);
		this.emit({
			event: "SessionReady",
			correlationIdentity: command.correlationIdentity,
			configuration: this.effectiveConfiguration(command),
		});
	}

	private async prompt(command: Extract<InboundFrame, { command: "Prompt" }>): Promise<void> {
		const session = this.requireIdleSession(command);
		if (!session) return;
		if (!this.isPromptPurposeLegal(command)) return this.reject(command, "invalid_command");
		if (this.effectiveSessionKind() === "TaskAttempt" && this.taskAttemptPromptAdmitted) return this.reject(command, "invalid_state");
		if (this.effectiveSessionKind() === "TaskAttempt") this.taskAttemptPromptAdmitted = true;
		this.activeTurn = {
			correlationIdentity: command.correlationIdentity,
			promptPurpose: command.payload.purpose,
			observedAgentSettled: false,
			observedAgentStarted: false,
			abortRequested: false,
			finalAssistantOutcome: undefined,
		};
		this.accepted(command);
		void this.completePrompt(command, session);
	}

	private async completePrompt(command: Extract<InboundFrame, { command: "Prompt" }>, session: SdkSession): Promise<void> {
		try {
			await session.prompt(command.payload.text);
			await session.verifyCanonicalTranscript();
			if (this.phase === "Fatal") return;
			const activeTurn = this.activeTurn;
			if (!activeTurn?.observedAgentSettled) {
				this.terminalEvidenceFailure(command.correlationIdentity, "missing_agent_settled");
				return;
			}
			if (activeTurn.finalAssistantOutcome === undefined) {
				this.terminalEvidenceFailure(command.correlationIdentity, "missing_final_assistant_outcome");
				return;
			}
			if (!this.emitUsage(command.correlationIdentity, session)) return;
			this.emit({
				event: "Settled",
				correlationIdentity: command.correlationIdentity,
				classification: settledClassification(activeTurn),
				finalAssistantOutcome: activeTurn.finalAssistantOutcome,
			});
			this.activeTurn = undefined;
		} catch (error) {
			if (this.phase === "Fatal") return;
			// Prompt admission was accepted before the model call. Its outcome is a
			// terminal classification, not a contradictory second CommandResult.
			this.settleCommandFailure(command.correlationIdentity, errorToFailureCode(error));
		}
	}

	private async followUp(command: Extract<InboundFrame, { command: "FollowUp" }>): Promise<void> {
		const session = this.requireReadySession(command);
		if (!session) return;
		if (this.effectiveSessionKind() !== "GrandArchitectOffice") return this.reject(command, "invalid_command");
		if (this.activeTurn === undefined || !this.activeTurn.observedAgentStarted) return this.reject(command, "invalid_state");
		await session.followUp(command.payload.text);
		this.accepted(command);
	}

	private async steer(command: Extract<InboundFrame, { command: "Steer" }>): Promise<void> {
		const session = this.requireReadySession(command);
		if (!session) return;
		if (this.effectiveSessionKind() !== "GrandArchitectOffice") return this.reject(command, "invalid_command");
		if (this.activeTurn === undefined || !this.activeTurn.observedAgentStarted) return this.reject(command, "invalid_state");
		await session.steer(command.payload.text);
		this.accepted(command);
	}

	private async abort(command: Extract<InboundFrame, { command: "Abort" }>): Promise<void> {
		const session = this.requireReadySession(command);
		if (!session) return;
		if (this.activeTurn !== undefined) this.activeTurn.abortRequested = true;
		await session.abort();
		this.accepted(command);
		this.emitUsage(command.correlationIdentity, session);
	}

	private getState(command: Extract<InboundFrame, { command: "GetState" }>): void {
		const session = this.requireReadySession(command);
		if (!session) return;
		this.accepted(command, {
			kind: "state",
			phase: this.phase,
			liveness: this.activeTurn === undefined && session.isIdle ? "idle" : "active",
			sessionFile: session.sessionFile,
		});
		this.emitUsage(command.correlationIdentity, session);
	}

	private async dispose(command: Extract<InboundFrame, { command: "Dispose" }>): Promise<void> {
		const session = this.requireIdleSession(command);
		if (!session) return;
		this.phase = "Closing";
		// Pi's `dispose()` is synchronous. Observe its canonical transcript after
		// it returns before claiming `Disposed` or allowing the process to exit.
		this.disposeSdkSession();
		const transcriptFlushReceipt = await session.verifyCanonicalTranscript();
		this.unsubscribe?.();
		this.unsubscribe = undefined;
		this.phase = "Disposed";
		this.accepted(command);
		this.emitUsage(command.correlationIdentity, session);
		this.emit({ event: "Disposed", correlationIdentity: command.correlationIdentity, transcriptFlushReceipt });
	}

	private onSdkEvent(event: AgentSessionEvent): void {
		if (this.phase === "Fatal") return;
		try {
			assertSdkEventExecutionProfile(event);
			const projected = projectAgentSessionEvent(event);
			if (projected.type === "agent_settled" && this.activeTurn !== undefined) {
				this.activeTurn.observedAgentSettled = true;
			}
			if (event.type === "agent_end" && !event.willRetry && this.activeTurn !== undefined) {
				this.activeTurn.finalAssistantOutcome = finalAssistantOutcome(event.messages);
			}
			if (projected.type === "agent_start" && this.activeTurn !== undefined) {
				this.activeTurn.observedAgentStarted = true;
			}
			this.emit({
				event: "AgentEvent",
				...(this.activeTurn === undefined ? {} : { correlationIdentity: this.activeTurn.correlationIdentity }),
				agentEvent: projected,
			});
			if (isExecutionProfileMutation(event)) {
				this.fatal("execution_profile_drift");
				return;
			}
			if (isMeaningfulUsageEvent(event)) this.emitUsageIfChanged(this.activeTurn?.correlationIdentity, this.session);
		} catch (error) {
			this.fatal(errorToFailureCode(error));
		}
	}

	private requireReadySession(command: InboundFrame): SdkSession | undefined {
		if (this.phase !== "Ready" || this.session === undefined) {
			this.reject(command, "invalid_state");
			return undefined;
		}
		return this.session;
	}

	private requireIdleSession(command: InboundFrame): SdkSession | undefined {
		const session = this.requireReadySession(command);
		if (session === undefined) return undefined;
		if (!session.isIdle || this.activeTurn !== undefined) {
			this.reject(command, "invalid_state");
			return undefined;
		}
		return session;
	}

	private isPromptPurposeLegal(command: Extract<InboundFrame, { command: "Prompt" }>): boolean {
		const kind = this.effectiveSessionKind();
		return (kind === "TaskAttempt" && command.payload.purpose === "TaskAssignment") ||
			(kind === "GrandArchitectOffice" && command.payload.purpose === "OfficeTurn");
	}

	private effectiveSessionKind(): "TaskAttempt" | "GrandArchitectOffice" {
		const configuration = this.createPayload;
		if (configuration === undefined) throw new Error("missing_create_session_payload");
		return configuration.sessionKind;
	}

	private createPayload: CreateSessionCommand["payload"] | undefined;

	private effectiveConfiguration(command: CreateSessionCommand): EffectiveSessionConfiguration {
		this.createPayload = command.payload;
		const session = this.session;
		if (session === undefined) throw new Error("missing_sdk_session");
		return {
			sessionKind: command.payload.sessionKind,
			cwd: command.payload.cwd,
			sessionDirectory: command.payload.sessionDirectory,
			sessionFile: session.sessionFile,
			model: command.payload.model,
			modelCatalog: session.modelCatalog,
			toolProfile: command.payload.toolProfile,
			tools: toolsForProfile(command.payload.toolProfile),
			settings: command.payload.settings,
		};
	}

	private adapterReadyFrame(): Omit<AdapterReadyFrame, "protocolVersion" | "sequence" | "sessionIdentity"> {
		return {
			event: "AdapterReady",
			pid: this.identity.pid,
			spawnNonce: this.identity.spawnNonce,
			runtime: this.identity.runtime,
		};
	}

	private accepted(command: InboundFrame, detail: Extract<CommandResultDetail, { kind: "acknowledged" | "state" }> = { kind: "acknowledged" }): void {
		this.emit({
			event: "CommandResult",
			correlationIdentity: command.correlationIdentity,
			command: command.command,
			accepted: true,
			detail,
		});
	}

	private reject(command: InboundFrame, failureCode: AdapterFailureCode): void {
		this.emit({
			event: "CommandResult",
			correlationIdentity: command.correlationIdentity,
			command: command.command,
			accepted: false,
			detail: { kind: "rejected" },
			failureCode,
		});
	}

	private settleCommandFailure(correlationIdentity: InboundFrame["correlationIdentity"], failureCode: AdapterFailureCode): void {
		if (this.phase === "Fatal") return;
		if (this.session === undefined || !this.emitUsage(correlationIdentity, this.session)) return;
		this.emit({
			event: "Settled",
			correlationIdentity,
			classification: failureCode === "sdk_operation_failed" ? "failed" : "protocol_failed",
			finalAssistantOutcome: {
				kind: "Unavailable",
				reason: failureCode === "sdk_operation_failed" ? "sdk_promise_rejected" : "missing_final_assistant_outcome",
			},
		});
		this.activeTurn = undefined;
	}

	/** Missing final Pi evidence invalidates the execution profile, not just this turn. */
	private terminalEvidenceFailure(correlationIdentity: InboundFrame["correlationIdentity"], failureCode: "missing_agent_settled" | "missing_final_assistant_outcome"): void {
		this.settleCommandFailure(correlationIdentity, failureCode);
		this.fatal(failureCode);
	}

	private emitUsage(correlationIdentity: InboundFrame["correlationIdentity"], session: SdkSession): boolean {
		return this.emitUsageObservation(correlationIdentity, session, true);
	}

	private emitUsageIfChanged(correlationIdentity: InboundFrame["correlationIdentity"] | undefined, session: SdkSession | undefined): void {
		if (session !== undefined) this.emitUsageObservation(correlationIdentity, session, false);
	}

	private emitUsageObservation(
		correlationIdentity: InboundFrame["correlationIdentity"] | undefined,
		session: SdkSession,
		force: boolean,
	): boolean {
		try {
			const totals = session.usageTotals();
			assertUsageSnapshotAdmissible(this.lastUsage, totals);
			if (!force && this.lastUsage !== undefined && areUsageTotalsEqual(this.lastUsage, totals)) return true;
			this.lastUsage = totals;
			this.emit({ event: "UsageSnapshot", ...(correlationIdentity === undefined ? {} : { correlationIdentity }), usage: { kind: "Known", totals } });
			return true;
		} catch (error) {
			const reason = error instanceof UsageInvariantError ? error.reason : "invalid_sdk_usage";
			this.emit({
				event: "UsageSnapshot",
				...(correlationIdentity === undefined ? {} : { correlationIdentity }),
				usage: { kind: "Unavailable", reason },
			});
			this.fatal(reason === "invalid_sdk_usage" ? "sdk_operation_failed" : "execution_profile_drift");
			return false;
		}
	}

	private fatal(failureCode: AdapterFailureCode): void {
		if (this.phase === "Fatal") return;
		this.phase = "Fatal";
		this.activeTurn = undefined;
		try {
			this.unsubscribe?.();
			this.disposeSdkSession();
		} catch {
			// Fatal is terminal; its code, rather than a second uncontrolled error,
			// is the information Rust needs to contain the process group.
		}
		this.emit({ event: "Fatal", failureCode });
	}

	private disposeSdkSession(): void {
		if (this.sdkDisposeInvoked) return;
		this.sdkDisposeInvoked = true;
		this.session?.dispose();
	}

	private emit(frame: OutboundFrameDraft): void {
		if (this.outboundTransportClosed) return;
		const sealed = {
			...frame,
			protocolVersion: ADAPTER_PROTOCOL_VERSION,
			sequence: boundarySequence(this.nextOutboundSequence++),
			sessionIdentity: this.identity.sessionIdentity,
		} as OutboundFrame;
		try {
			const json = JSON.stringify(sealed);
			if (json === undefined || Buffer.byteLength(json, "utf8") + 1 > MAX_JSONL_FRAME_BYTES) {
				throw new Error("outbound_frame_too_large");
			}
			this.sink(sealed);
		} catch {
			// A malformed/oversize evidence frame must never be silently truncated.
			// The small typed Fatal frame remains emit-capable unless stdout itself has
			// failed, in which case main's error handler contains the process.
			if (frame.event !== "Fatal" && !this.emittingTerminalFrame) {
				this.emittingTerminalFrame = true;
				try {
					this.fatal("outbound_frame_too_large");
				} finally {
					this.emittingTerminalFrame = false;
				}
			}
		}
	}
}

class UsageInvariantError extends Error {
	constructor(readonly reason: "usage_regressed" | "usage_inconsistent") {
		super(reason);
	}
}

/**
 * Pi exposes cumulative counters. Regressing counters or a cost bit-pattern
 * is execution-profile drift, never a second billable observation. `total`
 * follows Pi 0.83's documented sum-of-buckets semantics exactly.
 */
function assertUsageSnapshotAdmissible(previous: UsageTotals | undefined, current: UsageTotals): void {
	const summed = current.inputTokens + current.outputTokens + current.cacheReadTokens + current.cacheWriteTokens;
	if (!Number.isSafeInteger(summed) || summed !== current.totalTokens) throw new UsageInvariantError("usage_inconsistent");
	if (previous === undefined) return;
	if (
		current.inputTokens < previous.inputTokens || current.outputTokens < previous.outputTokens ||
		current.cacheReadTokens < previous.cacheReadTokens || current.cacheWriteTokens < previous.cacheWriteTokens ||
		current.totalTokens < previous.totalTokens ||
		binary64FromObservation(current) < binary64FromObservation(previous)
	) throw new UsageInvariantError("usage_regressed");
}

function binary64FromObservation(totals: UsageTotals): number {
	const encoded = totals.providerCost.binary64BigEndianHex;
	if (!/^[a-f0-9]{16}$/u.test(encoded)) throw new UsageInvariantError("usage_inconsistent");
	const buffer = new ArrayBuffer(8);
	const view = new DataView(buffer);
	view.setBigUint64(0, BigInt(`0x${encoded}`), false);
	const value = view.getFloat64(0, false);
	if (!Number.isFinite(value) || value < 0) throw new UsageInvariantError("usage_inconsistent");
	return value;
}

export function localRuntimeIdentity(nodeVersion: string, evidence: SupervisorRuntimeEvidence): RuntimeIdentity {
	return {
		nodeVersion: nodeRuntimeVersion(nodeVersion),
		adapterVersion: ADAPTER_VERSION,
		piSdkVersion: resolvedInstalledPiSdkVersion(),
		...evidence,
	};
}

/**
 * `AdapterReady` is a statement about the package Node actually resolved, not
 * merely the version this source was authored against. Package exports prevent
 * direct JSON import, so resolve the executable entry then inspect its sibling
 * package manifest before the host becomes observable.
 */
export function resolvedInstalledPiSdkVersion() {
	const packageEntry = fileURLToPath(import.meta.resolve("@earendil-works/pi-coding-agent"));
	const packageManifestPath = join(dirname(packageEntry), "..", "package.json");
	let parsed: unknown;
	try {
		parsed = JSON.parse(readFileSync(packageManifestPath, "utf8")) as unknown;
	} catch {
		throw new Error("installed_pi_sdk_manifest_unreadable");
	}
	if (
		typeof parsed !== "object" ||
		parsed === null ||
		Array.isArray(parsed) ||
		(parsed as { name?: unknown }).name !== "@earendil-works/pi-coding-agent" ||
		(parsed as { version?: unknown }).version !== PINNED_PI_SDK_VERSION
	) {
		throw new Error("installed_pi_sdk_version_drift");
	}
	return PINNED_PI_SDK_VERSION;
}

function errorToFailureCode(error: unknown): AdapterFailureCode {
	if (error instanceof SdkConstructionError) return error.code;
	return "sdk_operation_failed";
}

function validateHostIdentity(identity: HostIdentity): void {
	if (identity.pid < 1 || !Number.isSafeInteger(identity.pid)) throw new Error("invalid_host_pid");
	const installedPiSdkVersion = resolvedInstalledPiSdkVersion();
	if (identity.runtime.adapterVersion !== ADAPTER_VERSION || identity.runtime.piSdkVersion !== installedPiSdkVersion) {
		throw new Error("runtime_identity_drift");
	}
	nodeRuntimeVersion(identity.runtime.nodeVersion);
	sha256Digest(identity.runtime.nodeExecutableSha256);
	sha256Digest(identity.runtime.lockfileSha256);
	sha256Digest(identity.runtime.adapterBuildSha256);
	sha256Digest(identity.runtime.piTransitivePackageSetSha256);
}

function isMeaningfulUsageEvent(event: AgentSessionEvent): boolean {
	return event.type === "message_end" || event.type === "entry_appended" || event.type === "compaction_end" || event.type === "agent_end";
}

function areUsageTotalsEqual(left: UsageTotals, right: UsageTotals): boolean {
	return left.inputTokens === right.inputTokens &&
		left.outputTokens === right.outputTokens &&
		left.cacheReadTokens === right.cacheReadTokens &&
		left.cacheWriteTokens === right.cacheWriteTokens &&
		left.totalTokens === right.totalTokens &&
		left.providerCost.encoding === right.providerCost.encoding &&
		left.providerCost.binary64BigEndianHex === right.providerCost.binary64BigEndianHex &&
		left.providerCost.rounding === right.providerCost.rounding;
}

function finalAssistantOutcome(messages: readonly unknown[]): Extract<FinalAssistantOutcome, { kind: "Observed" }> | undefined {
	for (let index = messages.length - 1; index >= 0; index -= 1) {
		const message = messages[index];
		if (typeof message !== "object" || message === null || Array.isArray(message)) continue;
		const candidate = message as { role?: unknown; stopReason?: unknown };
		if (candidate.role !== "assistant") continue;
		switch (candidate.stopReason) {
			case "stop":
			case "length":
			case "error":
			case "aborted":
				return { kind: "Observed", stopReason: candidate.stopReason };
			// `pending` and `toolUse` are intermediate Pi reasons, while any new
			// SDK reason is a schema drift. Neither can prove a settled outcome.
			default:
				return undefined;
		}
	}
	return undefined;
}

function settledClassification(activeTurn: ActiveTurn): "completed" | "length" | "error" | "aborted" {
	const outcome = activeTurn.finalAssistantOutcome;
	if (outcome === undefined) throw new Error("missing_final_assistant_outcome");
	switch (outcome.stopReason) {
		case "stop": return activeTurn.abortRequested ? "aborted" : "completed";
		case "length": return "length";
		case "error": return "error";
		case "aborted": return "aborted";
		default: return assertNever(outcome.stopReason);
	}
}

function assertNever(value: never): never {
	throw new Error(`unhandled_adapter_command:${String(value)}`);
}
