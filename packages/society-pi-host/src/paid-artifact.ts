import { blake3Hex } from "./digest.js";
import { blake3Digest, type Blake3Digest, type CanonicalModelSlug, type ModelId, type ThinkingLevel } from "./protocol.js";

/** The deliberately reduced live adapter profile, not a CL-001 episode. */
export const PAID_QUALIFICATION_ARTIFACT_KIND = "cl001_paid_qualification_artifact_v1" as const;
export const PAID_QUALIFICATION_EVIDENCE_STATUS = "noncanonical_adapter_qualification" as const;

export type PaidQualificationArm = "retained" | "reset";
export type PaidQualificationPopulation = "source" | "successor";
export type PaidQualificationRole = "observer" | "critic" | "synthesizer" | "challenger";
export type PaidQualificationActorStatus = "completed" | "failed" | "rate_limited" | "budget_guardrail";

/** One actor's bounded provider and tool accounting. */
export interface PaidQualificationActorReport {
	readonly actor: string;
	readonly arm: PaidQualificationArm;
	readonly population: PaidQualificationPopulation;
	readonly role: PaidQualificationRole;
	readonly status: PaidQualificationActorStatus;
	readonly providerAttempts: number;
	readonly inputTokens: number;
	readonly outputTokens: number;
	readonly cacheReadTokens: number;
	readonly cacheWriteTokens: number;
	readonly totalTokens: number;
	readonly costUsd: number;
	readonly providerCostUsd: number;
	readonly catalogEstimateUsd: number;
	readonly forumPosts: number;
	readonly forumReads: number;
	readonly forumErrors: number;
	readonly error?: string;
}

/** Exact immutable Forum post occurrence retained by the adapter smoke. */
export interface PaidQualificationForumPost {
	readonly messageId: string;
	readonly ordinal: number;
	readonly author: string;
	readonly messageKind: "finding" | "correction" | "question" | "challenge" | "synthesis";
	readonly bodyUtf8: string;
	readonly inReplyToMessageId: string | null;
	readonly supersedesMessageId: string | null;
}

/** Exact bounded read interval and returned rendering identity. */
export interface PaidQualificationForumRead {
	readonly actor: string;
	readonly firstMessageOrdinal: number;
	readonly throughMessageOrdinal: number;
	readonly returnedBytes: number;
	readonly renderingBlake3: Blake3Digest;
}

export interface PaidQualificationArtifactInput {
	readonly model: {
		readonly provider: "openrouter";
		readonly modelId: ModelId;
		readonly canonicalSlug: CanonicalModelSlug;
		readonly thinkingLevel: ThinkingLevel;
		readonly catalogBlake3: Blake3Digest;
	};
	readonly totalCostCeilingUsd: number;
	readonly actorCostCeilingUsd: number;
	/** Integer guardrails; USD renderings are derived only for display. */
	readonly totalCostCeilingMicroUsd: number;
	readonly actorCostCeilingMicroUsd: number;
	readonly reports: readonly PaidQualificationActorReport[];
	readonly forumPosts: readonly PaidQualificationForumPost[];
	readonly forumReads: readonly PaidQualificationForumRead[];
}

/**
 * Machine-readable evidence for one adapter qualification run. The explicit
 * persistence fields make it impossible to confuse this artifact with the
 * generic PostgreSQL study ledger: it has no episode, assignment, correction
 * release, replacement barrier, ground-truth reveal, or measurement rows.
 */
export interface PaidQualificationArtifact {
	readonly artifactKind: typeof PAID_QUALIFICATION_ARTIFACT_KIND;
	readonly evidenceStatus: typeof PAID_QUALIFICATION_EVIDENCE_STATUS;
	readonly protocol: {
		readonly topology: {
			readonly totalActorLifetimes: 16;
			readonly rolesPerCell: 4;
			readonly maxConcurrentActors: 8;
			readonly arms: readonly ["retained", "reset"];
			readonly populations: readonly ["source", "successor"];
			readonly roles: readonly ["observer", "critic", "synthesizer", "challenger"];
		};
		readonly treatmentLabels: "report_metadata_only";
		readonly forumState: "one_in_memory_store_shared_by_all_cells";
		readonly daemonCustody: "absent";
		readonly postgresPersistence: "absent";
		readonly cl001Lifecycle: "not_executed";
	};
	readonly model: PaidQualificationArtifactInput["model"];
	readonly budget: {
		readonly totalCeilingUsd: number;
		readonly actorCeilingUsd: number;
		readonly totalCeilingMicroUsd: number;
		readonly actorCeilingMicroUsd: number;
	};
	readonly actors: readonly PaidQualificationActorReport[];
	readonly forum: {
		readonly posts: readonly PaidQualificationForumPost[];
		readonly reads: readonly PaidQualificationForumRead[];
	};
	readonly integrity: {
		/** Digest of the canonical JSON body excluding this integrity object. */
		readonly bodyBlake3: Blake3Digest;
	};
}

/** Build a stable, self-describing artifact from one completed adapter run. */
export function buildPaidQualificationArtifact(input: PaidQualificationArtifactInput): PaidQualificationArtifact {
	const body = {
		artifactKind: PAID_QUALIFICATION_ARTIFACT_KIND,
		evidenceStatus: PAID_QUALIFICATION_EVIDENCE_STATUS,
		protocol: {
			topology: {
				totalActorLifetimes: 16 as const,
				rolesPerCell: 4 as const,
				maxConcurrentActors: 8 as const,
				arms: ["retained", "reset"] as const,
				populations: ["source", "successor"] as const,
				roles: ["observer", "critic", "synthesizer", "challenger"] as const,
			},
			treatmentLabels: "report_metadata_only" as const,
			forumState: "one_in_memory_store_shared_by_all_cells" as const,
			daemonCustody: "absent" as const,
			postgresPersistence: "absent" as const,
			cl001Lifecycle: "not_executed" as const,
		},
		model: input.model,
		budget: {
			totalCeilingUsd: input.totalCostCeilingUsd,
			actorCeilingUsd: input.actorCostCeilingUsd,
			totalCeilingMicroUsd: input.totalCostCeilingMicroUsd,
			actorCeilingMicroUsd: input.actorCostCeilingMicroUsd,
		},
		actors: input.reports,
		forum: {
			posts: input.forumPosts,
			reads: input.forumReads,
		},
	};
	const bodyBlake3 = blake3Digest(blake3Hex(JSON.stringify(body)));
	return { ...body, integrity: { bodyBlake3 } };
}
