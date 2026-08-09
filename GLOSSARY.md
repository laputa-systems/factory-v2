# Generic society glossary

## Status and use

This is the canonical vocabulary for the generic apparatus. It names durable
boundaries, not a product's source-language or evaluator concepts. An
application may extend this vocabulary beneath `applications/<product>/`; it
does not redefine the generic terms.

## Terms

### Application identity and revision

`ApplicationIdentity` is the durable external identity of one product-facing
application. `ApplicationRevisionId` and `ApplicationRevisionOrdinal` identify
one immutable, ordered revision of that application's typed mission contract.
They are domain types, not a package name interpreted by the kernel.

### Mission

The typed purpose and worldview supplied by an application at bootstrap. It
contains a named `MissionStatement`, ordered `MissionPrinciple` relation,
`NorthStarQuestionSet`, and a BLAKE3 identity for its source rendering. The
current digest is a caller attestation, not a physical content seal or
provenance claim. Mission is durable input, not ambient documentation or a
copied prompt fragment.

### Founding mission

The one installed `FoundingMission` record that binds bootstrap to its exact
`ApplicationRevisionId`. It is a root-governance cross-link: the kernel does
not interpret the application's mission prose or derive a product
constitution from it.

### Root authority office

The current prototype's single `RootAuthorityOffice` and active occupancy.
It grants the closed root-governance capability bundle and may open one
bounded `RootAuthorityOfficeSession` per Operating Cycle. It is not an
application office title or product-specific governance model.

### North-star alignment

One work object's typed application of its exact mission revision. It has four
named fields: intended capability or behavior change, improvement evidence,
boundary commitment, and revisit condition. Mission supplies purpose;
alignment tests an action. The current kernel requires this exact alignment on
Project creation and rejects a revision other than the founding mission revision.

### Capability

A narrow durable permission to execute one named command over one jurisdiction.
Capability, current office occupancy or actor grant, expiry, and lifecycle
state determine authority. A prompt, title, repository, or tool access is not a
capability.

### Operating Cycle

A finite, admitted execution epoch inside the resident authority. It pins the
mission revision, execution profile, budget, admission generation, and
cancellation root. The daemon may remain resident after cycle closure; a
successor is a new cycle.

### Admission generation

A monotonically increasing fence on an admission scope. A reservation captures
the generation and the trusted supervisor rechecks it immediately before a
paid or effectful child boundary. Quiescence or cancellation invalidates stale
admission.

### Budget envelope and reservation

A hard integer-micro-unit ceiling and its transactional reservation. Every
applicable envelope is charged before paid work begins. Unknown or unavailable
cost is not zero and freezes later paid admission according to the named
policy.

### Cancellation request and propagation

A typed control-plane request over a named scope, followed by its exact frozen
set of owner and child obligations. Quiescence stops new admission;
cancellation controls owned work; closure follows reconciliation. Process
exit, direct-child reap, protocol terminal evidence, and post-restart absence
are distinct facts.

### Execution profile

A closed, revisioned description of an eligible runtime boundary. Runtime
identity, qualification, and readiness are explicit facts; a caller-supplied
executable path or a successful prompt never qualifies a profile.

### Child process lifecycle

The generic OS-custody state of a registered native child. It is distinct from
SDK-session progress and from application work success. A retained readiness
fact cannot make a reaped or recovery-contained process operational.

### Content object and content seal receipt

A global immutable byte identity and the narrow kernel attestation that the
physical content boundary has sealed it. Sealing establishes byte identity
only; it does not establish producer, schema, provenance, evidence, or graph
meaning.

### Admitted evidence

A typed observation or sealed content object given a semantic role under a
named evaluator, scope, and admitting authority. Admission does not make the
claim true, sufficient, or culturally inherited.

### Event ledger

The append-only record of accepted commands, transitions, resources, and
principals. It is replay-auditable and is distinct from content bytes and
current epistemic interpretation.

### Curated account

A revisioned, purpose-specific selection of consequential admitted evidence,
arguments, conflict, unknowns, and exclusions for a decision. It preserves a
challenge path to source evidence rather than replacing it with a summary.

### Decision packet

The typed, revisioned basis for an authorized decision: scope, alternatives,
evidence, challenges, dissent, predictions, authority, and revisit condition.
It is not a free-form approval message.

### Product change

One independently reviewable and revertible proposed mutation to an
application-owned repository or other admitted product target. The generic
apparatus records its authorization boundary but does not interpret its product
semantics.

### Authorized product change

The future kernel-issued output binding one authorization, application
revision, decision, repository, admitted base, patch, expected tree,
validation profile, and delivery target. The current materializer receipt is
not yet this durable output.

### Product change authorization input

`ProductChangeAuthorizationInput` is caller-supplied structural input to the
standalone materializer. It preserves application-revision and decision
cross-links but proves neither kernel authority nor receipt authenticity. It
must not be confused with the future kernel-issued
`AuthorizedProductChange`.

### Controlled product materialization

The isolated transition from an already authorized base, patch, tree, and
validation profile to an exact tree, typed validation receipt, controlled
commit, and guarded local compare-and-swap delivery. Materialization verifies
authority; it does not create it.

### Replay

Reconstruction and validation of materialized state from the event ledger and
named command/event bodies. Replay is an audit property, not an application
evaluator and not a permission to reinterpret historical records.

### Trusted physics

The slowly changing Rust, SQLite, and operating-system mechanism layer that
makes identity, authority, budgets, process ownership, cancellation, content
identity, state transitions, delivery, and replay real. It is not an
application governance tier.

## Required distinctions

| Do not collapse | Distinction |
| --- | --- |
| Mission / north-star alignment | Purpose / action-specific test against that purpose |
| Capability / prompt instruction | Durable authority / untrusted content |
| Content object / admitted evidence | Byte identity / semantic role under an evaluator |
| Event ledger / curated account | Trusted occurrence history / decision representation |
| Quiescence / cancellation / closure | Stop admission / control owned work / reconcile terminal state |
| Process exit / reap / protocol terminal state | OS observation / parent custody / SDK-boundary fact |
| Materialization / authorization / delivery | Prepare exact tree / permit it / guarded target transition |
