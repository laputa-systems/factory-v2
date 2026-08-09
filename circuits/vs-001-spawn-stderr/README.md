# VS-001 spawn stderr deterministic circuit

This package is the provider-free experiment for the VS-001 process-contract
question. It creates all ephemeral data under the caller-selected output
directory and never writes to the XSH checkout. It is deliberately not a
workflow manifest: its executable cases are named fixtures and the durable
V2 implementation must represent their inputs and observations with closed
Rust types and normalized SQLite rows.

## Scope and invocation

Run the behavior matrix against a previously built XSH binary:

```text
circuits/vs-001-spawn-stderr/judges/run-behavior-matrix.sh \
  --xsh /absolute/path/to/xsh \
  --xsht /absolute/path/to/xsht \
  --xsh-root /absolute/path/to/xsh-source \
  --out /absolute/empty/output-directory
```

The documentation and discovery matrix needs the matching XSH source tree and
`xsht` binary:

```text
circuits/vs-001-spawn-stderr/judges/run-documentation-matrix.sh \
  --xsh-root /absolute/path/to/xsh \
  --xsht /absolute/path/to/xsht \
  --mode baseline-conflict \
  --out /absolute/empty/output-directory
```

All judges require an empty output directory and a clean XSH checkout. They
refuse a dirty source tree rather than attach behavior to an ambiguous revision.
Each output contains `xsh-source-head.txt`, `xsh-source-status.txt`, and a
typed `input-digests.v1.tsv` covering the XSH/Xsht binary, evaluator, fixtures,
source owners, and API outputs used by that judge. The kernel must still bind
the binary digest to its own qualified build receipt; a matching source `HEAD`
does not by itself prove how a local binary was built. The scripts write no
product checkout file.

The host profile is Unix-like: POSIX `sh`, `/dev/null`, an executable XSH/Xsht
pair, and `awk`, `cmp`, `cp`, `cut`, `env`, `find`, `git`, `mkdir`, `od`, `rg`, `sed`,
`sort`, plus either
`sha256sum` or `shasum` must be available. These are evaluator prerequisites,
not XSH product dependencies.

`judges/run-negative-controls.sh` takes the same `--xsh`, `--xsht`, and
`--xsh-root` inputs, invokes the positive behavior suite, and then proves that
each deliberately bad fixture is rejected for its named reason. It is a test
of the evaluator, not evidence that the product is correct.

## Closed observation protocol

The `*.tsv` files are a transport for a future typed V2 adapter, not state and
not a generic metadata channel. Each non-comment row has exactly the columns
listed in its header. Fields never contain tabs, newlines, or free-form maps.
Raw stdout, stderr, and redirected-file bytes stay as separately sealed files
under `artifacts/`; the TSV contains only their SHA-256 identities.

For each stream, `*_evidence_kind` is one of `redirected`,
`inherited_parent_stdout`, `inherited_parent_stderr`, `not_produced`, or
`redirection_ignored`, or `redirected_dev_null`. Only `redirected` and
`redirection_ignored` carry a digest in the companion `*_evidence_sha256`
field; the other four write `-`. Those spellings are wire encodings of
`StreamEvidence`, not an extensible tag space.

The Rust integration must parse a sealed row into the following closed shape
before admission:

```text
BehaviorObservationV1 {
  case_id: BehaviorCaseId,
  consumer: CommandConsumer,
  input_manifest: BehaviorInputManifest,
  expected_contract_source: ExpectedContractSource,
  disposition: BehaviorDisposition,
  exit_shape: ExitOrErrorShape,
  parent_stdout: ContentDigest,
  parent_stderr: ContentDigest,
  stdout_evidence: StreamEvidence,
  stderr_evidence: StreamEvidence,
  lifecycle: ProcessLifecycleReceipt,
}

StreamEvidence =
  | Redirected { destination: ContentDigest }
  | Inherited { parent_stream: ParentStream }
  | NotProduced
  | RedirectionIgnored { destination: ContentDigest }
  | RedirectedExternalSink { sink: ExternalStreamSink }

DocumentationObservationV1 {
  source: DocumentationSource,
  consumer: CommandConsumer,
  field: StdioField,
  claim: DocumentationClaim,
  citation: SourceCitation,
}
```

