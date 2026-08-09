import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { join } from "node:path";
import test from "node:test";
import { tmpdir } from "node:os";

import { absolutePath, sessionIdentity } from "../src/protocol.js";
import {
	PinnedPiSdkRuntime,
	SdkConstructionError,
	createInertResourceLoader,
	verifyCanonicalTranscriptFile,
} from "../src/sdk.js";
import { decodeCommand } from "./support.js";
import { createSessionPayload } from "./support.js";

test("sdk: the V2 ResourceLoader exposes only the supplied system prompt and no discovered resources", async () => {
	const loader = createInertResourceLoader("exact kernel system prompt");
	await loader.reload();
	assert.equal(loader.getSystemPrompt(), "exact kernel system prompt");
	assert.deepEqual(loader.getExtensions().extensions, []);
	assert.deepEqual(loader.getSkills().skills, []);
	assert.deepEqual(loader.getPrompts().prompts, []);
	assert.deepEqual(loader.getThemes().themes, []);
	assert.deepEqual(loader.getAgentsFiles().agentsFiles, []);
	assert.deepEqual(loader.getAppendSystemPrompt(), []);
	assert.throws(() => loader.extendResources({}), SdkConstructionError);
});

test("sdk: prompt-digest drift fails before ModelRuntime construction or a provider request", async () => {
	const payload = createSessionPayload();
	payload.systemPromptDigest = "0".repeat(64);
	const command = decodeCommand(1, "CreateSession", payload);
	if (command.command !== "CreateSession") throw new Error("expected_create_session");
	await assert.rejects(
		new PinnedPiSdkRuntime().create(command.sessionIdentity, command.payload),
		(error: unknown) => error instanceof SdkConstructionError && error.code === "execution_profile_drift",
	);
});

test("sdk: typed callers cannot bypass the VS-001 execution-profile constants", async () => {
	const payload = createSessionPayload();
	const command = decodeCommand(1, "CreateSession", payload);
	if (command.command !== "CreateSession") throw new Error("expected_create_session");
	const bypassed = {
		...command.payload,
		settings: {
			...command.payload.settings,
			retry: { ...command.payload.settings.retry, baseDelayMilliseconds: 1 as typeof command.payload.settings.retry.baseDelayMilliseconds },
		},
	};
	await assert.rejects(
		new PinnedPiSdkRuntime().create(command.sessionIdentity, bypassed),
		(error: unknown) => error instanceof SdkConstructionError && error.code === "execution_profile_drift",
	);
});

test("sdk: pinned production construction accepts only an explicit local model catalog and makes no provider call", async (context) => {
	const directory = await mkdtemp(join(tmpdir(), "society-pi-host-production-"));
	context.after(async () => rm(directory, { recursive: true, force: true }));
	const agentDirectory = join(directory, "agent");
	const sessionDirectory = join(directory, "sessions");
	await mkdir(agentDirectory);
	await mkdir(sessionDirectory);
	const authPath = join(agentDirectory, "auth.json");
	const modelsPath = join(agentDirectory, "models.json");
	await writeFile(authPath, "{}", "utf8");
	const catalogText = JSON.stringify({
		providers: {
			openrouter: {
				baseUrl: "https://openrouter.ai/api/v1",
				api: "openai-completions",
				models: [{
					id: "deepseek/deepseek-v4-flash-0731",
					name: "admitted test model",
					reasoning: true,
					input: ["text"],
					cost: { input: 0.00000009, output: 0.00000018, cacheRead: 0.000000018, cacheWrite: 0 },
					contextWindow: 1_048_576,
					maxTokens: 384_000,
				}],
			},
		},
	});
	await writeFile(modelsPath, catalogText, "utf8");
	const rawPayload = createSessionPayload();
	rawPayload.cwd = directory;
	rawPayload.agentDirectory = agentDirectory;
	rawPayload.authPath = authPath;
	rawPayload.modelsPath = modelsPath;
	rawPayload.sessionDirectory = sessionDirectory;
	(rawPayload.modelCatalog as Record<string, unknown>).catalogSha256 = createHash("sha256").update(catalogText, "utf8").digest("hex");
	const command = decodeCommand(1, "CreateSession", rawPayload);
	if (command.command !== "CreateSession") throw new Error("expected_create_session");
	const session = await new PinnedPiSdkRuntime().create(command.sessionIdentity, command.payload);
	try {
		assert.equal(session.sessionIdentity, command.sessionIdentity);
		assert.deepEqual((await session.verifyCanonicalTranscript()).firstUserPrompt, { kind: "absent" });
	} finally {
		session.dispose();
	}
});

