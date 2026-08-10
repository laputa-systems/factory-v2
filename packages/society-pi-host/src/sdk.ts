/**
 * The production Pi construction path. It is deliberately one-way: the Rust
 * supervisor supplies every execution input, and this module neither discovers
 * project resources nor exposes a mutating Pi configuration surface.
 */

import { access, mkdir, readFile, readdir, realpath, stat, writeFile } from "node:fs/promises";
import { basename, dirname, isAbsolute, join, normalize, relative, resolve, sep } from "node:path";

import {
	createAgentSession,
	createExtensionRuntime,
	createEditToolDefinition,
	createLsToolDefinition,
	createReadToolDefinition,
	createWriteToolDefinition,
	ModelRuntime,
	SessionManager,
	SettingsManager,
	type AgentSession,
	type AgentSessionEvent,
	type EditOperations,
	type LsOperations,
	type ReadOperations,
	type ResourceLoader,
	type ToolDefinition,
	type WriteOperations,
} from "@earendil-works/pi-coding-agent";

import { blake3Hex } from "./digest.js";

import {
	PINNED_MODEL,
	PINNED_PROVIDER,
	PINNED_THINKING_LEVEL,
	absolutePath,
	assertPinnedActorModelPolicy,
	assertPinnedModelCatalogPolicy,
	assertPinnedForumSessionContract,
	binary64BigEndianHex,
	nonNegativeInteger,
	providerCostObservation,
	blake3Digest,
	type ActorModelPolicyV1,
	type AbsolutePath,
	type CreateSessionPayload,
	type EffectiveModelDescriptorV1,
	type ModelCatalogPolicyV1,
	type PiToolName,
	type SessionIdentity,
	type ToolProfile,
	type TranscriptFlushReceiptV1,
	type UsageTotals,
	toolsForProfile,
} from "./protocol.js";

export type SdkEventListener = (event: AgentSessionEvent) => void;

/** Narrow surface used by the protocol host and its provider-free doubles. */
export interface SdkSession {
	readonly sessionIdentity: SessionIdentity;
	readonly sessionFile: AbsolutePath;
	/** The ModelRuntime-observed billing treatment, not an ambient model default. */
	readonly modelCatalog: ModelCatalogPolicyV1;
	readonly isIdle: boolean;
	prompt(text: string): Promise<void>;
	followUp(text: string): Promise<void>;
	steer(text: string): Promise<void>;
	abort(): Promise<void>;
	dispose(): void;
	subscribe(listener: SdkEventListener): () => void;
	usageTotals(): UsageTotals;
	verifyCanonicalTranscript(): Promise<TranscriptFlushReceiptV1>;
}

export interface SdkRuntime {
	create(sessionIdentity: SessionIdentity, payload: CreateSessionPayload): Promise<SdkSession>;
}

/**
 * A construction failure is rendered as a closed protocol code. Its caught
 * error text is deliberately omitted from host diagnostics; structured Pi
 * session events remain sealed boundary evidence and are handled separately.
 */
export class SdkConstructionError extends Error {
	constructor(readonly code: "execution_profile_drift" | "sdk_operation_failed") {
		super(code);
		this.name = "SdkConstructionError";
	}
}

/** The pinned production runtime. It makes no provider call during construction. */
export class PinnedPiSdkRuntime implements SdkRuntime {
	/** Test-only race seam; production supplies no hook and has no ambient discovery. */
	constructor(private readonly constructionHooks: { readonly afterCatalogRead?: (modelsPath: AbsolutePath) => Promise<void> } = {}) {}

