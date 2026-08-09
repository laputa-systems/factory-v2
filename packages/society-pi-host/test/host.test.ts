import assert from "node:assert/strict";
import test from "node:test";

import { PiSdkHost, localRuntimeIdentity, resolvedInstalledPiSdkVersion } from "../src/host.js";
import { nonNegativeInteger, providerCostObservation, spawnNonce, sessionIdentity, type OutboundFrame } from "../src/protocol.js";
import { TEST_RUNTIME_EVIDENCE, DeferredSdkRuntime, FakeSdkRuntime, createSessionPayload, decodeCommand, drainMicrotasks } from "./support.js";

function makeHost() {
	const frames: OutboundFrame[] = [];
	const runtime = new FakeSdkRuntime();
	const host = new PiSdkHost(
		{
			sessionIdentity: sessionIdentity("pi-session-test-001"),
			spawnNonce: spawnNonce("spawn-test-001"),
			pid: 4242,
			runtime: localRuntimeIdentity("v22.19.0", TEST_RUNTIME_EVIDENCE),
		},
		runtime,
		(frame) => frames.push(frame),
	);
	return { frames, runtime, host };
}

function lastFrameOfKind(frames: readonly OutboundFrame[], event: OutboundFrame["event"]): OutboundFrame | undefined {
	for (let index = frames.length - 1; index >= 0; index -= 1) {
		const frame = frames[index];
		if (frame?.event === event) return frame;
	}
	return undefined;
}

test("host: remains inert through AdapterReady and creates exactly one session only after CreateSession", async () => {
	const { frames, runtime, host } = makeHost();
	assert.equal(runtime.createCount, 0);
	assert.equal(frames.length, 1);
	assert.equal(frames[0]?.event, "AdapterReady");
	if (frames[0]?.event === "AdapterReady") assert.equal(frames[0].runtime.piSdkVersion, "0.83.0");
	assert.equal(resolvedInstalledPiSdkVersion(), "0.83.0");

	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload()));
	assert.equal(runtime.createCount, 1);
	assert.deepEqual(
		frames.map((frame) => frame.event),
		["AdapterReady", "CommandResult", "SessionReady"],
	);
	const ready = frames[2];
	assert.equal(ready?.event, "SessionReady");
	if (ready?.event === "SessionReady") {
		assert.equal(ready.configuration.settings.transport, "sse");
		assert.equal(ready.configuration.settings.projectTrust, "never");
		assert.equal(ready.configuration.settings.installTelemetryEnabled, false);
		assert.equal(ready.configuration.settings.analyticsEnabled, false);
		assert.equal(ready.configuration.settings.images, "blocked");
		assert.equal(ready.configuration.modelCatalog.catalogSha256, "6".repeat(64));
		assert.equal(ready.configuration.modelCatalog.effectiveModel.baseUrl, "https://openrouter.ai/api/v1");
		assert.equal(ready.configuration.modelCatalog.effectiveModel.cacheWriteUsdPerMillion.kind, "Absent");
	}
	await host.accept(decodeCommand(2, "GetState", {}));
	const state = frames.at(-2);
	assert.equal(state?.event, "CommandResult");
	if (state?.event === "CommandResult") {
		assert.deepEqual(state.detail, {
			kind: "state",
			phase: "Ready",
			liveness: "idle",
			sessionFile: "/tmp/xsh-society/session/fixture.jsonl",
		});
	}
});

