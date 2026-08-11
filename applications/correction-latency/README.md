# Correction-latency laboratory

This is Society's first experimental world. It studies whether access to
pre-replacement chronological Forum history changes how a fresh population
incorporates identical corrective evidence.

The world is deliberately synthetic. Each episode has hidden ground truth and
a fixed collection of bounded evidence cards. No actor receives enough private
evidence to reconstruct the whole answer alone. Actors may communicate only
through Society's recorded boundaries.

At the intervention point every actor is terminated and replaced with a fresh
instance drawn from the same policy distribution. One matched arm can read the
old Forum Thread. The other begins after its old head. Only then does a
deterministic service publish the same correction to both arms. Prompts, tools,
evidence, timing, and resource ceilings remain matched.

The desired outcome is a valid comparison, not evidence that retained memory
helps. Faster correction, slower correction, persistent error, and no
measurable difference are all legitimate results.

See [`VERTICAL-SLICE.md`](VERTICAL-SLICE.md) for the protocol.
[`LIVE-STUDY-PLAN.md`](LIVE-STUDY-PLAN.md) fixes the candidate native runtime,
the authorized $0.25 staged allocation, the two-pair feasibility pilot, and
the gates for a separately authorized substantive study.

## Provider-free acceptance run

The isolated deterministic paired run is available without a provider call:

```text
cargo run --manifest-path applications/correction-latency/Cargo.toml -p correction-latency-harness
```

From the repository root, the same provider-free run and its end-of-world
status report are available as:

```text
make run-society
```

The report includes per-arm actor, Forum, study-budget, measurement, baseline,
and replay-integrity facts. Monetary cost is explicitly reported as not
applicable: this fixture makes no provider calls and has no provider-backed
agent cost to inspect. Live Pi Office cost is a separate ledger-backed path.

For a machine-readable application-owned artifact, pass `--analysis-tsv`:

```text
SOCIETY_POSTGRES_TEST_URL='postgresql://...'
  cargo run --manifest-path applications/correction-latency/Cargo.toml \
    -p correction-latency-harness -- --analysis-tsv
```

The TSV retains every raw retained/reset value and derivation or missingness
digest, emits persisted pair/episode/revision/randomization provenance when
available, emits the paired retained-minus-reset delta, and summarizes
observed counts, unavailable/invalidated counts, means, and two-sided 95%
paired Student-t intervals. Monetary and amortized institutional costs are
explicit missing values in this provider-free fixture; a live runner supplies
those same two closed metric slots rather than treating absent billing as zero.
Persisted-pair conversion rejects a pair whose arms disagree on the sealed
protocol/world/measurement/institution/randomization contract, whose eleven
measurement slots are not exact, or whose closure/reveal/replacement evidence
is incomplete. A malformed typed result remains an explicit invalidated value;
it is never turned into zero.
The provider-free command intentionally emits an artifact without a
`PreregisteredAnalysisPlan`. A live scientific runner must construct the
application-owned plan with independent world-seed digests, pair identities,
the retained-minus-reset estimand, metricwise complete-case exclusions, and
per-metric precision targets, then call
`AnalysisArtifact::from_preregistered_plan` before rendering results.
The CLI uses `PairedReport::persisted_analysis_artifact`, which maps the
kernel's read-only persisted-pair query into CL-001's closed metric vocabulary.
For a live runner, collect one `PairObservation` per preregistered matched pair
and render the artifact only after the raw observations are fixed.

A daemon-owned live runner reads its sealed run identity and ordinal pair set
through `SocietyctlClient::study_run_observation`, then retrieves each arm pair
through `SocietyctlClient::study_pair_observation`. Pass those normalized
observations to `LiveRunPlan::analysis_artifact_from_study_run`. That
application-owned gate first proves that the daemon retained the exact sealed
CL-001 plan, then requires a terminal completed run and joins
every application-owned pair label and world-seed digest to the exact
registered generic pair. It rejects a post-hoc plan, order, or seed
substitution. Neither client operation grants a PostgreSQL connection or
returns application private-content bytes.

The live admission contract is application-owned in
`correction-latency-harness::LiveRunDescriptor` and
`correction-latency-harness::LiveRunPlan`. Build the descriptor with
`LiveRunDescriptor::canonical(ActorPolicyIdentity::new(...))`, supplying the
pre-registered policy, model/runtime, and sampling-contract digests. Build at
least two `PairSeed` values with distinct pair IDs and independently generated
seed digests, then call `LiveRunPlan::new` with the eleven precision targets.
The plan fixes the canonical eight source and eight successor role seats,
their private-view and prompt digests, F0 Forum contract, world/evidence/
correction/truth identities, three baseline identities, budgets, estimand, and
metricwise complete-case rule. An application adapter seals
`plan.admission_bytes()` through immutable content custody and submits only
the resulting content identity plus `plan.sealed_digest()` to a generic
daemon; it does not import this crate, PostgreSQL, or CL-001 semantics. An
application adapter can use `descriptor().source_roles()` and
`successor_roles()` to submit generic actor-obligation admissions and verify
that each runtime prompt/private view matches the pre-registered contract.

It admits the canonical eight-role source and successor populations in both
arms, freezes the source head, proves reset-history denial and source-authority
loss, releases the same correction through one matched service transition,
reveals the protocol-committed truth only after all actors terminate, records
typed measurements, and validates fresh materialized-state replay. It is not a
live Pi/native-child study; those custody facts remain prerequisites for a
separately admitted live profile.

The paid adapter qualification smoke retains a separate host-bound
`qualification-artifact.json` beside its raw session transcripts. That
artifact preserves provider usage/cost, exact in-memory Forum occurrences, and
read-rendering digests for audit and adapter analysis. Its sealed status marks
PostgreSQL and daemon custody as absent and the CL-001 lifecycle as not
executed; it must not be pooled with the persisted paired study artifact.
