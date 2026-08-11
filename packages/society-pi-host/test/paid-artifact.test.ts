import test from "node:test";
import assert from "node:assert/strict";

import {
	buildPaidQualificationArtifact,
	PAID_QUALIFICATION_ARTIFACT_KIND,
	PAID_QUALIFICATION_EVIDENCE_STATUS,
} from "../src/paid-artifact.js";
import {
	ADMITTED_LING_26_FLASH_CANONICAL_MODEL_SLUG,
	ADMITTED_LING_26_FLASH_MODEL,
	ADMITTED_NON_REASONING_THINKING_LEVEL,
	PINNED_FORUM_F0_AWARENESS_BLAKE3,
	PINNED_PROVIDER,
	blake3Digest,
} from "../src/protocol.js";

const digest = blake3Digest("a".repeat(64));

function input(bodyUtf8 = "A bounded observation.") {
	return {
		model: {
			provider: PINNED_PROVIDER,
			modelId: ADMITTED_LING_26_FLASH_MODEL,
			canonicalSlug: ADMITTED_LING_26_FLASH_CANONICAL_MODEL_SLUG,
			thinkingLevel: ADMITTED_NON_REASONING_THINKING_LEVEL,
			catalogBlake3: digest,
		},
		totalCostCeilingUsd: 0.5,
		actorCostCeilingUsd: 0.03125,
		reports: [{
			actor: "cl001-retained-source-observer-1",
			arm: "retained" as const,
			population: "source" as const,
			role: "observer" as const,
			status: "completed" as const,
			providerAttempts: 1,
			inputTokens: 10,
			outputTokens: 5,
			cacheReadTokens: 0,
			cacheWriteTokens: 0,
			totalTokens: 15,
			costUsd: 0.001,
			providerCostUsd: 0.001,
			catalogEstimateUsd: 0.001,
			forumPosts: 1,
			forumReads: 1,
			forumErrors: 0,
		}],
		forumPosts: [{
			messageId: "actor-m2",
			ordinal: 2,
			author: "cl001-retained-source-observer-1",
			messageKind: "finding" as const,
			bodyUtf8,
			inReplyToMessageId: null,
			supersedesMessageId: null,
		}],
		forumReads: [{
			actor: "cl001-retained-source-observer-1",
			firstMessageOrdinal: 1,
			throughMessageOrdinal: 1,
			returnedBytes: 120,
			renderingBlake3: blake3Digest(PINNED_FORUM_F0_AWARENESS_BLAKE3),
		}],
	};
}

test("paid qualification artifact is stable and explicitly noncanonical", () => {
	const first = buildPaidQualificationArtifact(input());
	const second = buildPaidQualificationArtifact(input());
	assert.deepEqual(first, second);
	assert.equal(first.artifactKind, PAID_QUALIFICATION_ARTIFACT_KIND);
	assert.equal(first.evidenceStatus, PAID_QUALIFICATION_EVIDENCE_STATUS);
	assert.equal(first.protocol.postgresPersistence, "absent");
	assert.equal(first.protocol.cl001Lifecycle, "not_executed");
	assert.equal(first.integrity.bodyBlake3.length, 64);
});

test("paid qualification artifact digest changes when a raw Forum body changes", () => {
	const first = buildPaidQualificationArtifact(input("first observation"));
	const second = buildPaidQualificationArtifact(input("different observation"));
	assert.notEqual(first.integrity.bodyBlake3, second.integrity.bodyBlake3);
});