`BehaviorCaseId`, `CommandConsumer`, `BehaviorDisposition`,
`BehaviorInputManifest`, `ExpectedContractSource`, `ExitOrErrorShape`,
`StreamEvidence`, `ParentStream`, `ExternalStreamSink`,
`ProcessLifecycleReceipt`, `DocumentationSource`, `StdioField`, and
`DocumentationClaim` must be closed Rust enums/newtypes. In particular, an
inherited stream is not a copy of the parent stream's digest, and an unstarted
child is not an empty redirected file. The adapter must reject an unknown
identifier instead of preserving it as a string. A resulting source conflict is
an `Observation`; a conclusion such as “the proposal is stale” remains an
`Argument` made later in the episode.

## Behavior cases

| Case ID | Consumer | Contract exercised |
| --- | --- | --- |
| `B01` | `process.run` | typed stdout/stderr `Path` fields redirect exact child bytes |
| `B02` | `spawn command` | managed handle honors the same fields, waits, and reports owned completion |
| `B03` | `spawn run` | direct command redirections are independent and owned completion is observable |
| `B04` | `process.spawn` | detached-record behavior is measured independently; it is never assumed equivalent to an owned handle |
| `B05` | `spawn command` | missing stderr field inherits child stderr to the parent |
| `B06` | `process.run` | non-append stderr truncates an existing file |
| `B07` | `process.run` | `stderr_append: true` appends without changing stdout policy |
| `B08` | `spawn command` | an invalid stderr destination produces a setup error before child completion evidence exists |
| `B09` | `spawn command` | a nonzero child status remains status data while stderr is retained |
| `B10` | `spawn command` | cancellation returns after preserving partial redirected bytes and blocking the delayed side effect |
| `B11` | `process.run` | `/dev/null` is an ordinary typed `Path` stderr sink |

`noisy-child.xsh` deliberately emits one distinct stdout line, stderr line,
and completion marker. The marker prevents a `process.spawn` observation from
being mistaken for a scheduling race merely because a detached child has not
yet started. `noisy-sleeper.xsh` completes two direct argument-vector `printf`
writes before publishing readiness, then emits its completion marker after 250
ms. B10 waits 400 ms after cancellation: it checks that cancellation preserves
those partial redirected bytes and blocks the delayed side effect over an
observation window that outlasts it. It does **not** prove that no OS descendant
remains alive: `societyd` must independently prove process-group reaping from
its child registry and termination receipts.

The evaluator observes `process.spawn` rather than presupposing whether that
API should honor command-plan stdio fields. At the inspected XSH revision,
the expected discriminating result is that B01–B03 honor redirection but B04
does not. That is support for H3, not a product verdict: delivery must decide
whether the API is intentionally detached, whether its public documentation is
adequate, and whether no runtime change is required.

| Hypothesis | Current disposition | Evidence boundary |
| --- | --- | --- |
| H1 — missing behavior | rejected for the owned paths | B01–B03 redirect exact stderr bytes, and B08/B09 preserve the setup-error/status distinction |
| H2 — implemented but culturally stale | partly supported but incomplete | `LANG.md` and `xsht` discovery are stale, but that alone does not explain the detached consumer's distinct policy |
| H3 — split or accidental behavior | supported | B04 preserves both redirection sentinels while B01–B03 redirect, and the runtime citations trace that distinction to detached versus managed options |

## Documentation/discovery claims

The documentation judge does not scrape prose into an untyped score. It asks
for exactly the cells relevant to this decision:

| Source | Consumer | Cell |
| --- | --- | --- |
| `LANG.md` | `spawn command` | proposal says stderr support is missing |
| `docs/SPEC.md` spawn section | `spawn command` | command-plan redirections are used and default stdio inherits |
| `docs/SPEC.md` API and builder sections | `Command` | `stderr` and `stderr_append` are typed fields |
| `docs/SPEC.md` process contract | `spawn command` | setup errors remain distinct from status data |
| `docs/SPEC-OS.md` | `spawn command` | handle owns a child process group; redirection failure is distinct from status |
| `xsht api api:process.command_argv` | `Command` | typed stderr fields are discoverable |
| `xsht api search:process` | `Command`, `process.spawn` | reference navigation finds both surfaces |
| `docs/SPEC.md` and `xsht api api:process.spawn` | `process.spawn` | detached-record versus owned-handle lifecycle claim |
| runtime construction | `process.spawn`, managed `spawn` | traced call paths distinguish detached disabled redirections from managed enabled redirections |
| focused tests | `process.run`, `spawn command` | current native coverage reaches run redirection but lacks a managed-spawn stderr assertion |

