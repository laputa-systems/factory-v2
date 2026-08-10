/**
 * The host is the evidence boundary, not the provenance curator. It preserves
 * every current Pi 0.83 AgentSessionEvent in a closed, JSON-safe union; Rust
 * decides which facts become curated operational influence and which remain
 * sealed forensic evidence.
 */

import type { AgentSessionEvent } from "@earendil-works/pi-coding-agent";

import type { ProjectedAgentEvent, SdkJsonValue } from "./protocol.js";

export function projectAgentSessionEvent(event: AgentSessionEvent): ProjectedAgentEvent {
	switch (event.type) {
		case "agent_start":
			return { type: "agent_start" };
		case "agent_end":
			return { type: "agent_end", messages: event.messages.map(toSdkAgentMessage), willRetry: event.willRetry };
		case "agent_settled":
			return { type: "agent_settled" };
		case "turn_start":
			return { type: "turn_start" };
		case "turn_end":
			return { type: "turn_end", message: toSdkAgentMessage(event.message), toolResults: event.toolResults.map(toSdkAgentMessage) };
		case "message_start":
			return { type: "message_start", message: toSdkAgentMessage(event.message) };
		case "message_update":
			return {
				type: "message_update",
				message: toSdkAgentMessage(event.message),
				assistantMessageEvent: toSdkJsonValueWithOptionalMembers(event.assistantMessageEvent),
			};
		case "message_end":
			return { type: "message_end", message: toSdkAgentMessage(event.message) };
		case "tool_execution_start":
			return { type: "tool_execution_start", toolCallIdentity: event.toolCallId, toolName: event.toolName, args: toSdkJsonValue(event.args) };
		case "tool_execution_update":
			return {
				type: "tool_execution_update",
				toolCallIdentity: event.toolCallId,
				toolName: event.toolName,
				args: toSdkJsonValue(event.args),
				partialResult: toSdkJsonValue(event.partialResult),
			};
		case "tool_execution_end":
			return {
				type: "tool_execution_end",
				toolCallIdentity: event.toolCallId,
				toolName: event.toolName,
				result: toSdkJsonValue(event.result),
				isError: event.isError,
			};
		case "queue_update":
			return { type: "queue_update", steering: [...event.steering], followUp: [...event.followUp] };
		case "entry_appended":
			return { type: "entry_appended", entry: toSdkJsonValue(event.entry) };
		case "bash_execution_update":
			return event.id === undefined
				? { type: "bash_execution_update", delta: event.delta }
				: { type: "bash_execution_update", executionIdentity: event.id, delta: event.delta };
		case "compaction_start":
			return { type: "compaction_start", reason: event.reason };
		case "session_info_changed":
			return event.name === undefined ? { type: "session_info_changed" } : { type: "session_info_changed", name: event.name };
		case "thinking_level_changed":
			return { type: "thinking_level_changed", level: event.level };
		case "compaction_end":
			return {
				type: "compaction_end",
				reason: event.reason,
				...(event.result === undefined ? {} : { result: toSdkJsonValue(event.result) }),
				aborted: event.aborted,
				willRetry: event.willRetry,
				...(event.errorMessage === undefined ? {} : { errorMessage: event.errorMessage }),
			};
		case "auto_retry_start":
			return {
				type: "auto_retry_start",
				attempt: event.attempt,
				maxAttempts: event.maxAttempts,
				delayMilliseconds: event.delayMs,
				errorMessage: event.errorMessage,
			};
		case "auto_retry_end":
			return event.finalError === undefined
				? { type: "auto_retry_end", success: event.success, attempt: event.attempt }
				: { type: "auto_retry_end", success: event.success, attempt: event.attempt, finalError: event.finalError };
		case "summarization_retry_scheduled":
			return {
				type: "summarization_retry_scheduled",
				attempt: event.attempt,
				maxAttempts: event.maxAttempts,
				delayMilliseconds: event.delayMs,
				errorMessage: event.errorMessage,
			};
		case "summarization_retry_attempt_start":
			return event.source === "branchSummary"
				? { type: "summarization_retry_attempt_start", source: "branchSummary" }
				: { type: "summarization_retry_attempt_start", source: "compaction", reason: event.reason };
		case "summarization_retry_finished":
			return { type: "summarization_retry_finished" };
		default:
			return assertNever(event);
	}
}

/** Detect post-construction mutations which the V2 host never authorizes. */
export function isExecutionProfileMutation(event: AgentSessionEvent): boolean {
	switch (event.type) {
		case "session_info_changed":
		case "thinking_level_changed":
			return true;
		case "entry_appended":
			switch (event.entry.type) {
				case "model_change":
				case "thinking_level_change":
				case "session_info":
					return true;
				case "message":
				case "compaction":
				case "branch_summary":
				case "custom":
				case "custom_message":
				case "label":
					return false;
				default:
					return assertNever(event.entry);
			}
		default:
			return false;
	}
}

