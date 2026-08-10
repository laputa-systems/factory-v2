import type { AgentSessionEvent } from "@earendil-works/pi-coding-agent";

import { blake3Hex } from "../src/digest.js";
import { FORUM_F0_AWARENESS_BLAKE3, FORUM_F0_TOOL_CONTRACT_BLAKE3 } from "../src/forum.js";
import {
	absolutePath,
	decodeInboundJsonl,
	nonNegativeInteger,
	providerCostObservation,
	blake3Digest,
	type CreateSessionPayload,
	type InboundFrame,
	type ModelCatalogPolicyV1,
	type RuntimeIdentity,
	type SessionIdentity,
	type TranscriptFlushReceiptV1,
	type UsageTotals,
} from "../src/protocol.js";
import type { SdkEventListener, SdkRuntime, SdkSession } from "../src/sdk.js";

export const TEST_SESSION_IDENTITY = "pi-session-test-001";

export const TEST_RUNTIME_EVIDENCE = {
	nodeExecutableBlake3: blake3Digest("1".repeat(64)),
	lockfileBlake3: blake3Digest("2".repeat(64)),
	adapterBuildBlake3: blake3Digest("3".repeat(64)),
	piTransitivePackageSetBlake3: blake3Digest("4".repeat(64)),
} satisfies Pick<RuntimeIdentity, "nodeExecutableBlake3" | "lockfileBlake3" | "adapterBuildBlake3" | "piTransitivePackageSetBlake3">;

export function decodeCommand(sequence: number, command: string, payload: Record<string, unknown>): InboundFrame {
	return decodeInboundJsonl(
		JSON.stringify({
			protocolVersion: "society-pi-host/v4",
			sequence,
			sessionIdentity: TEST_SESSION_IDENTITY,
			correlationIdentity: `command-${sequence}`,
			command,
			payload,
		}),
	);
}

export function createSessionPayload(sessionKind: "TaskAttempt" | "RootAuthorityOffice" = "RootAuthorityOffice"): Record<string, unknown> {
	const systemPrompt = "Founding Mission\nOffice contract";
	return {
		sessionKind,
		cwd: "/tmp/society-host-fixture/work",
		agentDirectory: "/tmp/society-host-fixture/agent",
		authPath: "/tmp/society-host-fixture/agent/auth.json",
		modelsPath: "/tmp/society-host-fixture/agent/models.json",
		sessionDirectory: "/tmp/society-host-fixture/session",
		systemPrompt,
		systemPromptDigest: blake3Hex(systemPrompt),
		model: {
			provider: "openrouter",
			modelId: "deepseek/deepseek-v4-flash-0731",
			thinkingLevel: "high",
		},
		modelCatalog: {
			catalogBlake3: "6".repeat(64),
			effectiveModel: {
				provider: "openrouter",
				baseUrl: "https://openrouter.ai/api/v1",
				api: "openai-completions",
				modelId: "deepseek/deepseek-v4-flash-0731",
				canonicalSlug: "deepseek/deepseek-v4-flash-20260731",
				input: "text_only",
				contextWindow: 1_048_576,
				maxTokens: 384_000,
				inputUsdPerMillion: { kind: "Known", usdPerMillion: "0.09" },
				outputUsdPerMillion: { kind: "Known", usdPerMillion: "0.18" },
				cacheReadUsdPerMillion: { kind: "Known", usdPerMillion: "0.018" },
				cacheWriteUsdPerMillion: { kind: "Absent" },
			},
		},
		toolProfile: "read_execute_v1",
		forumContract: {
			kind: "forum_enabled_v1",
			awarenessBlake3: FORUM_F0_AWARENESS_BLAKE3,
			toolContractBlake3: FORUM_F0_TOOL_CONTRACT_BLAKE3,
		},
		settings: {
			retry: {
				maxRetries: 2,
				baseDelayMilliseconds: 2_000,
				providerTimeoutMilliseconds: 300_000,
				providerMaxRetries: 1,
				providerMaxRetryDelayMilliseconds: 30_000,
			},
			compaction: {
				mode: "enabled",
				reserveTokens: 16_384,
				keepRecentTokens: 20_000,
			},
			steeringMode: "one-at-a-time",
			followUpMode: "one-at-a-time",
			transport: "sse",
			projectTrust: "never",
			installTelemetryEnabled: false,
			analyticsEnabled: false,
			images: "blocked",
		},
	};
}