	async create(sessionIdentity: SessionIdentity, payload: CreateSessionPayload): Promise<SdkSession> {
		assertSystemPromptDigest(payload);
		assertExactCreatePayload(payload);

		try {
			const catalog = await bindCatalogFiles(payload);
			await this.constructionHooks.afterCatalogRead?.(catalog.modelsPath);
			const modelRuntime = await ModelRuntime.create({
				authPath: catalog.authPath,
				modelsPath: catalog.modelsPath,
				// Dynamic provider catalogs are not part of the admitted execution
				// profile. Keep them process-local instead of allowing Pi to read or
				// write a models-store.json beside the admitted catalog.
				modelsStore: createInMemoryModelsStore(),
				allowModelNetwork: false,
			});
			await assertCatalogUnchanged(catalog);
			const model = modelRuntime.getModel(PINNED_PROVIDER, PINNED_MODEL);
			if (!model || model.provider !== PINNED_PROVIDER || model.id !== PINNED_MODEL) {
				throw new SdkConstructionError("execution_profile_drift");
			}
			assertEffectiveBillingTreatment(model, payload.modelCatalog.effectiveModel);

			const settingsManager = SettingsManager.inMemory(toPiSettings(payload.settings));
			const resourceLoader = createInertResourceLoader(payload.systemPrompt);
			const sessionManager = SessionManager.create(payload.cwd, payload.sessionDirectory, { id: sessionIdentity });
			const result = await createAgentSession({
				cwd: payload.cwd,
				agentDir: catalog.agentDirectory,
				model,
				thinkingLevel: "high",
				modelRuntime,
				resourceLoader,
				settingsManager,
				sessionManager,
				tools: [...toolsForProfile(payload.toolProfile)],
				excludeTools: [],
				customTools: workspaceToolDefinitions(payload.cwd, payload.toolProfile),
				scopedModels: [],
			});
			if (result.modelFallbackMessage !== undefined) throw new SdkConstructionError("execution_profile_drift");

			const session = result.session;
			assertEffectiveSession(session, sessionIdentity, payload, toolsForProfile(payload.toolProfile));
			return new PiSdkSession(session, sessionIdentity, payload.cwd, payload.sessionDirectory, payload.modelCatalog);
		} catch (error) {
			if (error instanceof SdkConstructionError) throw error;
			throw new SdkConstructionError("sdk_operation_failed");
		}
	}
}

/**
 * Keep Pi's dynamic model-catalog cache in the host process. The admitted
 * `models.json` remains the durable, digest-bound catalog; this store is only
 * the SDK's optional remote-refresh cache and is never allowed to become an
 * ambient file input.
 */
function createInMemoryModelsStore() {
	return {
		read: async (_providerId: string) => undefined,
		write: async (_providerId: string, _entry: unknown) => undefined,
		delete: async (_providerId: string) => undefined,
	};
}

/**
 * The Pi SDK's built-in file tools resolve paths relative to cwd but otherwise
 * permit absolute paths and `..`. This profile keeps the familiar tool names
 * and schemas while replacing their filesystem operations with a canonical
 * workspace policy. It deliberately does not provide bash, grep, find, Forum
 * tools, or any subprocess-backed custom tool.
 */
export function workspaceToolDefinitions(
	cwd: string,
	toolProfile: ToolProfile,
): ToolDefinition<any, any, any>[] {
	if (toolProfile !== "workspace_isolated_v1") return [];

	const policy = new WorkspacePathPolicy(cwd);
	const readOperations: ReadOperations = {
		readFile: async (path) => readFile(await policy.existing(path)),
		access: async (path) => {
			await stat(await policy.existing(path));
		},
	};
	const writeOperations: WriteOperations = {
		writeFile: async (path, content) => {
			await writeFile(await policy.creatable(path), content);
		},
		mkdir: async (path) => {
			await mkdir(await policy.creatable(path), { recursive: true });
		},
	};
	const editOperations: EditOperations = {
		readFile: async (path) => readFile(await policy.existing(path)),
		writeFile: async (path, content) => {
			await writeFile(await policy.creatable(path), content);
		},
		access: async (path) => {
			await stat(await policy.existing(path));
		},
	};
	const lsOperations: LsOperations = {
		exists: async (path) => {
			try {
				await policy.existing(path);
				return true;
			} catch {
				return false;
			}
		},
		stat: async (path) => stat(await policy.existing(path)),
		readdir: async (path) => readdir(await policy.existing(path)),
	};

	return [
		createReadToolDefinition(cwd, { operations: readOperations }),
		createEditToolDefinition(cwd, { operations: editOperations }),
		createWriteToolDefinition(cwd, { operations: writeOperations }),
		createLsToolDefinition(cwd, { operations: lsOperations }),
	];
}

