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
prototype implements the normalized mission, its daemon-private sealed-source
binding, and Project alignment portion, but not durable authorized
product-change output.

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
  source_rendering_digest: Blake3Digest

MissionSourceRendering
  canonical nonempty bytes, bounded to 16,384 bytes

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
input. The application owns bounded `MissionSourceRendering` bytes plus the
input's BLAKE3 digest, but never a `ContentObjectId`. A daemon-private
founding-source path checks those bytes, side-effect-free preflights the outer
command, physically seals them, records the seal receipt and global object,
then invokes `InstallFoundingMission`; the kernel resolves the declared digest
to the registered object and persists a derived private binding. Deterministic
internal operation identities make the content primitive retry-stable while the
authority remains live; they do not make a failed supervisor handler resumable.
The request ends on that failure, and restart remains `RecoveryFenced`, with no
partial source-operation recovery. The supervisor carries
`MissionSourceRendering` only with `InstallFoundingMission`; no public or
supervisor generic content-mutation command or content-writer authority exists.
This is byte custody only: it establishes no producer provenance,
semantic/evidence admission, or end-to-end application execution.
The materializer's caller-supplied
`ProductChangeAuthorizationInput` and local receipt are likewise not a durable
kernel-issued `AuthorizedProductChange`.

The fresh bootstrap names its single generic root-governance relation
`FoundingMission`, `RootAuthorityOffice`, and `RootAuthorityOfficeSession`.
Those names preserve the existing single-root-office state and numeric
protocol semantics without importing an application's institutional vocabulary.

The daemon-private Pi bridge currently projects an authorized Office Prompt's
complete physical delivery, accepted result, cumulative accounting facts, and
closed terminal sequence into the kernel. Observed assistant results require
`AgentSettled -> final accounting -> Settled`; SDK-unavailable results require
the adjacent final Known usage fact and `Settled` without inventing an agent
lifecycle. Only completed/observed-stop returns the Office to Ready. This is a
generic execution boundary, not evidence that an application's requested work
was correct or that a product change is authorized.

The same daemon-private foundation now records an idle Office-session Dispose
chain as `Authorize-before-write -> delivered -> accepted -> final
Known/failure -> Disposed`. Authorization is durable before the physical
Dispose write; delivery is the complete pipe write; accepted is a separate host
fact. The immediately following final Known usage permits the next `Disposed`
receipt, whose materialized SessionManager transcript is verified under the
owned session directory and sealed by the daemon's sole content writer before
the kernel records its object identity. A no-Prompt session may instead have a
sealed materialized transcript whose first prompt is explicitly absent; only a
lazy missing session file is explicitly unmaterialized and has no content
object. Neither absence arm can invent a first prompt or content. The final Known terminal
reconciles the existing parent reservation, releasing its unused reserve, or
freezes a known overrun. A final accounting failure freezes the reservation and
enters containment without a synthetic `Disposed` receipt. Reaping the child
is a separate process-custody transition.

This is still only a same-lifetime foundation: the resident serving loop has
no scheduler/control-loop call site for it, and there is no post-restart
recovery, workspace disposal, semantic submission, paid/native qualification,
or end-to-end application execution claim.

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