test("host: serializes command admission while allowing office controls to reach a live Prompt", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("GrandArchitectOffice")));
	const session = runtime.session;
	assert.ok(session);

	await Promise.all([
		host.accept(decodeCommand(2, "Prompt", { purpose: "OfficeTurn", text: "initial Office turn" })),
		host.accept(decodeCommand(3, "FollowUp", { noticeDeliveryIdentity: "notice-001", ledgerFrontier: 42, text: "budget notice" })),
		host.accept(decodeCommand(4, "Steer", { reason: "UrgentUnsafePremise", text: "recheck containment" })),
		host.accept(decodeCommand(5, "Abort", { reason: "GracefulCancellation" })),
	]);
	assert.deepEqual(session.calls, [
		"Prompt:initial Office turn",
		"FollowUp:budget notice",
		"Steer:recheck containment",
		"Abort",
	]);

	const eventCountBeforeForensicUpdate = frames.filter((frame) => frame.event === "AgentEvent").length;
	session.emitForensicOnlyUpdate();
	assert.equal(frames.filter((frame) => frame.event === "AgentEvent").length, eventCountBeforeForensicUpdate + 1);
	const bashUpdate = frames.at(-1);
	assert.equal(bashUpdate?.event, "AgentEvent");
	if (bashUpdate?.event === "AgentEvent") {
		assert.deepEqual(bashUpdate.agentEvent, { type: "bash_execution_update", executionIdentity: "execution-1", delta: "expensive raw output" });
	}

	session.finishPrompt();
	await drainMicrotasks();
	const settled = lastFrameOfKind(frames, "Settled");
	assert.equal(settled?.event, "Settled");
	if (settled?.event === "Settled") assert.equal(settled.classification, "aborted");
	assert.equal(session.verifyCount, 1);

	await host.accept(decodeCommand(6, "Dispose", { reason: "CycleReconciliation" }));
	assert.equal(session.disposed, true);
	assert.equal(session.verifyCount, 2);
	assert.deepEqual(session.calls.slice(-2), ["Dispose", "VerifyCanonicalTranscript"]);
	const disposed = frames.at(-1);
	assert.equal(disposed?.event, "Disposed");
	if (disposed?.event === "Disposed") {
		assert.deepEqual(disposed.transcriptFlushReceipt, {
			format: "pi_session_manager_jsonl_v3",
			sessionIdentity: "pi-session-test-001",
			sessionFile: "/tmp/xsh-society/session/fixture.jsonl",
			materialization: "observed",
			sessionFileSha256: "5555555555555555555555555555555555555555555555555555555555555555",
			headerCwd: "/tmp/xsh-society/work",
			firstUserPrompt: { kind: "verified", digest: "f6054ca339dc2ec4dcfaf1977f1d0ec978eea809e8214f3e89b4f7a4d5d16de2" },
		});
	}
});

test("host: terminal Pi evidence closes narrative mutation and late Abort cannot relabel completion", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("GrandArchitectOffice")));
	const session = runtime.session;
	assert.ok(session);
	await host.accept(decodeCommand(2, "Prompt", { purpose: "OfficeTurn", text: "initial Office turn" }));
	session.deferCanonicalTranscriptVerification();
	session.finishPrompt("stop");
	await drainMicrotasks();
	assert.equal(session.verifyCount, 1, "Prompt completion is now waiting only on transcript verification");

	await host.accept(decodeCommand(3, "FollowUp", { noticeDeliveryIdentity: "notice-001", ledgerFrontier: 42, text: "too late" }));
	await host.accept(decodeCommand(4, "Steer", { reason: "UrgentUnsafePremise", text: "too late" }));
	await host.accept(decodeCommand(5, "Abort", { reason: "GracefulCancellation" }));
	assert.equal(session.calls.includes("FollowUp:too late"), false);
	assert.equal(session.calls.includes("Steer:too late"), false);
	assert.equal(session.calls.includes("Abort"), false);
	const lateResults = frames.filter(
		(frame) => frame.event === "CommandResult" && ["command-3", "command-4", "command-5"].includes(frame.correlationIdentity),
	);
	assert.equal(lateResults.length, 3);
	for (const result of lateResults.slice(0, 2)) {
		assert.equal(result.event, "CommandResult");
		if (result.event === "CommandResult") {
			assert.equal(result.accepted, false);
			assert.equal(result.failureCode, "invalid_state");
		}
	}
	const lateAbort = lateResults[2];
	assert.equal(lateAbort?.event, "CommandResult");
	if (lateAbort?.event === "CommandResult") assert.equal(lateAbort.accepted, true);

	session.releaseCanonicalTranscriptVerification();
	await drainMicrotasks();
	const settled = lastFrameOfKind(frames, "Settled");
	assert.equal(settled?.event, "Settled");
	if (settled?.event === "Settled") assert.equal(settled.classification, "completed");
});

test("host: a failed accepted Prompt has one CommandResult and a terminal failure", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	const session = runtime.session;
	assert.ok(session);
	await host.accept(decodeCommand(2, "Prompt", { purpose: "TaskAssignment", text: "task" }));
	session.failPrompt();
	await drainMicrotasks();

	const promptResults = frames.filter(
		(frame) => frame.event === "CommandResult" && frame.correlationIdentity === "command-2",
	);
	assert.equal(promptResults.length, 1);
	assert.equal(promptResults[0]?.event, "CommandResult");
	if (promptResults[0]?.event === "CommandResult") {
		assert.equal(promptResults[0].accepted, true);
		assert.equal("failureCode" in promptResults[0], false);
	}
	const settled = lastFrameOfKind(frames, "Settled");
	assert.equal(settled?.event, "Settled");
	if (settled?.event === "Settled") assert.equal(settled.classification, "failed");
});

