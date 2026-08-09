import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import { blake3Hex } from "../src/digest.js";
import { MAX_JSONL_FRAME_BYTES, type OutboundFrame } from "../src/protocol.js";
import { consumeInboundJsonl } from "../src/main.js";

const hostEntrypoint = fileURLToPath(new URL("../src/main.js", import.meta.url));
const DIGEST = "a".repeat(64);

test("main: malformed real-pipe input produces stdout-only JSONL and a typed fatal", async () => {
	const result = await runHost(["{"]);
	assert.equal(result.stderr, "");
	assert.equal(result.code, 1);
	const frames = parseFrames(result.stdout);
	assert.deepEqual(frames.map((frame) => frame.event), ["AdapterReady", "Fatal"]);
	assert.equal(frames[1]?.event, "Fatal");
	if (frames[1]?.event === "Fatal") assert.equal(frames[1].failureCode, "protocol_decode_failed");
});

test("main: byte decoder bounds an overlong line before JSON decoding", async () => {
	const result = await runHost(["x".repeat(MAX_JSONL_FRAME_BYTES + 1)]);
	assert.equal(result.stderr, "");
	const frames = parseFrames(result.stdout);
	assert.deepEqual(frames.map((frame) => frame.event), ["AdapterReady", "Fatal"]);
});

test("main: a valid final JSONL record is delivered on EOF without requiring a trailing newline", async () => {
	const lines: string[] = [];
	await consumeInboundJsonl(oneChunk(Buffer.from('{"final":true}', "utf8")), (line) => {
		if (line !== undefined) lines.push(line);
	});
	assert.deepEqual(lines, ['{"final":true}']);
});

test("main: deeply nested sub-megabyte input terminates in a typed fatal, not a stack abort", async () => {
	const deeplyNested = `${"[".repeat(10_000)}${"]".repeat(10_000)}`;
	const result = await runHost([deeplyNested]);
	assert.equal(result.stderr, "");
	assert.equal(result.code, 1);
	assert.deepEqual(parseFrames(result.stdout).map((frame) => frame.event), ["AdapterReady", "Fatal"]);
});

test("main: invalid UTF-8 in an otherwise-shaped Prompt is contained before command admission", async () => {
	const prefix = Buffer.from('{"protocolVersion":"society-pi-host/v3","sequence":1,"sessionIdentity":"pipe-session-001","correlationIdentity":"prompt-001","command":"Prompt","payload":{"purpose":"TaskAssignment","text":"', "utf8");
	const suffix = Buffer.from('"}}', "utf8");
	const result = await runHostBytes([prefix, Buffer.from([0xff]), suffix], true);
	assert.equal(result.stderr, "");
	assert.equal(result.code, 1);
	const frames = parseFrames(result.stdout);
	assert.deepEqual(frames.map((frame) => frame.event), ["AdapterReady", "Fatal"]);
	assert.equal(frames.some((frame) => frame.event === "SessionReady"), false);
});

test("main: invalid UTF-8 EOF fragment is contained without replacement decoding", async () => {
	const result = await runHostBytes([Buffer.from('{"final":"', "utf8"), Buffer.from([0xff])], false);
	assert.equal(result.stderr, "");
	assert.equal(result.code, 1);
	assert.deepEqual(parseFrames(result.stdout).map((frame) => frame.event), ["AdapterReady", "Fatal"]);
});

test("main: a supervisor-closed stdout pipe is contained without protocol diagnostics on stderr", async () => {
	const code = await runHostWithClosedStdout("{");
	assert.equal(code, 1);
});