The emitted conflict report is intentionally small. It proves a source surface
or test is present and preserves its cited line range; it does not claim that a
regular-expression match establishes intent.

`baseline-conflict` is the observation mode: it succeeds while emitting every
named present conflict. `candidate-reconciled` is the delivery gate: D01 and
D02 become `resolved` only when the checked-in candidate documentation removes
the missing-behavior claim and the candidate's actual built `xsht` declares the
detached lifecycle. D03 is an `intentional_semantic_split`, not a contradiction:
the detached and managed APIs have deliberately different redirection policy.
Candidate runs admit no source overlays or invented API transcript.

## Negative controls

The negative suite proves the judge has a rejection path for each VS-001 rule:

| Fixture/control | Must be rejected because |
| --- | --- |
| `negative/no-stderr-plan.xsh` | omitting the stderr field cannot satisfy a redirect expectation |
| `negative/shell-wrapper.xsh` | an `sh -c` wrapper violates the typed-command boundary even if bytes redirect |
| `negative/fake-log.xsh` | writing the expected log cannot pass a varying-payload/lifecycle test |
| current clean baseline checkout in `candidate-reconciled` mode | the actual evaluator rejects its stale proposal claim |
| default-inheritance counterfactual | the behavior evaluator runs with `--default-stderr suppressed` and rejects the actual inherited stderr bytes |

The shell-wrapper fixture is inspected, never executed. The circuit refuses to
make a shell boundary part of the experiment merely to prove that shell syntax
can redirect a file descriptor.

## Milestone-7 provider-free extensions

The following judges advance the deterministic circuit toward the Milestone 7
contract without pretending to be `societyd`, a Pi session, or a SQLite ledger.
They use a small collection of named TSV relations only because the listed
closed Rust parser types will replace them at the trusted boundary. None is a
workflow manifest, an opaque metadata map, or authority to accept a curation,
promote a lesson, or disclose material.

They do **not** claim Milestone 7 completion. The resident Rust kernel,
normalized SQLite state, typed Pi boundary, actor configuration/assignment
records, notice/cycle policy, and process-double end-to-end closure judge remain
the integration boundary outside this circuit.

### Paired fluency fixture workspace and task evaluator

Run a single opaque arm with its sealed candidate `supervise.xsh` and the
closed deterministic stand-ins for its submission and tool-event summary:

```text
circuits/vs-001-spawn-stderr/judges/run-fluency-task-evaluator.sh \
  --xsh /absolute/path/to/xsh \
  --xsht /absolute/path/to/xsht \
  --xsh-root /absolute/path/to/xsh-source \
  --solution circuits/vs-001-spawn-stderr/fixtures/fluency/positive/supervise.xsh \
  --submission circuits/vs-001-spawn-stderr/fixtures/fluency/positive/submission.v1.tsv \
  --tool-events circuits/vs-001-spawn-stderr/fixtures/fluency/positive/tool-events.v1.tsv \
  --workspace-label q7f3a \
  --out /absolute/empty/output-directory
```

The judge materializes exactly this actor-visible workspace beneath `--out`:

```text
work/<opaque-label>/
├── TASK.md
├── REFERENCE.md
├── bin/{xsh,xsht}
├── fixtures/{child-alpha.xsh,child with spaces.xsh,child-nonzero.xsh}
├── output/
└── submission/{supervise.xsh,submission.v1.tsv}
```

It copies the assigned binaries and uses their workspace-local absolute paths;
the actor workspace has no XSH checkout. Every `xsht check` and supervisor case
actually changes directory to that opaque workspace and runs under `env -i`:
only an explicit assigned-bin-first `PATH`, workspace-local HOME/XDG/config/
cache/data/temp roots, locale, and terminal variables are present. The evaluator
records this closed execution envelope, rather than an absolute host path. Run
it once for each sealed opaque label. Treatment mapping, freshness, actor
identity, and settlement belong to the future kernel and must remain hidden
until both real submissions seal.

F01 proves pre-existing stderr truncation and exact inherited stdout; F02
repeats that proof with spaces in both child and log paths; F03 requires a
nonzero child status to become the supervisor's nonzero exit. Before execution
the same judge requires `xsht check --strict`, `process.command_argv`, owned
`spawn command?`/`wait handle?`, typed stderr, no stdout redirection, no shell
string, no `process.spawn`, and no fixture-specific output. Its closed output
is `FluencyProbeObservationV1`; `TaskActorSubmissionV1`,
`TaskActorToolEventV1`, and `FluencyProbeExecutionSurfaceV1` are likewise
closed Rust types. The last records each resource surface as
`not_observed_no_provider`, separately from correctness—this fixture makes no
fictional claim about Pi turns, tokens, wall time, tool errors, or cost.

