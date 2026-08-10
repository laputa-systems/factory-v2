import assert from "node:assert/strict";
import test from "node:test";

import {
	absolutePath,
	binary64BigEndianHex,
	decodeInboundJsonl,
	nodeRuntimeVersion,
	providerCostObservation,
	ProtocolDecodeError,
} from "../src/protocol.js";
import { createSessionPayload, decodeCommand } from "./support.js";

test("protocol: decodes the complete pinned CreateSession profile and rejects ambient fields", () => {
	const command = decodeCommand(1, "CreateSession", createSessionPayload());
	assert.equal(command.command, "CreateSession");
	if (command.command !== "CreateSession") return;
	assert.equal(command.payload.model.provider, "openrouter");
	assert.equal(command.payload.model.modelId, "deepseek/deepseek-v4-flash-0731");
	assert.equal(command.payload.settings.transport, "sse");
	assert.equal(command.payload.settings.retry.baseDelayMilliseconds, 2_000);
	assert.equal(command.payload.settings.retry.providerTimeoutMilliseconds, 300_000);
	assert.equal(command.payload.settings.retry.providerMaxRetryDelayMilliseconds, 30_000);
	assert.equal(command.payload.settings.projectTrust, "never");
	assert.equal(command.payload.settings.images, "blocked");
	assert.deepEqual(command.payload.forumContract, {
		kind: "forum_enabled_v1",
		awarenessBlake3: "c2db53f69595a724b745a3b0ccbee710b70ebea4b2cc06dfff902bd7d3e886ea",
		toolContractBlake3: "738e664f66be09dfb7f8e5e4873521d7b9f1600d385dd0c8a41c80ca087566be",
	});

	const injected = createSessionPayload();
	const settings = injected.settings as Record<string, unknown>;
	settings.ambientDiscovery = true;
	assert.throws(() => decodeCommand(1, "CreateSession", injected), ProtocolDecodeError);

	const retryDrift = createSessionPayload();
	((retryDrift.settings as Record<string, unknown>).retry as Record<string, unknown>).baseDelayMilliseconds = 100;
	assert.throws(() => decodeCommand(1, "CreateSession", retryDrift), ProtocolDecodeError);
});

test("protocol: admits the closed workspace-isolated runner profile", () => {
	const isolated = createSessionPayload();
	isolated.toolProfile = "workspace_isolated_v1";
	isolated.forumContract = { kind: "sequestered_v1" };
	const command = decodeCommand(1, "CreateSession", isolated);
	assert.equal(command.command, "CreateSession");
	if (command.command === "CreateSession") assert.equal(command.payload.toolProfile, "workspace_isolated_v1");
});

test("protocol: admits each saved OpenRouter model while keeping model and catalog paired", () => {
	for (const [modelId, canonicalSlug, contextWindow, maxTokens, inputCost, outputCost, cacheReadCost] of [
		["inclusionai/ling-3.0-tiny:free", "inclusionai/ling-3.0-tiny:free", 262_144, 32_768, "0", "0", "0"],
		["poolside/laguna-xs-2.1:free", "poolside/laguna-xs-2.1:free", 262_144, 32_768, "0", "0", "0"],
		["inclusionai/ling-2.6-flash", "inclusionai/ling-2.6-flash", 262_144, 32_768, "0.01", "0.03", "0.002"],
	] as const) {
		const payload = createSessionPayload();
		(payload.model as Record<string, unknown>).modelId = modelId;
		(payload.model as Record<string, unknown>).thinkingLevel = modelId === "inclusionai/ling-2.6-flash" ? "off" : "high";
		const catalog = payload.modelCatalog as Record<string, unknown>;
		const effectiveModel = catalog.effectiveModel as Record<string, unknown>;
		effectiveModel.modelId = modelId;
		effectiveModel.canonicalSlug = canonicalSlug;
		effectiveModel.contextWindow = contextWindow;
		effectiveModel.maxTokens = maxTokens;
		effectiveModel.inputUsdPerMillion = { kind: "Known", usdPerMillion: inputCost };
		effectiveModel.outputUsdPerMillion = { kind: "Known", usdPerMillion: outputCost };
		effectiveModel.cacheReadUsdPerMillion = { kind: "Known", usdPerMillion: cacheReadCost };
		const command = decodeCommand(1, "CreateSession", payload);
		assert.equal(command.command, "CreateSession");
		if (command.command === "CreateSession") assert.equal(command.payload.model.modelId, modelId);
	}
});

test("protocol: workspace-isolated profile cannot claim Forum tools it does not install", () => {
	const invalid = createSessionPayload();
	invalid.toolProfile = "workspace_isolated_v1";
	assert.throws(() => decodeCommand(1, "CreateSession", invalid), ProtocolDecodeError);
});

test("protocol: Forum session contract rejects digest drift and invalid pairings", () => {
	const awarenessDrift = createSessionPayload();
	(awarenessDrift.forumContract as Record<string, unknown>).awarenessBlake3 = "0".repeat(64);
	assert.throws(() => decodeCommand(1, "CreateSession", awarenessDrift), ProtocolDecodeError);

	const toolDrift = createSessionPayload();
	(toolDrift.forumContract as Record<string, unknown>).toolContractBlake3 = "1".repeat(64);
	assert.throws(() => decodeCommand(1, "CreateSession", toolDrift), ProtocolDecodeError);

	const enabledAsSequestered = createSessionPayload();
	(enabledAsSequestered.forumContract as Record<string, unknown>).kind = "sequestered_v1";
	assert.throws(() => decodeCommand(1, "CreateSession", enabledAsSequestered), ProtocolDecodeError);

	const sequesteredWithDigest = createSessionPayload();
	sequesteredWithDigest.forumContract = { kind: "sequestered_v1", awarenessBlake3: "0".repeat(64) };
	assert.throws(() => decodeCommand(1, "CreateSession", sequesteredWithDigest), ProtocolDecodeError);
});