export function decodedCreatePayload(sessionKind: "TaskAttempt" | "RootAuthorityOffice" = "RootAuthorityOffice"): CreateSessionPayload {
	const command = decodeCommand(1, "CreateSession", createSessionPayload(sessionKind));
	if (command.command !== "CreateSession") throw new Error("expected_create_session_command");
	return command.payload;
}

export class FakeSdkSession implements SdkSession {
	readonly sessionFile = absolutePath("/tmp/society-host-fixture/session/fixture.jsonl");
	readonly calls: string[] = [];
	private readonly listeners = new Set<SdkEventListener>();
	private promptResolver: (() => void) | undefined;
	private promptRejecter: ((error: Error) => void) | undefined;
	private idle = true;
	private promptStartDelayed = false;
	private transcriptVerificationDeferred = false;
	private transcriptVerificationResolver: (() => void) | undefined;
	private usageInvalid = false;
	private usage: UsageTotals = {
		inputTokens: nonNegativeInteger(11),
		outputTokens: nonNegativeInteger(7),
		cacheReadTokens: nonNegativeInteger(3),
		cacheWriteTokens: nonNegativeInteger(2),
		totalTokens: nonNegativeInteger(23),
		providerCost: providerCostObservation(0.0123456),
	};
	private firstPrompt: string | undefined;
	verifyCount = 0;
	disposed = false;

	constructor(
		readonly sessionIdentity: SessionIdentity,
		readonly modelCatalog: ModelCatalogPolicyV1,
	) {}

	get isIdle(): boolean {
		return this.idle;
	}

	prompt(text: string): Promise<void> {
		this.calls.push(`Prompt:${text}`);
		if (this.firstPrompt === undefined) this.firstPrompt = text;
		this.idle = false;
		if (!this.promptStartDelayed) this.emit({ type: "agent_start" });
		return new Promise<void>((resolve, reject) => {
			this.promptResolver = resolve;
			this.promptRejecter = reject;
		});
	}

	async followUp(text: string): Promise<void> {
		this.calls.push(`FollowUp:${text}`);
	}

	async steer(text: string): Promise<void> {
		this.calls.push(`Steer:${text}`);
	}

	async abort(): Promise<void> {
		this.calls.push("Abort");
	}

	dispose(): void {
		this.calls.push("Dispose");
		this.disposed = true;
	}

	subscribe(listener: SdkEventListener): () => void {
		this.listeners.add(listener);
		return () => this.listeners.delete(listener);
	}

	usageTotals(): UsageTotals {
		if (this.usageInvalid) throw new Error("invalid fixture usage");
		return this.usage;
	}

	async verifyCanonicalTranscript(): Promise<TranscriptFlushReceiptV1> {
		this.calls.push("VerifyCanonicalTranscript");
		this.verifyCount += 1;
		const receipt: TranscriptFlushReceiptV1 = this.firstPrompt === undefined
			? {
				format: "pi_session_manager_jsonl_v3",
				sessionIdentity: this.sessionIdentity,
				sessionFile: this.sessionFile,
				materialization: "unmaterialized_no_prompt",
				firstUserPrompt: { kind: "absent" },
			}
			: {
				format: "pi_session_manager_jsonl_v3",
				sessionIdentity: this.sessionIdentity,
				sessionFile: this.sessionFile,
				materialization: "observed",
				sessionFileBlake3: blake3Digest("5".repeat(64)),
				headerCwd: absolutePath("/tmp/society-host-fixture/work"),
				firstUserPrompt: { kind: "verified", digest: blake3Digest(blake3Hex(this.firstPrompt)) },
			};
		if (this.transcriptVerificationDeferred) {
			await new Promise<void>((resolve) => { this.transcriptVerificationResolver = resolve; });
			this.transcriptVerificationResolver = undefined;
		}
		return receipt;
	}

	deferCanonicalTranscriptVerification(): void {
		this.transcriptVerificationDeferred = true;
	}

