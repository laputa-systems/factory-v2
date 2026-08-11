// Provider-free executable double for M4 process-ownership tests.
//
// It accepts the exact inert-host invocation shape, never imports Pi or opens
// a network socket, and emits only the pinned v1 handshake needed to prove
// supervisor process physics.  Session-identity suffixes select deterministic
// race fixtures; production code never selects behavior this way.

import { writeFileSync } from "node:fs";

// The checked-in provider-free double needs the same byte digest primitive as
// the pinned host only to make a materialized transcript receipt internally
// truthful. This is the adapter's already-pinned exact dependency; no model,
// SDK, or network surface is imported by the fixture.
import { blake3 } from "../../packages/society-pi-host/node_modules/@noble/hashes/blake3.js";

const argumentsByFlag = new Map();
for (let index = 2; index < process.argv.length; index += 2) {
	argumentsByFlag.set(process.argv[index], process.argv[index + 1]);
}

const sessionIdentity = requireArgument("--session-identity");
const spawnNonce = requireArgument("--spawn-nonce");
const runtime = {
	nodeVersion: process.version,
	adapterVersion: "1",
	piSdkVersion: "0.84.1",
	nodeExecutableBlake3: requireArgument("--node-executable-blake3"),
	lockfileBlake3: requireArgument("--lockfile-blake3"),
	adapterBuildBlake3: requireArgument("--adapter-build-blake3"),
	piTransitivePackageSetBlake3: requireArgument("--pi-transitive-package-set-blake3"),
};

if (sessionIdentity.includes("exit-before-ready")) process.exit(0);
if (sessionIdentity.includes("ignore-term")) {
	process.on("SIGTERM", () => {});
}
if (sessionIdentity.includes("m5-never-session-ready-ignore-term")) {
	// EOF from the M5 session-readiness containment must not accidentally let
	// this exact TERM-escalation fixture exit before its SIGKILL deadline.
	const keepAlive = setInterval(() => {}, 60_000);
	process.once("exit", () => clearInterval(keepAlive));
	// This is deliberately outside the JSONL protocol: the paired process
	// regression has already crossed AdapterReady, but still needs a direct
	// proof that the TERM handler and retained event-loop handle existed before
	// it advances synthetic containment deadlines.
	writeFileSync(".m5-never-session-ready-ready", "ready\n", { mode: 0o600 });
}
if (sessionIdentity.includes("m5-setup-fault-ignore-term")) {
	// The post-spawn setup-failure regression closes stdin before it begins its
	// emergency schedule. Keep the direct child alive through TERM so the test
	// exercises the documented TERM -> KILL -> direct-reap ordering, rather
	// than racing a voluntary EOF exit and a recycled process-group identity.
	const keepAlive = setInterval(() => {}, 60_000);
	process.once("exit", () => clearInterval(keepAlive));
	// The supervisor deliberately cannot initialize the Pi peer for this
	// injected setup fault. This private, test-only workspace marker proves the
	// child installed its TERM handler before the Rust regression advances the
	// logical emergency deadline; it is not host protocol evidence.
	writeFileSync(".m5-setup-fault-ready", "ready\n", { mode: 0o600 });
}
if (sessionIdentity.includes("m6-usage-unavailable-pre-agent-settled-ignore-term")) {
	// The paired M6 regression needs an owned live process through both
	// automatic-containment deadlines. EOF must not turn it into an
	// absence-versus-signal race before the supervisor can record/reap it.
	const keepAlive = setInterval(() => {}, 60_000);
	process.once("exit", () => clearInterval(keepAlive));
}
if (sessionIdentity.includes("m6-protocol-failed-final-known-ignore-term")) {
	// Keep this protocol-failed terminal fixture alive long enough to prove the
	// driver contains and reaps it, rather than receiving a voluntary EOF exit.
	const keepAlive = setInterval(() => {}, 60_000);
	process.once("exit", () => clearInterval(keepAlive));
}
if (sessionIdentity.includes("m7-dispose-usage-unavailable-ignore-term")) {
	// The Dispose failure bridge must prove deadline-driven TERM/KILL and
	// ordered reaping after the exact UsageUnavailable freeze, rather than
	// racing a voluntary stdin EOF exit.
	const keepAlive = setInterval(() => {}, 60_000);
	process.once("exit", () => clearInterval(keepAlive));
}
if (sessionIdentity.includes("never-adapter")) {
	const keepAlive = setInterval(() => {}, 60_000);
	process.once("exit", () => clearInterval(keepAlive));
	await new Promise(() => {});
}