function toSdkJsonValue(value: unknown): SdkJsonValue {
	return validateSdkJsonValue(value, new Set<object>());
}

function toSdkAgentMessage(value: unknown): SdkJsonValue {
	if (typeof value === "object" && value !== null && !Array.isArray(value)) {
		const role = (value as { readonly role?: unknown }).role;
		if (role === "user" || role === "assistant" || role === "toolResult") return toSdkJsonValueWithOptionalMembers(value);
	}
	return toSdkJsonValue(value);
}

/**
 * Pi's typed streaming messages contain optional fields which some providers
 * materialize as own properties with `undefined` values. They have no JSON
 * representation, so omit only those typed SDK members while retaining the
 * strict validator for general evidence and all array values.
 */
function toSdkJsonValueWithOptionalMembers(value: unknown): SdkJsonValue {
	return validateSdkJsonValue(value, new Set<object>(), true);
}

/**
 * Pi event evidence is projected field-for-field, but never through
 * JSON.stringify/parse. That shortcut silently turns non-finite numbers into
 * null, omits undefined object members, and can make a malformed SDK object
 * look admissible. The adapter instead accepts the closed JSON subset only.
 */
function validateSdkJsonValue(value: unknown, ancestors: Set<object>, omitUndefinedObjectMembers = false): SdkJsonValue {
	if (value === null || typeof value === "boolean" || typeof value === "string") return value;
	if (typeof value === "number") {
		if (!Number.isFinite(value) || Object.is(value, -0)) throw new Error("sdk_value_not_json_safe");
		return value;
	}
	if (Array.isArray(value)) {
		assertCanonicalJsonArray(value);
		if (ancestors.has(value)) throw new Error("sdk_value_not_json_safe");
		ancestors.add(value);
		try {
			const output: SdkJsonValue[] = [];
			for (let index = 0; index < value.length; index += 1) {
				output.push(validateSdkJsonValue(value[index], ancestors, omitUndefinedObjectMembers));
			}
			return output;
		} finally {
			ancestors.delete(value);
		}
	}
	if (typeof value !== "object" || value === undefined) throw new Error("sdk_value_not_json_safe");
	if (ancestors.has(value)) throw new Error("sdk_value_not_json_safe");
	const prototype = Object.getPrototypeOf(value);
	if (prototype !== Object.prototype && prototype !== null || Object.getOwnPropertySymbols(value).length !== 0) {
		throw new Error("sdk_value_not_json_safe");
	}
	ancestors.add(value);
	try {
		// A normal `{}` turns an own `__proto__` evidence field into prototype
		// mutation on assignment and JSON.stringify then loses it. SDK evidence
		// may legitimately contain any JSON object key, so use a null dictionary.
		const propertyNames = Object.getOwnPropertyNames(value);
		const enumerableKeys = Object.keys(value);
		if (propertyNames.length !== enumerableKeys.length) throw new Error("sdk_value_not_json_safe");
		const output: Record<string, SdkJsonValue> = Object.create(null) as Record<string, SdkJsonValue>;
		for (const key of propertyNames) {
			const descriptor = Object.getOwnPropertyDescriptor(value, key);
			if (descriptor === undefined || !descriptor.enumerable || !("value" in descriptor)) {
				throw new Error("sdk_value_not_json_safe");
			}
			if (descriptor.value === undefined) {
				if (omitUndefinedObjectMembers) continue;
				throw new Error("sdk_value_not_json_safe");
			}
			output[key] = validateSdkJsonValue(descriptor.value, ancestors, omitUndefinedObjectMembers);
		}
		return output;
	} finally {
		ancestors.delete(value);
	}
}

/** Arrays cannot carry hidden object fields, holes, or accessors across JSON. */
function assertCanonicalJsonArray(value: readonly unknown[]): void {
	if (Object.getPrototypeOf(value) !== Array.prototype) throw new Error("sdk_value_not_json_safe");
	if (Object.getOwnPropertySymbols(value).length !== 0) throw new Error("sdk_value_not_json_safe");
	const propertyNames = Object.getOwnPropertyNames(value);
	if (propertyNames.length !== value.length + 1 || !propertyNames.includes("length")) {
		throw new Error("sdk_value_not_json_safe");
	}
	const lengthDescriptor = Object.getOwnPropertyDescriptor(value, "length");
	if (lengthDescriptor === undefined || !("value" in lengthDescriptor) || lengthDescriptor.enumerable) {
		throw new Error("sdk_value_not_json_safe");
	}
	for (let index = 0; index < value.length; index += 1) {
		const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
		if (descriptor === undefined || !("value" in descriptor) || descriptor.value === undefined) {
			throw new Error("sdk_value_not_json_safe");
		}
	}
}

function assertNever(value: never): never {
	throw new Error(`unhandled_pi_sdk_variant:${String(value)}`);
}