test("protocol: rejects noncanonical paths, model drift, and nonempty GetState payloads", () => {
	const relative = createSessionPayload();
	relative.cwd = "relative/workspace";
	assert.throws(() => decodeCommand(1, "CreateSession", relative), ProtocolDecodeError);
	assert.throws(() => absolutePath("/tmp/society/../escape"), ProtocolDecodeError);
	assert.throws(() => absolutePath("/tmp/society//double-separator"), ProtocolDecodeError);
	assert.throws(() => absolutePath("/tmp/society/trailing/"), ProtocolDecodeError);
	assert.throws(() => absolutePath("/tmp/society/\0poison"), ProtocolDecodeError);
	assert.equal(absolutePath("/tmp/society/normalized"), "/tmp/society/normalized");
	const trailingCreatePath = createSessionPayload();
	trailingCreatePath.cwd = "/tmp/society-host-fixture/work/";
	assert.throws(() => decodeCommand(1, "CreateSession", trailingCreatePath), ProtocolDecodeError);

	const modelDrift = createSessionPayload();
	(modelDrift.model as Record<string, unknown>).modelId = "inclusionai/other";
	assert.throws(() => decodeCommand(1, "CreateSession", modelDrift), ProtocolDecodeError);

	assert.throws(() => decodeCommand(1, "GetState", { arbitrary: "no" }), ProtocolDecodeError);
});

test("protocol: rejects every lexical JSON negative-zero form before integer admission", () => {
	for (const spelling of ["-0", "-0.0", "-0e0", "-0E+10"]) {
		const line = `{"protocolVersion":"society-pi-host/v4","sequence":1,"sessionIdentity":"pi-session-test-001","correlationIdentity":"command-1","command":"FollowUp","payload":{"noticeDeliveryIdentity":"notice-001","ledgerFrontier":${spelling},"text":"notice"}}`;
		assert.throws(() => decodeInboundJsonl(line), ProtocolDecodeError, spelling);
	}
	const nonzero = '{"protocolVersion":"society-pi-host/v4","sequence":1,"sessionIdentity":"pi-session-test-001","correlationIdentity":"command-1","command":"FollowUp","payload":{"noticeDeliveryIdentity":"notice-001","ledgerFrontier":1,"text":"notice"}}';
	assert.equal(decodeInboundJsonl(nonzero).command, "FollowUp");
});

test("protocol: preserves provider binary64 cost observations for Rust to round upward", () => {
	const zero = providerCostObservation(0);
	const tenthMicroUsd = providerCostObservation(0.0000001);
	const oneMicroUsd = providerCostObservation(0.000001);
	const justAboveOneMicroUsd = providerCostObservation(0.000001000001);
	assert.deepEqual(zero, {
		encoding: "ieee754_binary64_be_hex_v1",
		binary64BigEndianHex: "0000000000000000",
		rounding: "ceil_to_micro_usd",
	});
	assert.notEqual(tenthMicroUsd.binary64BigEndianHex, zero.binary64BigEndianHex);
	assert.equal(tenthMicroUsd.rounding, "ceil_to_micro_usd");
	assert.equal(oneMicroUsd.binary64BigEndianHex, binary64BigEndianHex(0.000001));
	assert.notEqual(justAboveOneMicroUsd.binary64BigEndianHex, oneMicroUsd.binary64BigEndianHex);
	assert.equal(ceilingMicroUsdForBoundaryTest(tenthMicroUsd.binary64BigEndianHex), 1);
	assert.equal(ceilingMicroUsdForBoundaryTest(oneMicroUsd.binary64BigEndianHex), 1);
	assert.equal(ceilingMicroUsdForBoundaryTest(justAboveOneMicroUsd.binary64BigEndianHex), 2);
	assert.throws(() => providerCostObservation(Number.NaN), ProtocolDecodeError);
	assert.throws(() => nodeRuntimeVersion("v22.18.0"), ProtocolDecodeError);
	assert.equal(nodeRuntimeVersion("v22.19.0"), "v22.19.0");

	const malformed = JSON.stringify({ protocolVersion: "society-pi-host/v4" });
	assert.throws(() => decodeInboundJsonl(malformed), ProtocolDecodeError);
});

test("protocol: rejects duplicate object keys before JSON.parse can select the last value", () => {
	const frame = JSON.stringify({
		protocolVersion: "society-pi-host/v4", sequence: 1, sessionIdentity: "pi-session-test-001",
		correlationIdentity: "command-1", command: "GetState", payload: {},
	});
	const duplicateTopLevel = frame.replace('"sequence":1', '"sequence":1,"sequence":2');
	assert.throws(() => decodeInboundJsonl(duplicateTopLevel), ProtocolDecodeError);
	const duplicateNested = frame.replace('"payload":{}', '"payload":{"ignored":1,"ignored":2}');
	assert.throws(() => decodeInboundJsonl(duplicateNested), ProtocolDecodeError);
});

/** The host sends bits; Rust implements the charging conversion independently. */
function ceilingMicroUsdForBoundaryTest(binary64Hex: string): number {
	const buffer = new ArrayBuffer(8);
	const view = new DataView(buffer);
	view.setBigUint64(0, BigInt(`0x${binary64Hex}`), false);
	return Math.ceil(view.getFloat64(0, false) * 1_000_000);
}