let outboundSequence = 1;
let createPayload;
let disposed = false;
let promptCount = 0;
let pendingPromptTerminal;
let pendingForumTerminal;
let firstPromptRendering;
let transcriptBytes;

emit({ event: "AdapterReady", pid: process.pid, spawnNonce, runtime });
if (sessionIdentity.includes("never-read-stdin")) {
	// The supervisor must never block writing a large CreateSession frame to
	// this deliberately paused control pipe.
	const keepAlive = setInterval(() => {}, 60_000);
	process.once("exit", () => clearInterval(keepAlive));
	await new Promise(() => {});
}
if (sessionIdentity.includes("paused-reader-resume")) {
	// This delay begins only after AdapterReady was written. It is deliberately
	// beyond the supervisor's 5s graceful abort deadline. The paired test
	// drives that deadline and terminates this process before it can attach an
	// stdin reader; it never relies on voluntary reader resumption.
	await new Promise((resolve) => setTimeout(resolve, 60_000));
}
if (sessionIdentity.includes("malformed-live")) {
	process.stdout.write("{\n");
	if (sessionIdentity.includes("malformed-live-ignore-term")) {
		// Keep this exact forced-escalation fixture alive after the supervisor
		// closes stdin. Its paired test proves the SIGKILL receipt rather than
		// conflating a voluntary EOF exit with a failed escalation.
		const keepAlive = setInterval(() => {}, 60_000);
		process.once("exit", () => clearInterval(keepAlive));
	}
}
if (sessionIdentity.includes("overlong-live")) {
	process.stdout.write(`${"x".repeat(1_048_577)}\n`);
}
if (sessionIdentity.includes("escaped-descendant-holds-pipe")) {
	// This deliberately leaves the owned group. The Rust assertion is only
	// that it detects incomplete pipe evidence and reaps the direct child
	// without blocking; it never claims this detached process was killed.
	const { spawn } = await import("node:child_process");
	const escaped = spawn(process.execPath, ["-e", "setTimeout(() => {}, 1500)"], {
		detached: true,
		stdio: ["ignore", "inherit", "inherit"],
	});
	escaped.unref();
	setImmediate(() => process.exit(0));
}
if (sessionIdentity.includes("owned-descendant-after-ready")) {
	// Unlike the escaped-descendant fixture above, this descendant deliberately
	// remains in the adapter's owned process group. The M5 bridge must first
	// record the direct wait, then issue its one distinct LingeringGroupKill,
	// then later observe the group absent before sealing/finalizing.
	const { spawn } = await import("node:child_process");
	spawn(process.execPath, ["-e", "setInterval(() => {}, 60_000)"], {
		stdio: ["ignore", "inherit", "inherit"],
	});
	// PostgreSQL-backed admission is deliberately slower than the old
	// in-process fixture. Keep the direct child alive long enough for the
	// resident to persist AdapterReady before testing descendant custody.
	setTimeout(() => process.exit(0), 1000);
}
if (sessionIdentity.includes("malformed-after-ready")) {
	process.stdout.write("{\n");
	setTimeout(() => process.exit(0), 1000);
}
// Leave a real observation window between AdapterReady and direct exit. The
// resident persists native admission before it can poll this frame.
// Keep the observation window generous enough for a loaded PostgreSQL-backed
// test runner to persist AdapterReady before this race fixture exits.
if (sessionIdentity.includes("exit-after-ready")) setTimeout(() => process.exit(0), 1000);

let pending = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
	pending += chunk;
	for (;;) {
		const newline = pending.indexOf("\n");
		if (newline === -1) return;
		const line = pending.slice(0, newline);
		pending = pending.slice(newline + 1);
		if (line.length !== 0) accept(JSON.parse(line));
	}
});
process.stdin.on("end", () => {
	if (!disposed) process.exitCode = 0;
});

function requireArgument(flag) {
	const value = argumentsByFlag.get(flag);
	if (typeof value !== "string") throw new Error(`missing ${flag}`);
	return value;
}