export class WorkspacePathPolicy {
	private readonly root: string;

	constructor(root: string) {
		this.root = resolve(root);
	}

	private async assertInside(candidate: string): Promise<string> {
		const root = await realpath(this.root);
		const path = resolve(candidate);
		const pathFromRoot = relative(root, path);
		if (
			pathFromRoot === "" ||
			(pathFromRoot !== ".." &&
				!pathFromRoot.startsWith(`..${sep}`) &&
				!isAbsolute(pathFromRoot))
		) {
		return path;
		}
		throw new Error("workspace_path_escape");
	}

	private assertLexicallyInside(candidate: string): string {
		const path = resolve(candidate);
		const pathFromRoot = relative(this.root, path);
		if (
			pathFromRoot === "" ||
			(pathFromRoot !== ".." &&
				!pathFromRoot.startsWith(`..${sep}`) &&
				!isAbsolute(pathFromRoot))
		) {
			return path;
		}
		throw new Error("workspace_path_escape");
	}

	private async canonicalExisting(path: string): Promise<string> {
		const resolved = this.assertLexicallyInside(path);
		const canonical = await realpath(resolved);
		return this.assertInside(canonical);
	}

	/** Resolve a path whose final component may not exist yet. */
	private async canonicalForCreation(path: string): Promise<string> {
		let current = this.assertLexicallyInside(path);
		const missingComponents: string[] = [];

		while (true) {
			try {
				const canonicalBase = await realpath(current);
				const candidate = join(canonicalBase, ...missingComponents.reverse());
				return this.assertInside(candidate);
			} catch (error) {
				if (!isNotFound(error)) throw error;
				const parent = dirname(current);
				if (parent === current) throw error;
				missingComponents.push(basename(current));
				current = parent;
			}
		}
	}

	async existing(path: string): Promise<string> {
		return this.canonicalExisting(path);
	}

	async creatable(path: string): Promise<string> {
		return this.canonicalForCreation(path);
	}
}

function isNotFound(error: unknown): boolean {
	return typeof error === "object" && error !== null && (error as { code?: unknown }).code === "ENOENT";
}

/**
 * No loader method reads ambient files. Pi still derives its explicit tool
 * descriptions from the closed tool allowlist passed to `createAgentSession`.
 */
export function createInertResourceLoader(systemPrompt: string): ResourceLoader {
	const emptyExtensions = {
		extensions: [],
		errors: [],
		runtime: createExtensionRuntime(),
	};
	return {
		getExtensions: () => emptyExtensions,
		getSkills: () => ({ skills: [], diagnostics: [] }),
		getPrompts: () => ({ prompts: [], diagnostics: [] }),
		getThemes: () => ({ themes: [], diagnostics: [] }),
		getAgentsFiles: () => ({ agentsFiles: [] }),
		getSystemPrompt: () => systemPrompt,
		getSystemPromptSource: () => undefined,
		getAppendSystemPrompt: () => [],
		getAppendSystemPromptSources: () => [],
		extendResources: () => {
			throw new SdkConstructionError("execution_profile_drift");
		},
		reload: async () => {},
	};
}

