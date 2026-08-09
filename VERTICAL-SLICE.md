# Generic society execution sequence

[`GLOSSARY.md`](GLOSSARY.md) is canonical for the generic terms used here, and
[`ARCHITECTURE.md`](ARCHITECTURE.md) owns the general trust boundary. An
application owns its own executable slice beneath `applications/<product>/`.

## Purpose

The generic apparatus is a resident authority for bounded, replayable work. It
does not contain a mission for a named product, an evaluator for a named
product behavior, or a product-specific delivery rule. It accepts typed
application purpose and alignment records, enforces authority and physical
boundaries, and emits an authorized product-change output only after the
required generic decision and review gates succeed.

## Intended typed application port

The following contract is the required complete boundary. The current
prototype implements the normalized mission and Project alignment portion, but
not the sealed source binding or durable authorized product-change output.

```text
ApplicationMissionInput
  application: ApplicationIdentity
  revision: ApplicationRevision
  mission: MissionStatement
  principles: nonempty ordered MissionPrinciple relation
  north_star_questions:
    change
    improvement_evidence
    boundary_commitment
    revisit
  source_content: ContentObjectId

ProjectNorthStarAlignment
  application_revision: ApplicationRevisionId
  change: CapabilityOrBehaviorChange
  improvement_evidence: ImprovementDiscriminator
  boundary_commitment: BoundaryCommitment
  revisit: RevisitCondition

AuthorizedProductChange
  authorization: ProductDeliveryAuthorizationId
  application_revision: ApplicationRevisionId
  product_change: ProductChangeId
  authorizing_decision: DecisionId
  repository: ProductRepositoryId
  admitted_base: CommitId
  accepted_patch: PatchDigest
  accepted_tree: TreeId
  validation_profile: ValidationProfileId
  delivery_target: LocalBranchRef
```

Each field is a domain newtype, identifier, or closed relation. The durable
schema uses one named table per body and closed enum kinds where a vocabulary
is required. The port must not introduce a JSON document, generic payload,
metadata map, EAV table, or a stringly application discriminator. Human text is
admitted only through its named typed field and, where provenance matters,
through a separately sealed `ContentObjectId`.

An application may define additional meaning behind its own revision and
content objects. The generic kernel records the revision, cardinality,
authority, lineage, and evidence links; it does not reinterpret application
semantics or load an application crate.

## Generic execution sequence

```text
typed application mission and north-star input
  -> generic bootstrap, capability, and budget envelope
  -> admitted Operating Cycle with a pinned execution profile
  -> bounded actor or deterministic work under process ownership
  -> sealed content and typed evidence admission
  -> preserved challenge, decision, and review records
  -> AuthorizedProductChange
  -> isolated materialization and typed validation receipt
  -> guarded local delivery receipt
  -> outcome obligation, replay, reconciliation, and closure
```

The trusted apparatus owns SQLite mutation, identity, capability checks,
budgets, content sealing, process groups, cancellation, replay, and the
authenticated product-delivery boundary. Applications own mission prose,
alignment questions, source-specific evaluators, validation profiles, and
product conclusions. An application child never gains SQLite, capability,
Pi-session, process-reaping, cancellation, or delivery-ref authority merely by
being able to run a tool.

## Current implementation boundary

The repository currently implements a normalized `ApplicationMissionInput`,
four typed north-star questions, founding-mission-revision-bound
`ProjectNorthStarAlignment`, and closed daemon transport for that founding
input. The source rendering is currently identified by a declared BLAKE3
digest only; no `ContentObjectId`, physical seal, retention, or provenance is
established. The materializer's caller-supplied
`ProductChangeAuthorizationInput` and local receipt are likewise not a durable
kernel-issued `AuthorizedProductChange`.

The fresh bootstrap names its single generic root-governance relation
`FoundingMission`, `RootAuthorityOffice`, and `RootAuthorityOfficeSession`.
Those names preserve the existing single-root-office state and numeric
protocol semantics without importing an application's institutional vocabulary.

The generic sequence therefore makes no claim that an application can yet run
end to end. A product-specific executable contract must state its own admitted
inputs, evaluators, budgets, delivery gates, and acceptance judge under
`applications/<product>/`.

## Acceptance direction

Before an application may claim a complete generic-society execution, its
acceptance suite must demonstrate all of the following without provider calls
in ordinary tests:

- typed mission and alignment records are revisioned, authorized, and linked to
  every consequential work object;
- stale admission, authority escalation, budget overrun, unsealed content,
  invalid evidence, and a moved delivery base are rejected;
- every accepted transition replays from the ledger and no application child
  writes durable state directly;
- a product delivery follows one exact authorization, base, patch, tree, and
  validation profile; and
- closure proves reconciliation of owned children, budgets, and open outcome
  obligations.

An application-specific slice may prove additional behavior. That evidence is
owned by the application and must not be represented as a generic capability.