function emit(event) {
	process.stdout.write(`${JSON.stringify({
		protocolVersion: "society-pi-host/v4",
		sequence: outboundSequence++,
		sessionIdentity,
		...event,
	})}\n`);
}

function accepted(command, correlationIdentity) {
	emit({
		event: "CommandResult",
		correlationIdentity,
		command,
		accepted: true,
		detail: { kind: "acknowledged" },
	});
}

function accept(frame) {
	switch (frame.command) {
		case "CreateSession": {
			createPayload = frame.payload;
			// This double is also the provider-free proof that the supervisor
			// used the direct native workspace and its closed EmptyV1 environment,
			// rather than merely echoing the requested protocol paths.
			if (process.cwd() !== createPayload.cwd) {
				throw new Error("supervisor current working directory drift");
			}
			// Darwin's process runtime synthesizes this one value even under
			// `env -i`; it is not inherited from the supervisor and is neither
			// a host capability nor a secret. Any other key proves env_clear
			// failed at this boundary.
			const runtimeSynthesizedEnvironment = new Set(["__CF_USER_TEXT_ENCODING"]);
			if (Object.keys(process.env).some((key) => !runtimeSynthesizedEnvironment.has(key))) {
				throw new Error("supervisor inherited an ambient environment value");
			}
			accepted("CreateSession", frame.correlationIdentity);
			if (sessionIdentity.includes("never-session-ready")) return;
			emit({
				event: "SessionReady",
				correlationIdentity: frame.correlationIdentity,
				configuration: {
					sessionKind: createPayload.sessionKind,
					cwd: createPayload.cwd,
					sessionDirectory: createPayload.sessionDirectory,
					sessionFile: `${createPayload.sessionDirectory}/double-session.jsonl`,
					model: createPayload.model,
					modelCatalog: createPayload.modelCatalog,
					toolProfile: createPayload.toolProfile,
					tools: toolsForProfile(createPayload.toolProfile),
					settings: createPayload.settings,
					forumContract: createPayload.forumContract,
				},
			});
			if (sessionIdentity.includes("exit-after-session-ready")) {
				// The test-only Rust scheduling seam pauses after it has durably
				// recorded this exact protocol fact. The direct child then exits
				// before the separate Office-ready liveness probe. Leave enough
				// time for a loaded provider-free test process to observe and
				// commit SessionReady before the deterministic pause begins.
				// Allow the PostgreSQL-backed Rust bridge to persist SessionReady
				// before the separate liveness seam deliberately pauses it.
				setTimeout(() => process.exit(0), 500);
			}
			return;
		}
		case "Abort":
			accepted("Abort", frame.correlationIdentity);
			return;
		case "GetState":
			accepted("GetState", frame.correlationIdentity);
			if (pendingPromptTerminal) {
				emit({
					event: "UsageSnapshot",
					correlationIdentity: frame.correlationIdentity,
					usage: knownUsage(0),
				});
				const pendingTerminal = pendingPromptTerminal;
				pendingPromptTerminal = undefined;
				pendingTerminal();
			}
			return;
		case "Prompt": {
			if (
				frame.payload.purpose !== "OfficeTurn" &&
				frame.payload.purpose !== "TaskAssignment"
			) {
				throw new Error("provider-free fixture accepts only an OfficeTurn or TaskAssignment Prompt");
			}
			accepted("Prompt", frame.correlationIdentity);
			promptCount += 1;
			if (firstPromptRendering === undefined) {
				firstPromptRendering = frame.payload.text;
				transcriptBytes = Buffer.from(
					`provider-free-session-v3\n${createPayload.cwd}\n${firstPromptRendering}\n`,
					"utf8",
				);
				writeFileSync(sessionTranscriptPath(), transcriptBytes, { mode: 0o600 });
			}
			if (sessionIdentity.includes("m6-exit-after-prompt-accepted")) {
				setImmediate(() => process.exit(0));
				return;
			}
			if (sessionIdentity.includes("m6-usage-unavailable-pre-agent-settled")) {
				// Pi may fail to produce cumulative accounting before it has an
				// assistant lifecycle outcome. This remains an exact Prompt-correlated
				// host fact, not a reason to synthesize `agent_settled` or `Settled`.
				emit({
					event: "UsageSnapshot",
					correlationIdentity: frame.correlationIdentity,
					usage: { kind: "Unavailable", reason: "invalid_sdk_usage" },
				});
				return;
			}
			if (sessionIdentity.includes("m6-sdk-promise-rejected-final-known")) {
				// The real host's SDK-promise rejection path has no assistant
				// lifecycle event. Its forced Prompt-correlated Known snapshot and
				// immediately adjacent Settled frame are the complete peer evidence.
				emit({
					event: "UsageSnapshot",
					correlationIdentity: frame.correlationIdentity,
					usage: knownUsage(promptCount),
				});
				emit({
					event: "Settled",
					correlationIdentity: frame.correlationIdentity,
					classification: "failed",
					finalAssistantOutcome: {
						kind: "Unavailable",
						reason: "sdk_promise_rejected",
					},
				});
				return;
			}
			const stopReason = sessionIdentity.includes("m6-prompt-error") ? "error" : "stop";
			emit({
				event: "AgentEvent",
				correlationIdentity: frame.correlationIdentity,
				agentEvent: { type: "agent_start" },
			});
			emit({
				event: "AgentEvent",
				correlationIdentity: frame.correlationIdentity,
				agentEvent: {
					type: "agent_end",
					messages: [{ role: "assistant", stopReason }],
				willRetry: false,
				},
			});
			const emitPromptTerminal = () => {
				emit({
					event: "UsageSnapshot",
					correlationIdentity: frame.correlationIdentity,
					usage: knownUsage(promptCount),
				});
				const protocolFailed = sessionIdentity.includes("m6-protocol-failed-final-known");
				emit({
					event: "Settled",
					correlationIdentity: frame.correlationIdentity,
					classification: protocolFailed
						? "protocol_failed"
						: stopReason === "stop"
							? "completed"
							: "error",
					finalAssistantOutcome: protocolFailed
						? { kind: "Unavailable", reason: "missing_final_assistant_outcome" }
						: { kind: "Observed", stopReason },
				});
			};
			if (sessionIdentity.includes("m6-forum-call")) {
				// The actor cannot advance to settled evidence until the resident
				// returns the result of this peer-validated, daemon-authorized call.
				emit({
					event: "ForumToolCall",
					correlationIdentity: frame.correlationIdentity,
					toolCallIdentity: "forum-call-1",
					toolName: "society_forum_post",
					args: {
						message_kind: "finding",
						body_utf8: "provider-free Forum bridge observation",
						in_reply_to_message_id: null,
						supersedes_message_id: null,
					},
				});
				pendingForumTerminal = () => {
					emit({
						event: "AgentEvent",
						correlationIdentity: frame.correlationIdentity,
						agentEvent: { type: "agent_settled" },
					});
					emitPromptTerminal();
				};
				return;
			}
			if (sessionIdentity.includes("m6-known-before-and-final-same")) {
				// The first cumulative snapshot is useful observability but cannot
				// certify the turn. Pi then forces the exact same totals after
				// AgentSettled, which the kernel must retain as final evidence.
				emit({
					event: "UsageSnapshot",
					correlationIdentity: frame.correlationIdentity,
					usage: knownUsage(promptCount),
				});
			}
			emit({
				event: "AgentEvent",
				correlationIdentity: frame.correlationIdentity,
				agentEvent: { type: "agent_settled" },
			});
			if (sessionIdentity.includes("m6-usage-unavailable")) {
				emit({
					event: "UsageSnapshot",
					correlationIdentity: frame.correlationIdentity,
					usage: { kind: "Unavailable", reason: "invalid_sdk_usage" },
				});
				return;
			}
			if (sessionIdentity.includes("m6-missing-final-usage")) {
				// This exact frame is schema-valid but deliberately violates the
				// peer's final-accounting invariant. The Rust driver must preserve
				// this observed Settled sequence as Unknown, not invent Usage.
				emit({
					event: "Settled",
					correlationIdentity: frame.correlationIdentity,
					classification: stopReason === "stop" ? "completed" : "error",
					finalAssistantOutcome: {
						kind: "Observed",
						stopReason,
					},
				});
				return;
			}
			if (sessionIdentity.includes("m6-control-interleave")) {
				pendingPromptTerminal = emitPromptTerminal;
			} else {
				emitPromptTerminal();
			}
			return;
		}
		case "ForumToolResult":
			if (!sessionIdentity.includes("m6-forum-call")) {
				throw new Error("provider-free fixture received an unexpected Forum result");
			}
			if (frame.payload.toolCallIdentity !== "forum-call-1" || frame.payload.isError) {
				throw new Error("resident did not return the admitted Forum call result");
			}
			if (pendingForumTerminal === undefined) {
				throw new Error("Forum result arrived without a pending call");
			}
			const forumTerminal = pendingForumTerminal;
			pendingForumTerminal = undefined;
			forumTerminal();
			return;
		case "Dispose":
			accepted("Dispose", frame.correlationIdentity);
			if (sessionIdentity.includes("m7-dispose-usage-unavailable")) {
				// This is the actual host ordering for typed unusable cumulative
				// accounting: the frame itself terminally fences the peer, so no
				// later Disposed receipt can be trusted or projected.
				emit({
					event: "UsageSnapshot",
					correlationIdentity: frame.correlationIdentity,
					usage: { kind: "Unavailable", reason: "invalid_sdk_usage" },
				});
				return;
			}
			if (sessionIdentity.includes("m7-dispose-missing-final-usage")) {
				// Schema-valid terminal coordinates without the mandatory forced
				// final Usage frame. The Rust peer must fence this exact Disposed
				// sequence; the daemon may freeze but must never close the session.
				emit({
					event: "Disposed",
					correlationIdentity: frame.correlationIdentity,
					transcriptFlushReceipt: transcriptFlushReceipt(),
				});
				disposed = true;
				return;
			}
			emit({
				event: "UsageSnapshot",
				correlationIdentity: frame.correlationIdentity,
				// The host's forced final Dispose snapshot is cumulative. After a
				// completed prompt it commonly repeats the same total, which is
				// still terminal evidence even though the peer normalizes it as
				// an explicit idempotent zero delta.
				usage: knownUsage(promptCount),
			});
			emit({
				event: "Disposed",
				correlationIdentity: frame.correlationIdentity,
				transcriptFlushReceipt: transcriptFlushReceipt(),
			});
			disposed = true;
			return;
		default:
			throw new Error(`unexpected provider-free command ${frame.command}`);
	}
}