function toPiSettings(policy: ActorModelPolicyV1) {
	return {
		retry: {
			enabled: policy.retry.maxRetries > 0,
			maxRetries: policy.retry.maxRetries,
			baseDelayMs: policy.retry.baseDelayMilliseconds,
			provider: {
				timeoutMs: policy.retry.providerTimeoutMilliseconds,
				maxRetries: policy.retry.providerMaxRetries,
				maxRetryDelayMs: policy.retry.providerMaxRetryDelayMilliseconds,
			},
		},
		compaction: {
			enabled: policy.compaction.mode === "enabled",
			reserveTokens: policy.compaction.reserveTokens,
			keepRecentTokens: policy.compaction.keepRecentTokens,
		},
		steeringMode: policy.steeringMode,
		followUpMode: policy.followUpMode,
		transport: policy.transport,
		defaultProjectTrust: policy.projectTrust,
		enableInstallTelemetry: policy.installTelemetryEnabled,
		enableAnalytics: policy.analyticsEnabled,
		images: { blockImages: policy.images === "blocked" },
	} as const;
}

function assertSystemPromptDigest(payload: CreateSessionPayload): void {
	const observed = blake3Hex(payload.systemPrompt);
	if (observed !== payload.systemPromptDigest) throw new SdkConstructionError("execution_profile_drift");
}

function assertExactCreatePayload(payload: CreateSessionPayload): void {
	if (
		payload.model.provider !== PINNED_PROVIDER ||
		payload.model.modelId !== PINNED_MODEL ||
		payload.model.thinkingLevel !== PINNED_THINKING_LEVEL
		|| payload.toolProfile === "workspace_isolated_v1" && payload.forumContract.kind !== "sequestered_v1"
	) throw new SdkConstructionError("execution_profile_drift");
	try {
		assertPinnedActorModelPolicy(payload.settings);
		assertPinnedModelCatalogPolicy(payload.modelCatalog);
		assertPinnedForumSessionContract(payload.forumContract);
	} catch {
		throw new SdkConstructionError("execution_profile_drift");
	}
}

interface BoundCatalogFiles {
	readonly agentDirectory: AbsolutePath;
	readonly authPath: AbsolutePath;
	readonly modelsPath: AbsolutePath;
	readonly catalogBlake3: ReturnType<typeof blake3Digest>;
}

/**
 * The catalog and auth file must resolve beneath the owned agent directory.
 * `realpath` is deliberate: lexical paths alone cannot contain a symlink that
 * redirects the model treatment or credentials outside the admitted boundary.
 */
async function bindCatalogFiles(payload: CreateSessionPayload): Promise<BoundCatalogFiles> {
	try {
		const agentDirectory = absolutePath(await realpath(payload.agentDirectory));
		const authPath = await resolveOwnedRegularFile(payload.authPath, agentDirectory);
		const modelsPath = await resolveOwnedRegularFile(payload.modelsPath, agentDirectory);
		const bytes = await readFile(modelsPath);
		const catalogBlake3 = blake3Digest(blake3Hex(bytes));
		if (catalogBlake3 !== payload.modelCatalog.catalogBlake3) throw new SdkConstructionError("execution_profile_drift");
		return { agentDirectory, authPath, modelsPath, catalogBlake3 };
	} catch (error) {
		if (error instanceof SdkConstructionError) throw error;
		throw new SdkConstructionError("execution_profile_drift");
	}
}

async function assertCatalogUnchanged(catalog: BoundCatalogFiles): Promise<void> {
	try {
		const observed = blake3Digest(blake3Hex(await readFile(catalog.modelsPath)));
		if (observed !== catalog.catalogBlake3) throw new SdkConstructionError("execution_profile_drift");
	} catch (error) {
		if (error instanceof SdkConstructionError) throw error;
		throw new SdkConstructionError("execution_profile_drift");
	}
}

async function resolveOwnedRegularFile(path: AbsolutePath, agentDirectory: AbsolutePath): Promise<AbsolutePath> {
	const resolved = absolutePath(await realpath(path));
	assertPathContained(resolved, agentDirectory);
	if (!(await stat(resolved)).isFile()) throw new SdkConstructionError("execution_profile_drift");
	return resolved;
}