test("host: a resolved SDK Prompt with final assistant error is not classified as completed", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	const session = runtime.session;
	assert.ok(session);
	await host.accept(decodeCommand(2, "Prompt", { purpose: "TaskAssignment", text: "task" }));
	session.finishPrompt("error");
	await drainMicrotasks();
	const settled = lastFrameOfKind(frames, "Settled");
	assert.equal(settled?.event, "Settled");
	if (settled?.event === "Settled") {
		assert.equal(settled.classification, "error");
		assert.deepEqual(settled.finalAssistantOutcome, { kind: "Observed", stopReason: "error" });
	}
	assert.equal(host.currentPhase, "Ready");
});

test("host: length and aborted final assistant outcomes remain distinct receipts", async () => {
	const length = makeHost();
	await length.host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	assert.ok(length.runtime.session);
	await length.host.accept(decodeCommand(2, "Prompt", { purpose: "TaskAssignment", text: "task" }));
	length.runtime.session.finishPrompt("length");
	await drainMicrotasks();
	const lengthSettled = lastFrameOfKind(length.frames, "Settled");
	assert.equal(lengthSettled?.event, "Settled");
	if (lengthSettled?.event === "Settled") assert.equal(lengthSettled.classification, "length");

	const aborted = makeHost();
	await aborted.host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	assert.ok(aborted.runtime.session);
	await aborted.host.accept(decodeCommand(2, "Prompt", { purpose: "TaskAssignment", text: "task" }));
	aborted.runtime.session.finishPrompt("aborted");
	await drainMicrotasks();
	const abortedSettled = lastFrameOfKind(aborted.frames, "Settled");
	assert.equal(abortedSettled?.event, "Settled");
	if (abortedSettled?.event === "Settled") assert.equal(abortedSettled.classification, "aborted");
});

test("host: a retry-era error cannot override the non-retried terminal assistant outcome", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	const session = runtime.session;
	assert.ok(session);
	await host.accept(decodeCommand(2, "Prompt", { purpose: "TaskAssignment", text: "task" }));
	session.finishPromptAfterRetriedError();
	await drainMicrotasks();
	const settled = lastFrameOfKind(frames, "Settled");
	assert.equal(settled?.event, "Settled");
	if (settled?.event === "Settled") {
		assert.equal(settled.classification, "completed");
		assert.deepEqual(settled.finalAssistantOutcome, { kind: "Observed", stopReason: "stop" });
	}
});

test("host: TaskAttempt admits exactly one task assignment even after it settles", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	const session = runtime.session;
	assert.ok(session);
	await host.accept(decodeCommand(2, "Prompt", { purpose: "TaskAssignment", text: "one shot" }));
	session.finishPrompt();
	await drainMicrotasks();
	await host.accept(decodeCommand(3, "Prompt", { purpose: "TaskAssignment", text: "forbidden second shot" }));
	assert.equal(session.calls.filter((call) => call.startsWith("Prompt:")).length, 1);
	const rejection = frames.at(-1);
	assert.equal(rejection?.event, "CommandResult");
	if (rejection?.event === "CommandResult") {
		assert.equal(rejection.accepted, false);
		assert.equal(rejection.failureCode, "invalid_state");
	}
});

test("host: Dispose before a Prompt is a valid, evidenced terminal path", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	await host.accept(decodeCommand(2, "Dispose", { reason: "ProcessRecovery" }));
	assert.equal(host.currentPhase, "Disposed");
	assert.equal(runtime.session?.disposed, true);
	const disposed = frames.at(-1);
	assert.equal(disposed?.event, "Disposed");
	if (disposed?.event === "Disposed") {
		assert.equal(disposed.transcriptFlushReceipt.materialization, "unmaterialized_no_prompt");
		assert.deepEqual(disposed.transcriptFlushReceipt.firstUserPrompt, { kind: "absent" });
	}
});

test("host: Office FollowUp and Steer wait for Pi to start the initial Prompt", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("GrandArchitectOffice")));
	const session = runtime.session;
	assert.ok(session);
	session.setPromptStartDelayed();
	await host.accept(decodeCommand(2, "Prompt", { purpose: "OfficeTurn", text: "initial Office turn" }));
	await host.accept(decodeCommand(3, "FollowUp", { noticeDeliveryIdentity: "notice-001", ledgerFrontier: 42, text: "too early" }));
	assert.equal(session.calls.includes("FollowUp:too early"), false);
	const earlyRejection = frames.at(-1);
	assert.equal(earlyRejection?.event, "CommandResult");
	if (earlyRejection?.event === "CommandResult" && !earlyRejection.accepted) assert.equal(earlyRejection.failureCode, "invalid_state");
	session.startPrompt();
	await host.accept(decodeCommand(4, "Steer", { reason: "UrgentUnsafePremise", text: "now legal" }));
	assert.equal(session.calls.includes("Steer:now legal"), true);
});

