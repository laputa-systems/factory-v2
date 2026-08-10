# Society engineering guide

## Working contract

Implementation is cheap; ambiguity is not. Spend care where meaning becomes
durable: names, types, schemas, interfaces, state transitions, permissions,
tests, and explanations. Let implementation be a candidate. Let the contract
and the evidence around it be what survives.

- Prefer closed Rust enums, identifier newtypes, normalized SQLite tables, and
  narrow protocols which make invalid state difficult to express.
- JSON is permitted only at the Pi SDK-host boundary. The Rust control protocol
  and SQLite schema must not gain generic payloads, metadata maps, EAV tables,
  or stringly typed discriminants.
- Test observable transitions and cross-boundary invariants. Favor integration,
  replay, fault-injection, process, and counterfactual experiment tests over
  trivial units.
- Add no dependency casually. Keep features narrow, pin exact dependencies,
  and document why a dependency belongs in trusted physics.
- Do not run pre-commit hooks. Never push a remote.

## Research direction

Society is an institutional model organism: a controlled laboratory for asking
how persistent institutions change the collective behavior of replaceable,
individually weak actors. The models are experimental components. The durable
research object is the control plane which allocates authority and resources,
routes information, preserves memory, promotes claims, and survives actor
replacement.

The central empirical question is:

> Under the same actor-policy distribution and resource budget, how does
> accumulated institutional state change collective capability, error, and
> recovery?

The immediate objective is not autonomous software production or recursive
self-improvement. The first vertical slice is a small synthetic correction-
latency experiment under actor replacement. Every new durable mechanism must
name the experiment or invariant which requires it.

Keep these distinctions explicit:

- **ledger replay** reconstructs and validates one accepted history;
  **experimental replay** reruns a fork from declared state under a changed
  treatment;
- **provenance** records declared production and dependency lineage;
  **causality** requires intervention, ablation, randomization, or another
  identified design;
- **mission** states normative purpose, **measurement** reports observations,
  and **selection policy** chooses what is retained or reproduced;
- **platform ground truth** is never overwritten by institutional belief;
- **system-mediated observability** covers messages, authority, resources, and
  artifacts, but does not claim access to a model's latent cognition; and
- **actor state** dies with the actor unless an admitted institutional
  transition preserves it.

## Authority documents

- `RESEARCH-PROGRAM.md` owns the research thesis, hypotheses, and sequencing.
- `GLOSSARY.md` owns generic vocabulary and implementation-status labels.
- `ARCHITECTURE.md` owns the trust, society, experiment, and application
  boundaries.
- `FORUM.md` owns the staged communication substrate, Pi awareness contract,
  and explicit deferrals.
- `VERTICAL-SLICE.md` owns the generic path required by the first experiment.
- `applications/correction-latency/VERTICAL-SLICE.md` owns the exact synthetic
  world, treatment arms, measurements, controls, and falsification criteria.
- `DEPENDENCIES.md` owns generic dependency allowance.

An experimental world beneath `applications/<experiment>/` owns its world
semantics, fixtures, actor obligations, measurements, and analysis. It depends
only on generic public boundaries. Generic crates, manifests, and documents do
not import or identify a particular experiment.

## Code map

```text
README.md                  project entry point
RESEARCH-PROGRAM.md        institutional model-organism research program
AGENTS.md                  engineering and research contract
ARCHITECTURE.md            generic four-plane architecture
GLOSSARY.md                canonical generic vocabulary
FORUM.md                   chronological Forum baseline and deferred research
VERTICAL-SLICE.md          generic CL-001 execution requirements
DEPENDENCIES.md            exact dependency contract

Cargo.toml                 generic Rust workspace manifest
crates/society-kernel/     trusted identities, ledger, state, and authority
crates/society-content/    immutable byte custody and bounded verified reads
crates/society-pi/         typed Rust peer for the SDK-host boundary
crates/societyd/           resident authority and native-child custody
crates/societyctl/         public query and supervisor-stream client
crates/society-product/    dormant guarded Git materialization mechanics;
                           outside the current research slice
migrations/                one canonical fresh-schema bootstrap
packages/society-pi-host/  replaceable actor-runtime adapter
tests/                     generic boundary, daemon, and supervision judges
applications/              isolated experimental worlds and their semantics
var/                       ignored runtime database, objects, and workspaces
```

The root workspace intentionally excludes application workspaces. An
application may consume generic public crates by path or published interface;
it does not become a generic workspace member merely by living here.

## Current boundary

The existing implementation provides useful trusted physics: typed mission
and project alignment, capabilities, budgets, cancellation, append-only events,
content custody, native process ownership, Pi session receipts, deterministic
evaluator custody, and integrity replay. These mechanisms do not yet constitute
an institutional experiment.

In particular, the code does not yet provide first-class study protocols,
episodes, treatment assignment, population snapshots, institution revisions,
interventions, measurements, or experimental forks. Current evaluator evidence
is deliberately `ApplicationSemanticsUninterpreted`. Current replay validates
history; it does not rerun a counterfactual episode. The resident serving loop
does not yet execute the deterministic-evaluator or evidence-admission path.
There is no Forum storage, read/post tool, exposure frontier, Forum prompt
contract, or read receipt yet.

Do not relabel these omissions as implemented by reusing a nearby type. Add a
new contract only after its authority and role in CL-001 are explicit.

## Nearest hard judges

Run the narrowest relevant judge first, then broaden:

```text
cargo test -p society-kernel --test <contract>
cargo test -p society-pi --lib
npm test --prefix packages/society-pi-host
cargo test -p society-content
cargo test -p societyd --lib native_child
cargo test -p societyd --lib deterministic_evaluator -- --test-threads=1
cargo test -p societyd --test supervision -- --test-threads=1
tests/generic-boundary/run-no-application-knowledge.sh

cargo test --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo fmt --all --check
git diff --check
```

Provider calls are never part of an ordinary test. A live weak-actor study
requires a separately admitted runtime profile, fixed population and budget,
pre-registered protocol, and retained raw observations.