function assertPathContained(path: AbsolutePath, directory: AbsolutePath): void {
	const child = relative(directory, path);
	if (child.length === 0 || child === ".." || child.startsWith(`..${"/"}`) || isAbsolute(child)) {
		throw new SdkConstructionError("execution_profile_drift");
	}
}

function assertEffectiveBillingTreatment(model: {
	readonly baseUrl: string;
	readonly api: string;
	readonly id: string;
	readonly input: readonly string[];
	readonly contextWindow: number;
	readonly maxTokens: number;
	readonly cost: { readonly input: number; readonly output: number; readonly cacheRead: number; readonly cacheWrite: number };
}, expected: EffectiveModelDescriptorV1): void {
	if (
		model.baseUrl !== expected.baseUrl || model.api !== expected.api || model.id !== expected.modelId ||
		model.contextWindow !== expected.contextWindow || model.maxTokens !== expected.maxTokens ||
		model.input.length !== 1 || model.input[0] !== "text" ||
		!samePerTokenRate(model.cost.input, expected.inputUsdPerMillion.usdPerMillion) ||
		!samePerTokenRate(model.cost.output, expected.outputUsdPerMillion.usdPerMillion) ||
		!samePerTokenRate(model.cost.cacheRead, expected.cacheReadUsdPerMillion.usdPerMillion) ||
		!matchesCacheWriteNormalization(model.cost.cacheWrite, expected)
	) throw new SdkConstructionError("execution_profile_drift");
}

function samePerTokenRate(actual: number, perMillion: string): boolean {
	return binary64BigEndianHex(actual) === binary64BigEndianHex(Number(perMillion) / 1_000_000);
}

function matchesCacheWriteNormalization(actual: number, expected: EffectiveModelDescriptorV1): boolean {
	// Pi's ModelCost has a required numeric cacheWrite field. The admitted
	// OpenRouter catalog has no cache-write price, which normalizes only here to
	// binary64 zero; `Absent` remains distinct in durable policy/provenance.
	return expected.cacheWriteUsdPerMillion.kind === "Absent"
		? binary64BigEndianHex(actual) === binary64BigEndianHex(0)
		: samePerTokenRate(actual, expected.cacheWriteUsdPerMillion.usdPerMillion);
}

function assertEffectiveSession(
	session: AgentSession,
	sessionIdentity: SessionIdentity,
	payload: CreateSessionPayload,
	expectedTools: readonly PiToolName[],
): void {
	if (session.sessionId !== sessionIdentity || session.sessionFile === undefined) {
		throw new SdkConstructionError("execution_profile_drift");
	}
	if (session.model?.provider !== PINNED_PROVIDER || session.model.id !== PINNED_MODEL || session.thinkingLevel !== "high") {
		throw new SdkConstructionError("execution_profile_drift");
	}
	if (normalize(session.sessionManager.getCwd()) !== normalize(payload.cwd)) {
		throw new SdkConstructionError("execution_profile_drift");
	}
	const actualTools = session.getActiveToolNames();
	if (actualTools.length !== expectedTools.length || actualTools.some((tool, index) => tool !== expectedTools[index])) {
		throw new SdkConstructionError("execution_profile_drift");
	}
	if (session.systemPrompt !== expectedPiSystemPrompt(payload.systemPrompt, payload.cwd)) {
		throw new SdkConstructionError("execution_profile_drift");
	}
	const settings = session.settingsManager;
	const retry = settings.getRetrySettings();
	const providerRetry = settings.getProviderRetrySettings();
	const compaction = settings.getCompactionSettings();
	if (
		retry.maxRetries !== payload.settings.retry.maxRetries ||
		retry.baseDelayMs !== payload.settings.retry.baseDelayMilliseconds ||
		providerRetry.timeoutMs !== payload.settings.retry.providerTimeoutMilliseconds ||
		providerRetry.maxRetries !== payload.settings.retry.providerMaxRetries ||
		providerRetry.maxRetryDelayMs !== payload.settings.retry.providerMaxRetryDelayMilliseconds ||
		compaction.enabled !== (payload.settings.compaction.mode === "enabled") ||
		compaction.reserveTokens !== payload.settings.compaction.reserveTokens ||
		compaction.keepRecentTokens !== payload.settings.compaction.keepRecentTokens ||
		settings.getSteeringMode() !== payload.settings.steeringMode ||
		settings.getFollowUpMode() !== payload.settings.followUpMode ||
		settings.getTransport() !== payload.settings.transport ||
		settings.getDefaultProjectTrust() !== payload.settings.projectTrust ||
		settings.getEnableInstallTelemetry() !== payload.settings.installTelemetryEnabled ||
		settings.getEnableAnalytics() !== payload.settings.analyticsEnabled ||
		settings.getBlockImages() !== (payload.settings.images === "blocked")
	) {
		throw new SdkConstructionError("execution_profile_drift");
	}
}