test("host: invalid SDK usage emits Unavailable then terminally contains the host", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	const session = runtime.session;
	assert.ok(session);
	session.makeUsageInvalid();
	await host.accept(decodeCommand(2, "GetState", {}));
	assert.equal(host.currentPhase, "Fatal");
	const unavailable = lastFrameOfKind(frames, "UsageSnapshot");
	assert.equal(unavailable?.event, "UsageSnapshot");
	if (unavailable?.event === "UsageSnapshot") assert.deepEqual(unavailable.usage, { kind: "Unavailable", reason: "invalid_sdk_usage" });
	assert.equal(frames.at(-1)?.event, "Fatal");
});

test("host: cumulative token or binary64 cost regression is drift, never a lower usage receipt", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	const session = runtime.session;
	assert.ok(session);
	// Establish the first observed cumulative receipt, then reproduce the
	// provider-side $0.02 -> $0.01 regression the host must contain.
	await host.accept(decodeCommand(2, "GetState", {}));
	session.setUsage({
		inputTokens: nonNegativeInteger(11),
		outputTokens: nonNegativeInteger(7),
		cacheReadTokens: nonNegativeInteger(3),
		cacheWriteTokens: nonNegativeInteger(2),
		totalTokens: nonNegativeInteger(23),
		providerCost: providerCostObservation(0.01),
	});
	await host.accept(decodeCommand(3, "GetState", {}));
	assert.equal(host.currentPhase, "Fatal");
	const unavailable = lastFrameOfKind(frames, "UsageSnapshot");
	assert.equal(unavailable?.event, "UsageSnapshot");
	if (unavailable?.event === "UsageSnapshot") assert.deepEqual(unavailable.usage, { kind: "Unavailable", reason: "usage_regressed" });
	const fatal = frames.at(-1);
	assert.equal(fatal?.event, "Fatal");
	if (fatal?.event === "Fatal") assert.equal(fatal.failureCode, "execution_profile_drift");
});

test("host: Pi total must equal every cumulative token bucket", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	const session = runtime.session;
	assert.ok(session);
	session.setUsage({
		inputTokens: nonNegativeInteger(11), outputTokens: nonNegativeInteger(7), cacheReadTokens: nonNegativeInteger(3), cacheWriteTokens: nonNegativeInteger(2),
		totalTokens: nonNegativeInteger(22), providerCost: providerCostObservation(0.0123456),
	});
	await host.accept(decodeCommand(2, "GetState", {}));
	const unavailable = lastFrameOfKind(frames, "UsageSnapshot");
	assert.equal(unavailable?.event, "UsageSnapshot");
	if (unavailable?.event === "UsageSnapshot") assert.deepEqual(unavailable.usage, { kind: "Unavailable", reason: "usage_inconsistent" });
	assert.equal(host.currentPhase, "Fatal");
});

test("host: assistant-message completion emits one incremental known usage observation", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	const session = runtime.session;
	assert.ok(session);
	session.emitAssistantMessageEnd();
	const snapshot = frames.at(-1);
	assert.equal(snapshot?.event, "UsageSnapshot");
	if (snapshot?.event === "UsageSnapshot") {
		assert.equal(snapshot.usage.kind, "Known");
		if (snapshot.usage.kind === "Known") assert.equal(snapshot.usage.totals.totalTokens, 23);
	}
	assert.equal(host.currentPhase, "Ready");
});

test("host: any assistant response-model drift is terminal", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	const session = runtime.session;
	assert.ok(session);
	session.emitAssistantResponseModelDrift();
	assert.equal(host.currentPhase, "Fatal");
	const fatal = frames.at(-1);
	assert.equal(fatal?.event, "Fatal");
	if (fatal?.event === "Fatal") assert.equal(fatal.failureCode, "execution_profile_drift");
});

test("host: a sequence gap is terminal and creates no SDK session", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(2, "CreateSession", createSessionPayload()));
	assert.equal(runtime.createCount, 0);
	const rejection = frames.at(-2);
	assert.equal(rejection?.event, "CommandResult");
	if (rejection?.event === "CommandResult") {
		assert.equal(rejection.accepted, false);
		assert.deepEqual(rejection.detail, { kind: "rejected" });
		assert.equal(rejection.failureCode, "sequence_gap");
	}
	assert.equal(frames.at(-1)?.event, "Fatal");
	assert.equal(host.currentPhase, "Fatal");
});

