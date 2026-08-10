import assert from "node:assert/strict";
import test from "node:test";

import { blake3Hex } from "../src/digest.js";
import {
	FORUM_ENABLED_TOOL_CONTRACT,
	FORUM_F0_AWARENESS_BLAKE3,
	FORUM_F0_AWARENESS_TEXT,
	FORUM_F0_TOOL_CONTRACT_BLAKE3,
	FORUM_F0_TOOL_CONTRACT_TEXT,
	SEQUESTERED_TOOL_CONTRACT,
	forumF0AwarenessBytes,
	forumToolContractDescriptor,
} from "../src/forum.js";

test("forum F0 awareness is exact UTF-8 and has the registered digest", () => {
	const bytes = forumF0AwarenessBytes();
	assert.deepEqual(Buffer.from(bytes), Buffer.from(FORUM_F0_AWARENESS_TEXT, "utf8"));
	assert.equal(blake3Hex(bytes), FORUM_F0_AWARENESS_BLAKE3);
	assert.equal(blake3Hex(FORUM_F0_AWARENESS_TEXT), FORUM_F0_AWARENESS_BLAKE3);
	assert.equal(blake3Hex(FORUM_F0_TOOL_CONTRACT_TEXT), FORUM_F0_TOOL_CONTRACT_BLAKE3);
	assert.equal(FORUM_F0_AWARENESS_BLAKE3, "c2db53f69595a724b745a3b0ccbee710b70ebea4b2cc06dfff902bd7d3e886ea");
	assert.equal(FORUM_F0_TOOL_CONTRACT_BLAKE3, "738e664f66be09dfb7f8e5e4873521d7b9f1600d385dd0c8a41c80ca087566be");

	// The exported accessor is a copy, not mutable shared prompt state.
	const changed = forumF0AwarenessBytes();
	const first = changed[0];
	if (first === undefined) throw new Error("sealed awareness unexpectedly empty");
	changed[0] = first ^ 0xff;
	assert.equal(blake3Hex(forumF0AwarenessBytes()), FORUM_F0_AWARENESS_BLAKE3);
});

test("Forum-enabled and Sequestered descriptors are closed and disjoint", () => {
	assert.deepEqual(FORUM_ENABLED_TOOL_CONTRACT, forumToolContractDescriptor("forum_enabled_v1"));
	assert.deepEqual(SEQUESTERED_TOOL_CONTRACT, forumToolContractDescriptor("sequestered_v1"));
	assert.deepEqual(FORUM_ENABLED_TOOL_CONTRACT.tools, ["society_forum_read", "society_forum_post"]);
	assert.equal(FORUM_ENABLED_TOOL_CONTRACT.awarenessText, FORUM_F0_AWARENESS_TEXT);
	assert.equal(FORUM_ENABLED_TOOL_CONTRACT.awarenessBlake3, FORUM_F0_AWARENESS_BLAKE3);
	assert.equal(FORUM_ENABLED_TOOL_CONTRACT.toolContractBlake3, FORUM_F0_TOOL_CONTRACT_BLAKE3);
	assert.deepEqual(SEQUESTERED_TOOL_CONTRACT.tools, []);
	assert.equal(SEQUESTERED_TOOL_CONTRACT.awarenessText, undefined);
	assert.equal(SEQUESTERED_TOOL_CONTRACT.awarenessBlake3, undefined);
	assert.equal(SEQUESTERED_TOOL_CONTRACT.toolContractBlake3, undefined);
	assert.equal(Object.isFrozen(FORUM_ENABLED_TOOL_CONTRACT), true);
	assert.equal(Object.isFrozen(FORUM_ENABLED_TOOL_CONTRACT.tools), true);
	assert.equal(Object.isFrozen(SEQUESTERED_TOOL_CONTRACT), true);
	assert.equal(Object.isFrozen(SEQUESTERED_TOOL_CONTRACT.tools), true);

	// A descriptor contains policy metadata only; no mutable Forum Message
	// fields or body can cross the SDK boundary through this contract.
	assert.deepEqual(Object.keys(FORUM_ENABLED_TOOL_CONTRACT).sort(), [
		"awarenessBlake3", "awarenessText", "kind", "revision", "toolContractBlake3", "tools",
	]);
	assert.deepEqual(Object.keys(SEQUESTERED_TOOL_CONTRACT).sort(), ["awarenessBlake3", "awarenessText", "kind", "revision", "toolContractBlake3", "tools"]);
});
