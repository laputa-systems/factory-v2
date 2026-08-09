// Provider-free executable double for M4 process-ownership tests.
//
// It accepts the exact inert-host invocation shape, never imports Pi or opens
// a network socket, and emits only the pinned v1 handshake needed to prove
// supervisor process physics.  Session-identity suffixes select deterministic
// race fixtures; production code never selects behavior this way.

const argumentsByFlag = new Map();
for (let index = 2; index < process.argv.length; index += 2) {
	argumentsByFlag.set(process.argv[index], process.argv[index + 1]);
}

const sessionIdentity = requireArgument("--session-identity");
const spawnNonce = requireArgument("--spawn-nonce");
const runtime = {
	nodeVersion: process.version,
	adapterVersion: "1",
	piSdkVersion: "0.83.0",
	nodeExecutableSha256: requireArgument("--node-executable-sha256"),
	lockfileSha256: requireArgument("--lockfile-sha256"),
	adapterBuildSha256: requireArgument("--adapter-build-sha256"),
	piTransitivePackageSetSha256: requireArgument("--pi-transitive-package-set-sha256"),
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
}
if (sessionIdentity.includes("never-adapter")) {
	const keepAlive = setInterval(() => {}, 60_000);
	process.once("exit", () => clearInterval(keepAlive));
	await new Promise(() => {});
}

let outboundSequence = 1;
let createPayload;
let disposed = false;

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
	setImmediate(() => process.exit(0));
}
if (sessionIdentity.includes("malformed-after-ready")) {
	process.stdout.write("{\n");
	setTimeout(() => process.exit(0), 100);
}
if (sessionIdentity.includes("exit-after-ready")) setTimeout(() => process.exit(0), 25);

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
		protocolVersion: "society-pi-host/v1",
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
				},
			});
			if (sessionIdentity.includes("exit-after-session-ready")) {
				// The test-only Rust scheduling seam pauses after it has durably
				// recorded this exact protocol fact. The direct child then exits
				// before the separate Office-ready liveness probe.
				setTimeout(() => process.exit(0), 1);
			}
			return;
		}
		case "Abort":
			accepted("Abort", frame.correlationIdentity);
			return;
		case "Dispose":
			accepted("Dispose", frame.correlationIdentity);
			emit({
				event: "UsageSnapshot",
				correlationIdentity: frame.correlationIdentity,
				usage: {
					kind: "Known",
					totals: {
						inputTokens: 0,
						outputTokens: 0,
						cacheReadTokens: 0,
						cacheWriteTokens: 0,
						totalTokens: 0,
						providerCost: {
							encoding: "ieee754_binary64_be_hex_v1",
							binary64BigEndianHex: "0000000000000000",
							rounding: "ceil_to_micro_usd",
						},
					},
				},
			});
			emit({
				event: "Disposed",
				correlationIdentity: frame.correlationIdentity,
				transcriptFlushReceipt: {
					format: "pi_session_manager_jsonl_v3",
					sessionIdentity,
					sessionFile: `${createPayload.sessionDirectory}/double-session.jsonl`,
					materialization: "unmaterialized_no_prompt",
					firstUserPrompt: { kind: "absent" },
				},
			});
			disposed = true;
			return;
		default:
			throw new Error(`unexpected provider-free command ${frame.command}`);
	}
}

function toolsForProfile(profile) {
	if (profile === "read_source_v1") return ["read", "bash", "grep", "find", "ls"];
	if (profile === "curator_v1") return ["read", "write"];
	return ["read", "bash", "edit", "write", "grep", "find", "ls"];
}
