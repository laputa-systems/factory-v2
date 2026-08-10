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
application supplies a bounded `MissionSourceRendering` separately and neither
knows nor supplies the resulting `ContentObjectId`. The resident privately
checks the bytes against the declared digest, preflights the outer command
without mutation, physically seals them, records the receipt/object chain, and
then the kernel resolves the digest to the registered object during founding
installation. The binding is byte custody, not producer provenance,
semantic/evidence admission, or a copied prompt fragment.

### Founding mission source operation

The daemon-private, retry-stable path from one application rendering to its
founding-mission binding: digest preflight, physical byte seal, kernel seal
receipt, global content-object registration, then `InstallFoundingMission`.
Its deterministic operation identities make the internal content primitive
retry-stable while the same daemon authority is retained. That is not a
supervisor-visible continuation after a handler failure: the request ends, and
a restarted nonempty daemon is `RecoveryFenced` rather than completing a
partial operation. The supervisor carries `MissionSourceRendering` with
`InstallFoundingMission`, but no public or supervisor protocol offers a generic
content mutation or content-writer authority through this path.

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

The custody nucleus owns the one PID/PGID, group liveness, signals, direct
wait, and bounded native streams. Pi may attach a strict session/protocol
sidecar; a deterministic evaluator attaches no Pi identity. The current
direct-executable evaluator treatment is a private unscheduled fixture, not an
application evaluator profile, scheduler, evidence admission, or execution
result.

### Office Prompt terminal evidence

The closed SDK-boundary result for one authorized Office Prompt. An observed
assistant outcome requires the exact `AgentSettled -> final accounting ->
Settled` sequence. An SDK-level unavailable assistant outcome can occur before
any agent lifecycle and instead requires `final Known usage -> Settled` with no
invented `AgentSettled`. Only completed/observed-stop may restore Office Ready;
failed and protocol-failed terminals remain non-ready, and protocol failure
also removes further session authority.

### Office-session Dispose chain

The durable close grammar for one idle `RootAuthorityOfficeSession`:
`Authorize-before-write -> delivered -> accepted -> final Known/failure ->
Disposed`. Authorization precedes every host-pipe byte; delivery means the
complete physical Dispose frame was written, not merely admitted. The accepted
result is followed immediately by one final cumulative Known usage observation
or one typed accounting failure. Only the Known branch can carry the next
`Disposed` transcript-flush receipt. A materialized receipt binds an
owned-custody, content-sealed transcript and may explicitly mark its first
prompt absent. Only the lazy missing-file no-Prompt receipt is unmaterialized
and cannot fabricate content; neither absence arm may invent a first prompt.
The final Known terminal reconciles or freezes the existing parent reservation.
A failure freezes it without a synthetic
`Disposed` receipt. It does not establish process exit or direct-child reap.

### Content object and content seal receipt

A global immutable byte identity and the narrow kernel attestation that the
physical content boundary has sealed it. Sealing establishes byte identity
only; it does not establish producer, schema, provenance, evidence, or graph
meaning. A bounded verified-read receipt likewise establishes only the digest
and byte count copied from the owned store; it confers no executable or
semantic authority.

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