test("sdk: catalog digest, endpoint, zero-price and TOCTOU drift are rejected before a session can be admitted", async (context) => {
	const directory = await mkdtemp(join(tmpdir(), "society-pi-host-catalog-"));
	context.after(async () => rm(directory, { recursive: true, force: true }));
	const agentDirectory = join(directory, "agent");
	const sessionDirectory = join(directory, "sessions");
	await mkdir(agentDirectory);
	await mkdir(sessionDirectory);
	const authPath = join(agentDirectory, "auth.json");
	const modelsPath = join(agentDirectory, "models.json");
	await writeFile(authPath, "{}", "utf8");
	const admittedCatalog = (overrides: { readonly baseUrl?: string; readonly inputCost?: number } = {}): string => JSON.stringify({
		providers: { openrouter: {
			baseUrl: overrides.baseUrl ?? "https://openrouter.ai/api/v1",
			api: "openai-completions",
			models: [{
				id: "deepseek/deepseek-v4-flash-0731", name: "admitted test model", reasoning: true, input: ["text"],
				cost: { input: overrides.inputCost ?? 0.00000009, output: 0.00000018, cacheRead: 0.000000018, cacheWrite: 0 },
				contextWindow: 1_048_576, maxTokens: 384_000,
			}],
		} },
	});
	const commandFor = (catalogText: string, digest = createHash("sha256").update(catalogText, "utf8").digest("hex")) => {
		const payload = createSessionPayload();
		payload.cwd = directory;
		payload.agentDirectory = agentDirectory;
		payload.authPath = authPath;
		payload.modelsPath = modelsPath;
		payload.sessionDirectory = sessionDirectory;
		(payload.modelCatalog as Record<string, unknown>).catalogSha256 = digest;
		return decodeCommand(1, "CreateSession", payload);
	};
	const expectDrift = async (command: ReturnType<typeof commandFor>, runtime = new PinnedPiSdkRuntime()) => {
		if (command.command !== "CreateSession") throw new Error("expected_create_session");
		await assert.rejects(runtime.create(command.sessionIdentity, command.payload),
			(error: unknown) => error instanceof SdkConstructionError && error.code === "execution_profile_drift");
	};

	const admitted = admittedCatalog();
	await writeFile(modelsPath, admitted, "utf8");
	await expectDrift(commandFor(admitted, "0".repeat(64))); // raw-byte digest mismatch

	const zeroPrice = admittedCatalog({ inputCost: 0 });
	await writeFile(modelsPath, zeroPrice, "utf8");
	await expectDrift(commandFor(zeroPrice));

	const wrongEndpoint = admittedCatalog({ baseUrl: "https://example.invalid/api/v1" });
	await writeFile(modelsPath, wrongEndpoint, "utf8");
	await expectDrift(commandFor(wrongEndpoint));

	await writeFile(modelsPath, admitted, "utf8");
	const raceCommand = commandFor(admitted);
	await expectDrift(raceCommand, new PinnedPiSdkRuntime({
		afterCatalogRead: async () => writeFile(modelsPath, `${admitted}\n`, "utf8"),
	}));
});