The tool-event relation is a deterministic normalizer fixture for contamination
classification. A real Pi adapter must derive it from sealed tool events and
flag known XSH, V1, treatment, or other external-path access. `owned_waited`
is task-level evidence, not a kernel child-reaping receipt: `societyd` must
still bind the actual process group and liveness evidence to the Attempt.
`C17` additionally supplies ambient cwd/environment sentinels to a candidate
which would otherwise rely on them; the controlled runner clears them and the
candidate cannot pass.

### C1 curation contract judge

```text
circuits/vs-001-spawn-stderr/judges/run-curation-contract-judge.sh \
  --account-dir circuits/vs-001-spawn-stderr/fixtures/curation/c1-valid \
  --frontier-members circuits/vs-001-spawn-stderr/fixtures/curation/frontier-c1-members.v1.tsv \
  --out /absolute/empty/output-directory
```

`account.v1.tsv`, `selected-items.v1.tsv`, `preserved-conflicts.v1.tsv`,
`decision-relevant-unknowns.v1.tsv`, `exclusions.v1.tsv`, and
`raw-evidence-escalations.v1.tsv` are six fixed normalized relations for future
`CuratedAccountV1` child rows. The judge requires the exact C1 purpose and
frontier, all H1/H2/H3 with a dissent role, declared counterevidence, the
preserved conflict, a live unknown, semantic exclusions, and an admitted source
for every selection. Every relation has an exact header, row count, field arity,
and identity set; duplicate source, exclusion, frontier, or ordinal identities
cannot pass. A raw-evidence escalation must name both its question and requested
object. `curation-raw-evidence-escalations.v1.tsv` preserves each named request
as a closed row; the aggregate account state is `none_requested` only when that
relation is empty.

The named-escalation fixture exercises the nonempty path:

```text
circuits/vs-001-spawn-stderr/judges/run-curation-contract-judge.sh \
  --account-dir circuits/vs-001-spawn-stderr/fixtures/curation/c1-valid-named-escalation \
  --frontier-members circuits/vs-001-spawn-stderr/fixtures/curation/frontier-c1-members.v1.tsv \
  --out /absolute/empty/output-directory
```

Its `acceptance_ready` disposition means only that this closed content shape is
sufficient for a kernel admission attempt. The Rust kernel must enforce source
admission, curator/producer independence, capability, lifecycle, and Grand
Architect acceptance; this package does not emulate any of them.

### Checked-propagation uptake judge

```text
circuits/vs-001-spawn-stderr/judges/run-uptake-application-judge.sh \
  --context circuits/vs-001-spawn-stderr/fixtures/uptake/positive/delivery-context.v1.tsv \
  --persisted-input circuits/vs-001-spawn-stderr/fixtures/uptake/positive/persisted-input.v1.tsv \
  --submission circuits/vs-001-spawn-stderr/fixtures/uptake/positive/investigator-submission.v1.tsv \
  --accesses circuits/vs-001-spawn-stderr/fixtures/uptake/positive/accesses.v1.tsv \
  --out /absolute/empty/output-directory
```

The judge keeps `delivered`, `encountered`, and `applied_once` as separate
fields of `PropagationObservationV1`. It requires the exact scoped L1 revision
in both context and persisted input, then requires the investigator submission
to name that lesson and compare all four named record classes: normative
registry, executable behavior, active proposal corpus, and real call sites. A
record class may be unavailable only with its own closed availability reason.
The recommendation is deliberately unconstrained among new API, existing
contract, further experiment, and no change. Forbidden VS-001 session or
post-target access records contamination rather than uptake success. This one
arm cannot establish `causally_supported`.

### W1 disclosure-frontier leakage controls

```text
circuits/vs-001-spawn-stderr/judges/run-frontier-leakage-controls.sh \
  --frontier-dir circuits/vs-001-spawn-stderr/fixtures/frontier/w1-valid \
  --out /absolute/empty/output-directory
```