test("main: real-pipe CreateSession then Dispose flushes before EOF without a provider call", async (context) => {
	const directory = await mkdtemp(join(tmpdir(), "society-pi-host-main-"));
	context.after(async () => rm(directory, { recursive: true, force: true }));
	const agentDirectory = join(directory, "agent");
	const sessionDirectory = join(directory, "sessions");
	await mkdir(agentDirectory);
	await mkdir(sessionDirectory);
	const authPath = join(agentDirectory, "auth.json");
	const modelsPath = join(agentDirectory, "models.json");
	await writeFile(authPath, "{}", "utf8");
	const catalog = JSON.stringify({ providers: { openrouter: {
		baseUrl: "https://openrouter.ai/api/v1", api: "openai-completions",
		models: [{ id: "deepseek/deepseek-v4-flash-0731", name: "admitted", reasoning: true, input: ["text"], contextWindow: 1_048_576, maxTokens: 384_000,
			cost: { input: 0.00000009, output: 0.00000018, cacheRead: 0.000000018, cacheWrite: 0 } }],
	} } });
	await writeFile(modelsPath, catalog, "utf8");
	const prompt = "Universe Seed\nTask contract";
	const createPayload = {
		sessionKind: "TaskAttempt", cwd: directory, agentDirectory, authPath, modelsPath, sessionDirectory,
		systemPrompt: prompt, systemPromptDigest: blake3Hex(prompt),
		model: { provider: "openrouter", modelId: "deepseek/deepseek-v4-flash-0731", thinkingLevel: "high" },
		modelCatalog: { catalogBlake3: blake3Hex(catalog), effectiveModel: admittedDescriptor() },
		toolProfile: "read_execute_v1",
		settings: admittedSettings(),
	};
	const result = await runHost([
		JSON.stringify(envelope(1, "CreateSession", createPayload)),
		JSON.stringify(envelope(2, "Dispose", { reason: "ProcessRecovery" })),
	]);
	assert.equal(result.stderr, "");
	assert.equal(result.code, 0);
	const frames = parseFrames(result.stdout);
	assert.deepEqual(frames.map((frame) => frame.event), ["AdapterReady", "CommandResult", "SessionReady", "CommandResult", "UsageSnapshot", "Disposed"]);
	const disposed = frames.at(-1);
	assert.equal(disposed?.event, "Disposed");
	if (disposed?.event === "Disposed") assert.equal(disposed.transcriptFlushReceipt.materialization, "unmaterialized_no_prompt");
});

test("main: a broken stdout after SessionReady fences a later Prompt before it can start a provider turn", async (context) => {
	const directory = await mkdtemp(join(tmpdir(), "society-pi-host-main-transport-"));
	context.after(async () => rm(directory, { recursive: true, force: true }));
	const agentDirectory = join(directory, "agent");
	const sessionDirectory = join(directory, "sessions");
	await mkdir(agentDirectory);
	await mkdir(sessionDirectory);
	const authPath = join(agentDirectory, "auth.json");
	const modelsPath = join(agentDirectory, "models.json");
	await writeFile(authPath, "{}", "utf8");
	const catalog = JSON.stringify({ providers: { openrouter: {
		baseUrl: "https://openrouter.ai/api/v1", api: "openai-completions",
		models: [{ id: "deepseek/deepseek-v4-flash-0731", name: "admitted", reasoning: true, input: ["text"], contextWindow: 1_048_576, maxTokens: 384_000,
			cost: { input: 0.00000009, output: 0.00000018, cacheRead: 0.000000018, cacheWrite: 0 } }],
	} } });
	await writeFile(modelsPath, catalog, "utf8");
	const systemPrompt = "Universe Seed\nTransport containment";
	const create = envelope(1, "CreateSession", {
		sessionKind: "TaskAttempt", cwd: directory, agentDirectory, authPath, modelsPath, sessionDirectory,
		systemPrompt, systemPromptDigest: blake3Hex(systemPrompt),
		model: { provider: "openrouter", modelId: "deepseek/deepseek-v4-flash-0731", thinkingLevel: "high" },
		modelCatalog: { catalogBlake3: blake3Hex(catalog), effectiveModel: admittedDescriptor() },
		toolProfile: "read_execute_v1", settings: admittedSettings(),
	});
	// GetState forces an outbound write after the parent closes the pipe. The
	// following Prompt is intentionally sequenced only after that fault trigger;
	// the host fake-runtime contract separately proves no prompt call is made.
	const faultTrigger = envelope(2, "GetState", {});
	const forbiddenPrompt = envelope(3, "Prompt", { purpose: "TaskAssignment", text: "must remain unfunded" });
	const result = await runHostBreakStdoutAfterSessionReady([create, faultTrigger, forbiddenPrompt]);
	assert.equal(result.code, 1);
	assert.equal(result.stderr, "");
});

function envelope(sequence: number, command: string, payload: unknown) {
	return { protocolVersion: "society-pi-host/v3", sequence, sessionIdentity: "pipe-session-001", correlationIdentity: `pipe-command-${sequence}`, command, payload };
}

async function* oneChunk(chunk: Uint8Array): AsyncIterable<Uint8Array> {
	yield chunk;
}

function admittedDescriptor() {
	return {
		provider: "openrouter", baseUrl: "https://openrouter.ai/api/v1", api: "openai-completions", modelId: "deepseek/deepseek-v4-flash-0731",
		canonicalSlug: "deepseek/deepseek-v4-flash-20260731", input: "text_only", contextWindow: 1_048_576, maxTokens: 384_000,
		inputUsdPerMillion: { kind: "Known", usdPerMillion: "0.09" }, outputUsdPerMillion: { kind: "Known", usdPerMillion: "0.18" },
		cacheReadUsdPerMillion: { kind: "Known", usdPerMillion: "0.018" }, cacheWriteUsdPerMillion: { kind: "Absent" },
	};
}