function expectedPiSystemPrompt(kernelSystemPrompt: string, cwd: string): string {
	return `${kernelSystemPrompt}\nCurrent working directory: ${cwd.replace(/\\/gu, "/")}`;
}

/** Every assistant artifact must still identify the admitted model treatment. */
export function assertSdkEventExecutionProfile(event: AgentSessionEvent): void {
	switch (event.type) {
		case "agent_end":
			for (const message of event.messages) assertAssistantMessageExecutionProfile(message);
			return;
		case "turn_end":
			assertAssistantMessageExecutionProfile(event.message);
			for (const message of event.toolResults) assertAssistantMessageExecutionProfile(message);
			return;
		case "message_start":
		case "message_update":
		case "message_end":
			assertAssistantMessageExecutionProfile(event.message);
			return;
		case "entry_appended":
			if (event.entry.type === "message") assertAssistantMessageExecutionProfile(event.entry.message);
			return;
		default:
			return;
	}
}

function assertAssistantMessageExecutionProfile(message: unknown): void {
	if (typeof message !== "object" || message === null || Array.isArray(message)) return;
	const candidate = message as { role?: unknown; provider?: unknown; model?: unknown; responseModel?: unknown };
	if (candidate.role !== "assistant") return;
	if (
		candidate.provider !== PINNED_PROVIDER ||
		candidate.model !== PINNED_MODEL ||
		(candidate.responseModel !== undefined && candidate.responseModel !== PINNED_MODEL)
	) {
		throw new SdkConstructionError("execution_profile_drift");
	}
}

class PiSdkSession implements SdkSession {
	readonly sessionFile: AbsolutePath;
	readonly modelCatalog: ModelCatalogPolicyV1;
	private firstPromptRendering: string | undefined;

	constructor(
		private readonly session: AgentSession,
		readonly sessionIdentity: SessionIdentity,
		private readonly expectedCwd: AbsolutePath,
		private readonly sessionDirectory: AbsolutePath,
		modelCatalog: ModelCatalogPolicyV1,
	) {
		if (session.sessionFile === undefined) throw new SdkConstructionError("execution_profile_drift");
		this.sessionFile = absolutePath(session.sessionFile);
		this.modelCatalog = modelCatalog;
		assertSessionFileContained(this.sessionFile, this.sessionDirectory);
	}

	get isIdle(): boolean {
		return this.session.isIdle;
	}

	async prompt(text: string): Promise<void> {
		// The exact sealed task/Office rendering must not be treated as a Pi command
		// or expanded through any filesystem-backed template mechanism.
		if (this.firstPromptRendering === undefined) this.firstPromptRendering = text;
		await this.session.prompt(text, { expandPromptTemplates: false });
	}

	async followUp(text: string): Promise<void> {
		await this.session.followUp(text);
	}