The frontier relations are an exact seven-member positive allowlist plus ten
distinct explicit sequestered aftermath classes. The judge derives 28 allowed
reads from the parsed members and 240 denied reads from the parsed aftermath
rows: every combination of four principals (replay actor, projector, ordinary
investigator, and Grand Architect query client), six lookup routes (identity,
graph, digest, current-repository path, culture, and projection), and ten
seeded aftermath classes. Every denial records
`contamination_audit_outside_w1`; none becomes an object in W1. The future
kernel must enforce the same `DisclosureFrontierV1` checks against real object,
graph, projector, and actor requests rather than trusting this fixture judge.

### New rejection controls

```text
circuits/vs-001-spawn-stderr/judges/run-society-negative-controls.sh \
  --xsh /absolute/path/to/xsh \
  --xsht /absolute/path/to/xsht \
  --xsh-root /absolute/path/to/xsh-source \
  --out /absolute/empty/output-directory
```

`C06` rejects detached `process.spawn`; `C07` rejects a fixture-specific fake
output; `C08` rejects known external XSH-path access; `C09` rejects an
unadmitted curation source; `C10` rejects an unnamed raw-evidence request;
`C11` rejects missing uptake comparison evidence; `C12` records forbidden
session access as contamination; `C13` rejects a frontier whose member overlaps
sequestered aftermath material; `C14`–`C16` reject duplicate source/exclusion
identities and an extra curation row; `C17` rejects inherited host cwd/env
dependence; and `C18`–`C19` reject a missing required positive member and a
duplicated sequestered aftermath class. `C20` rejects an otherwise-valid opaque
workspace label longer than the closed 64-byte path-component bound.

## Current XSH ownership map

At source revision `04fb98f8c63b63cccffce7ef2c3cabde81bb05ba` in the
authoritative checkout `/Users/josh/d/laputa-systems/xsh`:

| Concern | Exact owner | Current evidence |
| --- | --- | --- |
| Command plan fields | `src/runtime/eval/lowered_run.rs` | lowers `stderr` and `stderr_append` into command redirections |
| Managed `spawn command` | `src/runtime/eval/lowered_run.rs` | creates inherited managed options with `apply_redirections = true` |
| Managed OS process | `src/runtime/process.rs` | `SpawnManagedOptions::inherited_process_group()` enables redirections; `apply_redirections` opens typed paths |
| Detached `process.spawn` | `src/runtime/eval/lowered_run.rs`, `src/runtime/process.rs` | calls `spawn_command`, whose detached options set all stdio to null and `apply_redirections = false` |
| Owned handle lifecycle | `src/runtime/eval/process_handle.rs` | records spawn/wait/cancel handle lifecycle |
| Normative process contract | `docs/SPEC.md` | documents Command fields and says managed spawn uses command redirections |
| OS ownership contract | `docs/SPEC-OS.md` | defines child-group ownership, cancellation boundaries, and structured redirection failures |
| Proposal corpus | `LANG.md` | still says spawn lacks stderr support |
| Native XSH coverage | `tests/xsh/stdlib/process.xsh` | covers `process.run` command redirection, not `spawn command` stderr |
| Host/runtime coverage | `tests/runtime/process.rs` | owns process execution and cancellation integration coverage |
| Fluency task contract | `fixtures/fluency/{TASK,REFERENCE}.md` | actor-visible owned-spawn task and reference pack, with no XSH checkout |
| Fluency fixture judge | `judges/run-fluency-task-evaluator.sh` | materializes the opaque workspace and checks F01–F03 plus boundary/contamination controls |
| C1 curation fixture | `fixtures/curation/c1-valid/` | normalized candidate account relations and frontier members |
| Curation contract judge | `judges/run-curation-contract-judge.sh` | validates C1 shape, source admission boundary, exclusions, and raw escalation naming |
| Uptake fixture | `fixtures/uptake/` | exact L1 delivery/encounter/application records and contamination controls |
| Uptake application judge | `judges/run-uptake-application-judge.sh` | preserves propagation-state separation and required four-record comparison |
| W1 frontier fixture | `fixtures/frontier/w1-valid/` | positive allowlist and explicit aftermath sequestering classes |
| Frontier leakage judge | `judges/run-frontier-leakage-controls.sh` | exhaustive principal/route/sequestered-class denial matrix |

The likely current result is H3: `process.run`, managed `spawn command`, and
`spawn run` honor stderr policy, while detached `process.spawn` discards stdio
and does not apply command redirections. This circuit exists to verify that
claim from behavior and citations without a provider call.