	releaseCanonicalTranscriptVerification(): void {
		this.transcriptVerificationDeferred = false;
		this.transcriptVerificationResolver?.();
	}
	finishPrompt(stopReason: "stop" | "length" | "error" | "aborted" = "stop"): void {
		this.idle = true;
		this.emit({ type: "agent_end", messages: [this.assistantMessage(stopReason)], willRetry: false });
		this.emit({ type: "agent_settled" });
		this.promptResolver?.();
		this.promptResolver = undefined;
		this.promptRejecter = undefined;
	}

	finishPromptAfterRetriedError(): void {
		this.idle = true;
		this.emit({ type: "agent_end", messages: [this.assistantMessage("error")], willRetry: true });
		this.emit({ type: "agent_end", messages: [this.assistantMessage("stop")], willRetry: false });
		this.emit({ type: "agent_settled" });
		this.promptResolver?.();
		this.promptResolver = undefined;
		this.promptRejecter = undefined;
	}

	resolvePromptWithoutTerminalEvidence(): void {
		this.idle = true;
		this.promptResolver?.();
		this.promptResolver = undefined;
		this.promptRejecter = undefined;
	}

	setPromptStartDelayed(): void {
		this.promptStartDelayed = true;
	}

	startPrompt(): void {
		this.emit({ type: "agent_start" });
	}

	makeUsageInvalid(): void {
		this.usageInvalid = true;
	}

	setUsage(totals: UsageTotals): void {
		this.usage = totals;
	}

	failPrompt(): void {
		this.idle = true;
		this.promptRejecter?.(new Error("fixture_failure"));
		this.promptResolver = undefined;
		this.promptRejecter = undefined;
	}

	emitForensicOnlyUpdate(): void {
		this.emit({ type: "bash_execution_update", id: "execution-1", delta: "expensive raw output" });
	}

	emitThinkingLevelMutation(): void {
		this.emit({ type: "thinking_level_changed", level: "low" });
	}

	emitPersistedModelMutation(): void {
		this.emit({
			type: "entry_appended",
			entry: {
				type: "model_change",
				id: "model-change-001",
				parentId: null,
				timestamp: "2026-01-01T00:00:00.000Z",
				provider: "openrouter",
				modelId: "different-model",
			},
		});
	}

	emitAssistantResponseModelDrift(): void {
		this.emitAssistantMessageEnd("other-response-model");
	}

	emitAssistantMessageEnd(responseModel: string | undefined = undefined): void {
		this.emit({
			type: "message_end",
			message: this.assistantMessage("stop", responseModel),
		});
	}

	private assistantMessage(
		stopReason: "stop" | "length" | "error" | "aborted",
		responseModel: string | undefined = undefined,
	): Extract<AgentSessionEvent, { type: "message_end" }> ["message"] {
		return {
			role: "assistant", content: [], api: "openai-completions", provider: "openrouter", model: "deepseek/deepseek-v4-flash-0731",
			...(responseModel === undefined ? {} : { responseModel }),
			usage: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0, cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 } },
			stopReason, ...(stopReason === "error" ? { errorMessage: "fixture model error" } : {}), timestamp: 1,
		};
	}

	private emit(event: AgentSessionEvent): void {
		for (const listener of this.listeners) listener(event);
	}
}

export class FakeSdkRuntime implements SdkRuntime {
	createCount = 0;
	session: FakeSdkSession | undefined;

	async create(sessionIdentity: SessionIdentity, payload: CreateSessionPayload): Promise<SdkSession> {
		this.createCount += 1;
		this.session = new FakeSdkSession(sessionIdentity, payload.modelCatalog);
		return this.session;
	}
}

export class DeferredSdkRuntime implements SdkRuntime {
	createCount = 0;
	session: FakeSdkSession | undefined;
	private resolveCreate: ((session: SdkSession) => void) | undefined;

	create(sessionIdentity: SessionIdentity, payload: CreateSessionPayload): Promise<SdkSession> {
		this.createCount += 1;
		this.session = new FakeSdkSession(sessionIdentity, payload.modelCatalog);
		return new Promise<SdkSession>((resolve) => {
			this.resolveCreate = resolve;
		});
	}

	completeCreate(): void {
		const session = this.session;
		if (session === undefined || this.resolveCreate === undefined) throw new Error("deferred_runtime_not_creating");
		this.resolveCreate(session);
		this.resolveCreate = undefined;
	}
}

export async function drainMicrotasks(): Promise<void> {
	await new Promise<void>((resolve) => setImmediate(resolve));
}