	async steer(text: string): Promise<void> {
		await this.session.steer(text);
	}

	async abort(): Promise<void> {
		await this.session.abort();
	}

	dispose(): void {
		this.session.dispose();
	}

	subscribe(listener: SdkEventListener): () => void {
		return this.session.subscribe(listener);
	}

	usageTotals(): UsageTotals {
		const statistics = this.session.getSessionStats();
		return {
			inputTokens: nonNegativeInteger(statistics.tokens.input),
			outputTokens: nonNegativeInteger(statistics.tokens.output),
			cacheReadTokens: nonNegativeInteger(statistics.tokens.cacheRead),
			cacheWriteTokens: nonNegativeInteger(statistics.tokens.cacheWrite),
			totalTokens: nonNegativeInteger(statistics.tokens.total),
			providerCost: providerCostObservation(statistics.cost),
		};
	}

	async verifyCanonicalTranscript(): Promise<TranscriptFlushReceiptV1> {
		return verifyCanonicalTranscriptFile(
			this.sessionFile,
			this.sessionIdentity,
			this.expectedCwd,
			this.sessionDirectory,
			this.firstPromptRendering,
		);
	}
}

/**
 * A flush receipt is issued only after a synchronous Pi `dispose()` caller has
 * observed the JSONL file containing the original, unexpanded prompt exactly.
 * The digest is an evidence locator, not a second command payload.
 */
export async function verifyCanonicalTranscriptFile(
	sessionFile: AbsolutePath,
	sessionIdentity: SessionIdentity,
	expectedCwd: AbsolutePath,
	sessionDirectory: AbsolutePath,
	expectedFirstPromptRendering: string | undefined,
): Promise<TranscriptFlushReceiptV1> {
	try {
		const resolvedSessionDirectory = absolutePath(await realpath(sessionDirectory));
		const resolvedRequestedSessionFile = absolutePath(join(await realpath(dirname(sessionFile)), basename(sessionFile)));
		// A missing lazy transcript has no inode to resolve yet; containment still
		// holds against the resolved directory before we issue its explicit absent
		// receipt. A materialized transcript must additionally resolve beneath it.
		assertSessionFileContained(resolvedRequestedSessionFile, resolvedSessionDirectory);
		try {
			await access(sessionFile);
		} catch (error) {
			if (expectedFirstPromptRendering === undefined && isMissingFile(error)) {
				return {
					format: "pi_session_manager_jsonl_v3",
					sessionIdentity,
					sessionFile,
					materialization: "unmaterialized_no_prompt",
					firstUserPrompt: { kind: "absent" },
				};
			}
			throw error;
		}
		const resolvedSessionFile = absolutePath(await realpath(sessionFile));
		assertSessionFileContained(resolvedSessionFile, resolvedSessionDirectory);
		const transcript = await readFile(resolvedSessionFile, "utf8");
		const entries = parseJsonl(transcript);
		const headerCwd = await matchingSessionHeaderCwd(entries[0], sessionIdentity, expectedCwd);
		if (headerCwd === undefined) throw new SdkConstructionError("sdk_operation_failed");
		const firstUserMessage = entries.find(isUserMessageEntry);
		const firstUserPrompt = expectedFirstPromptRendering === undefined
			? absentFirstPromptReceipt(firstUserMessage)
			: verifiedFirstPromptReceipt(firstUserMessage, expectedFirstPromptRendering);
		return {
			format: "pi_session_manager_jsonl_v3",
			sessionIdentity,
			sessionFile: resolvedSessionFile,
			materialization: "observed",
			sessionFileBlake3: blake3Digest(blake3Hex(transcript)),
			headerCwd,
			firstUserPrompt,
		};
	} catch (error) {
		if (error instanceof SdkConstructionError) throw error;
		throw new SdkConstructionError("sdk_operation_failed");
	}
}