function admittedSettings() {
	return {
		retry: { maxRetries: 2, baseDelayMilliseconds: 2_000, providerTimeoutMilliseconds: 300_000, providerMaxRetries: 1, providerMaxRetryDelayMilliseconds: 30_000 },
		compaction: { mode: "enabled", reserveTokens: 16_384, keepRecentTokens: 20_000 },
		steeringMode: "one-at-a-time", followUpMode: "one-at-a-time", transport: "sse", projectTrust: "never",
		installTelemetryEnabled: false, analyticsEnabled: false, images: "blocked",
	};
}

async function runHost(lines: readonly string[]): Promise<{ readonly code: number | null; readonly stdout: string; readonly stderr: string }> {
	return runHostBytes([Buffer.from(lines.join("\n"), "utf8")], true);
}

async function runHostBytes(chunks: readonly Buffer[], trailingNewline: boolean): Promise<{ readonly code: number | null; readonly stdout: string; readonly stderr: string }> {
	return new Promise((resolve, reject) => {
		const child = spawn(process.execPath, [hostEntrypoint,
			"--session-identity", "pipe-session-001", "--spawn-nonce", "pipe-spawn-001",
			"--node-executable-blake3", DIGEST, "--lockfile-blake3", DIGEST,
			"--adapter-build-blake3", DIGEST, "--pi-transitive-package-set-blake3", DIGEST,
		], { stdio: ["pipe", "pipe", "pipe"] });
		let stdout = "";
		let stderr = "";
		child.stdout.setEncoding("utf8");
		child.stderr.setEncoding("utf8");
		child.stdout.on("data", (chunk: string) => { stdout += chunk; });
		child.stderr.on("data", (chunk: string) => { stderr += chunk; });
		child.once("error", reject);
		child.once("close", (code) => resolve({ code, stdout, stderr }));
		child.stdin.end(Buffer.concat(trailingNewline ? [...chunks, Buffer.from("\n", "utf8")] : chunks));
	});
}

function parseFrames(stdout: string): OutboundFrame[] {
	assert.equal(stdout.endsWith("\n"), true);
	return stdout.trimEnd().split("\n").map((line) => JSON.parse(line) as OutboundFrame);
}

async function runHostWithClosedStdout(line: string): Promise<number | null> {
	return new Promise((resolve, reject) => {
		const child = spawn(process.execPath, [hostEntrypoint,
			"--session-identity", "pipe-session-001", "--spawn-nonce", "pipe-spawn-001",
			"--node-executable-blake3", DIGEST, "--lockfile-blake3", DIGEST,
			"--adapter-build-blake3", DIGEST, "--pi-transitive-package-set-blake3", DIGEST,
		], { stdio: ["pipe", "pipe", "pipe"] });
		child.once("error", reject);
		child.once("close", (code) => resolve(code));
		// Destroying the parent reader produces the child-side EPIPE that the
		// executable must handle without throwing from its stdout event emitter.
		child.stdout.destroy();
		child.stdin.end(`${line}\n`);
	});
}

async function runHostBreakStdoutAfterSessionReady(frames: readonly object[]): Promise<{ readonly code: number | null; readonly stderr: string }> {
	return new Promise((resolve, reject) => {
		const child = spawn(process.execPath, [hostEntrypoint,
			"--session-identity", "pipe-session-001", "--spawn-nonce", "pipe-spawn-001",
			"--node-executable-blake3", DIGEST, "--lockfile-blake3", DIGEST,
			"--adapter-build-blake3", DIGEST, "--pi-transitive-package-set-blake3", DIGEST,
		], { stdio: ["pipe", "pipe", "pipe"] });
		let stdout = "";
		let stderr = "";
		let broken = false;
		child.stdout.setEncoding("utf8");
		child.stderr.setEncoding("utf8");
		child.stdout.on("data", (chunk: string) => {
			stdout += chunk;
			if (!broken && stdout.includes('"event":"SessionReady"')) {
				broken = true;
				child.stdout.destroy();
				child.stdin.write(`${JSON.stringify(frames[1])}\n`);
				setTimeout(() => child.stdin.end(`${JSON.stringify(frames[2])}\n`), 100);
			}
		});
		child.stderr.on("data", (chunk: string) => { stderr += chunk; });
		child.once("error", reject);
		child.once("close", (code) => resolve({ code, stderr }));
		child.stdin.write(`${JSON.stringify(frames[0])}\n`);
	});
}