function sessionTranscriptPath() {
	return `${createPayload.sessionDirectory}/double-session.jsonl`;
}

function blake3Hex(bytes) {
	return Buffer.from(blake3(bytes)).toString("hex");
}

function transcriptFlushReceipt() {
	if (firstPromptRendering === undefined || transcriptBytes === undefined) {
		return {
			format: "pi_session_manager_jsonl_v3",
			sessionIdentity,
			sessionFile: sessionTranscriptPath(),
			materialization: "unmaterialized_no_prompt",
			firstUserPrompt: { kind: "absent" },
		};
	}
	return {
		format: "pi_session_manager_jsonl_v3",
		sessionIdentity,
		sessionFile: sessionTranscriptPath(),
		materialization: "observed",
		sessionFileBlake3: blake3Hex(transcriptBytes),
		headerCwd: createPayload.cwd,
		firstUserPrompt: {
			kind: "verified",
			digest: blake3Hex(Buffer.from(firstPromptRendering, "utf8")),
		},
	};
}

function knownUsage(turn) {
	// The first cumulative snapshot ceilings to 4 micro-USD; the second uses
	// 8.5 micro-USD, whose exact binary64 ceiling is 9. This proves a 5-micro
	// delta without any provider call or a JS decimal-display round trip.
	const cost = turn === 0 ? 0 : turn === 1 ? 0.000004 : 0.0000085;
	const bytes = Buffer.alloc(8);
	bytes.writeDoubleBE(cost);
	return {
		kind: "Known",
		totals: {
			inputTokens: turn,
			outputTokens: turn,
			cacheReadTokens: turn,
			cacheWriteTokens: turn,
			totalTokens: 4 * turn,
			providerCost: {
				encoding: "ieee754_binary64_be_hex_v1",
				binary64BigEndianHex: bytes.toString("hex"),
				rounding: "ceil_to_micro_usd",
			},
		},
	};
}

function toolsForProfile(profile) {
	if (profile === "read_execute_v1") return ["read", "bash", "grep", "find", "ls"];
	if (profile === "read_write_v1") return ["read", "write"];
	if (profile === "forum_isolated_v1") return ["society_forum_read", "society_forum_post"];
	return ["read", "bash", "edit", "write", "grep", "find", "ls"];
}