function isMissingFile(error: unknown): boolean {
	return typeof error === "object" && error !== null && (error as { code?: unknown }).code === "ENOENT";
}

function parseJsonl(transcript: string): unknown[] {
	const nonEmptyLines = transcript.split("\n").filter((line) => line.length > 0);
	if (nonEmptyLines.length === 0) throw new SdkConstructionError("sdk_operation_failed");
	try {
		return nonEmptyLines.map((line) => JSON.parse(line) as unknown);
	} catch {
		throw new SdkConstructionError("sdk_operation_failed");
	}
}

async function matchingSessionHeaderCwd(value: unknown, sessionIdentity: SessionIdentity, expectedCwd: AbsolutePath): Promise<AbsolutePath | undefined> {
	if (typeof value !== "object" || value === null || Array.isArray(value)) return undefined;
	const header = value as { type?: unknown; version?: unknown; id?: unknown; timestamp?: unknown; cwd?: unknown; parentSession?: unknown };
	const keys = Object.keys(header).sort();
	const expectedKeys = ["cwd", "id", "timestamp", "type", "version"];
	if (keys.length !== expectedKeys.length || keys.some((key, index) => key !== expectedKeys[index])) return undefined;
	if (
		header.type !== "session" || header.version !== 3 || header.id !== sessionIdentity ||
		typeof header.timestamp !== "string" || !isCanonicalIsoTimestamp(header.timestamp) || typeof header.cwd !== "string"
	) return undefined;
	try {
		const observedCwd = absolutePath(header.cwd);
		if (observedCwd !== expectedCwd) return undefined;
		return absolutePath(await realpath(observedCwd)) === absolutePath(await realpath(expectedCwd)) ? observedCwd : undefined;
	} catch {
		return undefined;
	}
}

function isCanonicalIsoTimestamp(value: string): boolean {
	if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/u.test(value)) return false;
	const parsed = new Date(value);
	return Number.isFinite(parsed.getTime()) && parsed.toISOString() === value;
}

function isUserMessageEntry(value: unknown): value is { message: unknown } {
	if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
	const entry = value as { type?: unknown; message?: unknown };
	if (entry.type !== "message" || typeof entry.message !== "object" || entry.message === null || Array.isArray(entry.message)) return false;
	return (entry.message as { role?: unknown }).role === "user";
}

function hasExactTextContent(message: unknown, expectedText: string): boolean {
	if (typeof message !== "object" || message === null || Array.isArray(message)) return false;
	const content = (message as { content?: unknown }).content;
	if (!Array.isArray(content) || content.length !== 1) return false;
	const first = content[0];
	if (typeof first !== "object" || first === null || Array.isArray(first)) return false;
	const text = first as { type?: unknown; text?: unknown };
	return text.type === "text" && text.text === expectedText;
}

function absentFirstPromptReceipt(firstUserMessage: { message: unknown } | undefined): { readonly kind: "absent" } {
	if (firstUserMessage !== undefined) throw new SdkConstructionError("sdk_operation_failed");
	return { kind: "absent" };
}

function verifiedFirstPromptReceipt(
	firstUserMessage: { message: unknown } | undefined,
	expectedFirstPromptRendering: string,
): { readonly kind: "verified"; readonly digest: ReturnType<typeof blake3Digest> } {
	if (firstUserMessage === undefined || !hasExactTextContent(firstUserMessage.message, expectedFirstPromptRendering)) {
		throw new SdkConstructionError("sdk_operation_failed");
	}
	return {
		kind: "verified",
		digest: blake3Digest(blake3Hex(expectedFirstPromptRendering)),
	};
}

function assertSessionFileContained(sessionFile: AbsolutePath, sessionDirectory: AbsolutePath): void {
	const child = relative(sessionDirectory, sessionFile);
	if (child.length === 0 || child === ".." || child.startsWith(`..${"/"}`) || isAbsolute(child)) {
		throw new SdkConstructionError("execution_profile_drift");
	}
}