test("sdk: transcript receipt proves Pi persisted the exact first unexpanded Prompt rendering", async (context) => {
	const directory = await mkdtemp(join(tmpdir(), "society-pi-host-transcript-"));
	context.after(async () => rm(directory, { recursive: true, force: true }));
	const sessionFile = absolutePath(join(directory, "session.jsonl"));
	const cwd = absolutePath(directory);
	const identity = sessionIdentity("transcript-proof-001");
	const rendering = "Universe Seed\n\nTask assignment: retain literal /template syntax.";
	await writeFile(
		sessionFile,
		[
			JSON.stringify({ type: "session", version: 3, id: identity, timestamp: "2026-01-01T00:00:00.000Z", cwd }),
			JSON.stringify({
				type: "message",
				id: "message-001",
				parentId: null,
				timestamp: "2026-01-01T00:00:01.000Z",
				message: { role: "user", content: [{ type: "text", text: rendering }], timestamp: 1 },
			}),
		].join("\n"),
		"utf8",
	);

	const receipt = await verifyCanonicalTranscriptFile(sessionFile, identity, cwd, absolutePath(directory), rendering);
	assert.equal(receipt.format, "pi_session_manager_jsonl_v3");
	assert.equal(receipt.sessionFile, absolutePath(await realpath(sessionFile)));
	assert.equal(receipt.materialization, "observed");
	assert.match(receipt.sessionFileSha256, /^[a-f0-9]{64}$/u);
	assert.equal(receipt.headerCwd, cwd);
	assert.equal(receipt.firstUserPrompt.kind, "verified");
	if (receipt.firstUserPrompt.kind === "verified") assert.match(receipt.firstUserPrompt.digest, /^[a-f0-9]{64}$/u);

	await assert.rejects(
		verifyCanonicalTranscriptFile(sessionFile, identity, cwd, absolutePath(directory), `${rendering} changed`),
		(error: unknown) => error instanceof SdkConstructionError && error.code === "sdk_operation_failed",
	);
	await assert.rejects(
		verifyCanonicalTranscriptFile(sessionFile, identity, absolutePath("/tmp"), absolutePath(directory), rendering),
		(error: unknown) => error instanceof SdkConstructionError && error.code === "sdk_operation_failed",
	);

	const unopenedSessionFile = absolutePath(join(directory, "unopened.jsonl"));
	await writeFile(
		unopenedSessionFile,
		JSON.stringify({ type: "session", version: 3, id: identity, timestamp: "2026-01-01T00:00:00.000Z", cwd }),
		"utf8",
	);
	const unopenedReceipt = await verifyCanonicalTranscriptFile(
		unopenedSessionFile,
		identity,
		cwd,
		absolutePath(directory),
		undefined,
	);
	assert.deepEqual(unopenedReceipt.firstUserPrompt, { kind: "absent" });
	assert.equal(unopenedReceipt.materialization, "observed");
});

test("sdk: transcript receipts reject noncanonical headers and session-file symlink redirection", async (context) => {
	const directory = await mkdtemp(join(tmpdir(), "society-pi-host-transcript-boundary-"));
	context.after(async () => rm(directory, { recursive: true, force: true }));
	const sessions = join(directory, "sessions");
	await mkdir(sessions);
	const identity = sessionIdentity("transcript-boundary-001");
	const externalFile = join(directory, "outside.jsonl");
	await writeFile(externalFile, JSON.stringify({ type: "session", version: 3, id: identity, timestamp: "not-a-timestamp", cwd: directory }), "utf8");
	const redirected = join(sessions, "redirect.jsonl");
	await symlink(externalFile, redirected);
	await assert.rejects(
		verifyCanonicalTranscriptFile(absolutePath(redirected), identity, absolutePath(directory), absolutePath(sessions), undefined),
		(error: unknown) => error instanceof SdkConstructionError && error.code === "execution_profile_drift",
	);

	const malformedHeader = join(sessions, "malformed.jsonl");
	await writeFile(malformedHeader, JSON.stringify({ type: "session", version: 3, id: identity, timestamp: "not-a-timestamp", cwd: directory }), "utf8");
	await assert.rejects(
		verifyCanonicalTranscriptFile(absolutePath(malformedHeader), identity, absolutePath(directory), absolutePath(sessions), undefined),
		(error: unknown) => error instanceof SdkConstructionError && error.code === "sdk_operation_failed",
	);
});
