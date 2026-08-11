import assert from "node:assert/strict";
import test from "node:test";

import type { AgentSessionEvent } from "@earendil-works/pi-coding-agent";

import { projectAgentSessionEvent } from "../src/event-projection.js";

test("event projection: every Pi 0.84 event variant has a closed field-preserving JSON-safe projection", () => {
	const marker = { nested: { exact: "preserved" }, list: [1, true, null] };
	const fixtures: readonly AgentSessionEvent[] = [
		{ type: "agent_start" },
		{ type: "agent_end", messages: [marker], willRetry: false },
		{ type: "agent_settled" },
		{ type: "turn_start" },
		{ type: "turn_end", message: marker, toolResults: [marker] },
		{ type: "message_start", message: marker },
		{ type: "message_update", message: marker, assistantMessageEvent: marker },
		{ type: "message_end", message: marker },
		{ type: "tool_execution_start", toolCallId: "call-1", toolName: "read", args: marker },
		{ type: "tool_execution_update", toolCallId: "call-1", toolName: "read", args: marker, partialResult: marker },
		{ type: "tool_execution_end", toolCallId: "call-1", toolName: "read", result: marker, isError: false },
		{ type: "queue_update", steering: ["urgent"], followUp: ["notice"] },
		{ type: "entry_appended", entry: marker },
		{ type: "bash_execution_update", id: "bash-1", delta: "raw delta" },
		{ type: "compaction_start", reason: "threshold" },
		{ type: "session_info_changed", name: "fixed" },
		{ type: "thinking_level_changed", level: "high" },
		{ type: "compaction_end", reason: "manual", result: marker, aborted: false, willRetry: false },
		{ type: "auto_retry_start", attempt: 1, maxAttempts: 2, delayMs: 2000, errorMessage: "retry" },
		{ type: "auto_retry_end", success: true, attempt: 1 },
		{ type: "summarization_retry_scheduled", attempt: 1, maxAttempts: 2, delayMs: 2000, errorMessage: "retry" },
		{ type: "summarization_retry_attempt_start", source: "branchSummary" },
		{ type: "summarization_retry_attempt_start", source: "compaction", reason: "overflow" },
		{ type: "summarization_retry_finished" },
	] as unknown as readonly AgentSessionEvent[];

	const projected = fixtures.map(projectAgentSessionEvent);
	assert.deepEqual(projected.map((event) => event.type), [
		"agent_start", "agent_end", "agent_settled", "turn_start", "turn_end", "message_start", "message_update", "message_end",
		"tool_execution_start", "tool_execution_update", "tool_execution_end", "queue_update", "entry_appended", "bash_execution_update",
		"compaction_start", "session_info_changed", "thinking_level_changed", "compaction_end", "auto_retry_start", "auto_retry_end",
		"summarization_retry_scheduled", "summarization_retry_attempt_start", "summarization_retry_attempt_start", "summarization_retry_finished",
	]);
	assert.deepEqual(jsonClone(projected[1]), { type: "agent_end", messages: [marker], willRetry: false });
	assert.deepEqual(jsonClone(projected[6]), { type: "message_update", message: marker, assistantMessageEvent: marker });
	assert.deepEqual(jsonClone(projected[9]), { type: "tool_execution_update", toolCallIdentity: "call-1", toolName: "read", args: marker, partialResult: marker });
	assert.deepEqual(jsonClone(projected[12]), { type: "entry_appended", entry: marker });
});

function jsonClone<T>(value: T): unknown {
	return JSON.parse(JSON.stringify(value));
}

test("event projection: unsafe SDK values are rejected instead of JSON-normalized", () => {
	assert.throws(() => projectAgentSessionEvent({ type: "message_start", message: { nonFinite: Number.NaN } } as unknown as AgentSessionEvent));
	assert.throws(() => projectAgentSessionEvent({ type: "message_start", message: { omitted: undefined } } as unknown as AgentSessionEvent));
	const circular: { self?: unknown } = {};
	circular.self = circular;
	assert.throws(() => projectAgentSessionEvent({ type: "message_start", message: circular } as unknown as AgentSessionEvent));
});

test("event projection: dangerous-looking JSON keys survive projection and final JSON encoding", () => {
	const evidence = JSON.parse('{"__proto__":{"constructor":{"prototype":"preserved"}},"constructor":"outer","prototype":"outer-prototype"}') as unknown;
	const projected = projectAgentSessionEvent({ type: "message_start", message: evidence } as unknown as AgentSessionEvent);
	assert.equal(projected.type, "message_start");
	if (projected.type !== "message_start") return;
	const encoded = JSON.parse(JSON.stringify(projected)) as { message: Record<string, unknown> };
	assert.deepEqual(encoded.message, JSON.parse('{"__proto__":{"constructor":{"prototype":"preserved"}},"constructor":"outer","prototype":"outer-prototype"}'));
});

test("event projection: arrays with holes, extra keys, accessors, or symbols are rejected rather than normalized", () => {
	const sparse = new Array<unknown>(2);
	sparse[1] = "present";
	assert.throws(() => projectAgentSessionEvent({ type: "message_start", message: sparse } as unknown as AgentSessionEvent));
	const extra: unknown[] = ["present"];
	Object.defineProperty(extra, "forensic_only", { value: "lost by JSON", enumerable: true });
	assert.throws(() => projectAgentSessionEvent({ type: "message_start", message: extra } as unknown as AgentSessionEvent));
	const accessor: unknown[] = ["placeholder"];
	Object.defineProperty(accessor, "0", { get: () => "computed", enumerable: true, configurable: true });
	assert.throws(() => projectAgentSessionEvent({ type: "message_start", message: accessor } as unknown as AgentSessionEvent));
	const symbolic: unknown[] = ["present"];
	Object.defineProperty(symbolic, Symbol("forensic_only"), { value: "lost by JSON", enumerable: true });
	assert.throws(() => projectAgentSessionEvent({ type: "message_start", message: symbolic } as unknown as AgentSessionEvent));
	class EvidenceArray extends Array<unknown> {
		toJSON() { return ["rewritten"]; }
	}
	assert.throws(() => projectAgentSessionEvent({ type: "message_start", message: new EvidenceArray("present") } as unknown as AgentSessionEvent));
});

test("event projection: JSON-altering negative zero and hidden object fields are rejected", () => {
	assert.throws(() => projectAgentSessionEvent({ type: "message_start", message: { negativeZero: -0 } } as unknown as AgentSessionEvent));
	const hidden = { visible: "preserved" };
	Object.defineProperty(hidden, "hidden", { value: "lost by JSON", enumerable: false });
	assert.throws(() => projectAgentSessionEvent({ type: "message_start", message: hidden } as unknown as AgentSessionEvent));
});