test("host: EOF awaits a Creating admission before containing the constructed session", async () => {
	const frames: OutboundFrame[] = [];
	const runtime = new DeferredSdkRuntime();
	const host = new PiSdkHost(
		{
			sessionIdentity: sessionIdentity("pi-session-test-001"),
			spawnNonce: spawnNonce("spawn-test-001"),
			pid: 4242,
			runtime: localRuntimeIdentity("v22.19.0", TEST_RUNTIME_EVIDENCE),
		},
		runtime,
		(frame) => frames.push(frame),
	);
	const creating = host.accept(decodeCommand(1, "CreateSession", createSessionPayload()));
	await drainMicrotasks();
	assert.equal(host.currentPhase, "Creating");
	const eof = host.onControlPipeEof();
	assert.equal(host.currentPhase, "Creating");
	runtime.completeCreate();
	await creating;
	await eof;
	assert.equal(host.currentPhase, "Fatal");
	assert.equal(runtime.session?.disposed, true);
	assert.equal(frames.at(-1)?.event, "Fatal");
});

test("host: a detected Pi profile mutation is terminal drift, never ordinary provenance", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	const session = runtime.session;
	assert.ok(session);
	session.emitThinkingLevelMutation();
	assert.equal(host.currentPhase, "Fatal");
	const fatal = frames.at(-1);
	assert.equal(fatal?.event, "Fatal");
	if (fatal?.event === "Fatal") assert.equal(fatal.failureCode, "execution_profile_drift");
	assert.equal(frames.some((frame) => frame.event === "AgentEvent"), true);
});

test("host: a persisted Pi model-change entry is execution-profile drift", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	const session = runtime.session;
	assert.ok(session);
	session.emitPersistedModelMutation();
	assert.equal(host.currentPhase, "Fatal");
	const fatal = frames.at(-1);
	assert.equal(fatal?.event, "Fatal");
	if (fatal?.event === "Fatal") assert.equal(fatal.failureCode, "execution_profile_drift");
});

test("host: control-pipe EOF during a live prompt emits one terminal Fatal and no late settlement", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	const session = runtime.session;
	assert.ok(session);
	await host.accept(decodeCommand(2, "Prompt", { purpose: "TaskAssignment", text: "task" }));
	await host.onControlPipeEof();
	session.finishPrompt();
	await drainMicrotasks();

	assert.equal(host.currentPhase, "Fatal");
	assert.equal(session.disposed, true);
	const fatalIndex = frames.findIndex((frame) => frame.event === "Fatal");
	assert.notEqual(fatalIndex, -1);
	assert.equal(frames.slice(fatalIndex + 1).some((frame) => frame.event === "Settled"), false);
});

test("host: outbound transport failure disposes the owned session and fences later Prompt admission", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("TaskAttempt")));
	const session = runtime.session;
	assert.ok(session);
	const beforeFailureFrames = frames.length;
	host.outboundTransportFailed();
	assert.equal(host.currentPhase, "Fatal");
	assert.equal(session.disposed, true);
	// stdout is no longer a legal observation surface, so no recursive Fatal is
	// written and no later command can begin a potentially paid model turn.
	assert.equal(frames.length, beforeFailureFrames);
	await host.accept(decodeCommand(2, "Prompt", { purpose: "TaskAssignment", text: "must not execute" }));
	assert.equal(session.calls.some((call) => call.startsWith("Prompt:")), false);
});

test("host: missing agent settlement is terminal evidence failure and fences a second Office Prompt", async () => {
	const { frames, runtime, host } = makeHost();
	await host.accept(decodeCommand(1, "CreateSession", createSessionPayload("GrandArchitectOffice")));
	const session = runtime.session;
	assert.ok(session);
	await host.accept(decodeCommand(2, "Prompt", { purpose: "OfficeTurn", text: "first" }));
	session.resolvePromptWithoutTerminalEvidence();
	await drainMicrotasks();
	assert.equal(host.currentPhase, "Fatal");
	assert.equal(session.disposed, true);
	const settled = lastFrameOfKind(frames, "Settled");
	assert.equal(settled?.event, "Settled");
	if (settled?.event === "Settled") {
		assert.equal(settled.classification, "protocol_failed");
		assert.deepEqual(settled.finalAssistantOutcome, { kind: "Unavailable", reason: "missing_final_assistant_outcome" });
	}
	await host.accept(decodeCommand(3, "Prompt", { purpose: "OfficeTurn", text: "forbidden second" }));
	assert.equal(session.calls.filter((call) => call.startsWith("Prompt:")).length, 1);
});
