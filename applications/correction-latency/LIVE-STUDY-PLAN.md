# CL-001 staged live-study plan

## Status

This is a sealed planning protocol, not a native-runtime qualification, paid
execution, or scientific result. The native Pi profile is still unqualified,
and the canonical daemon-owned paired runner does not yet exist. Therefore no
current artifact answers either CL-001's retained-versus-reset treatment
question or the broader institutional-state question.

study_program.rs keeps three stages separate: an optional cheapest paid adapter
smoke, a feasibility pilot, and a later substantive study. A result from one stage is
never silently repurposed as evidence from a later stage.

## Fixed candidate treatment

Every pilot and substantive pair must use this candidate actor distribution:

| Field | Fixed value |
| --- | --- |
| provider | OpenRouter |
| model | inclusionai/ling-2.6-flash |
| thinking | off |
| Pi SDK | 0.84.1 |
| tool profile | forum_isolated_v1 |
| Forum | F0 prompt and two digest-bound Forum tools only |
| sampling | PINNED_ACTOR_MODEL_POLICY_V1, including retry, compaction, queue, telemetry, and image settings |

CanonicalLiveRuntimeProfile commits this fixed text plus the exact Node version,
Node executable, Pi host lockfile, Pi host build, transitive package set, and
saved model-catalog digests. ActorModelPolicy::PinnedOpenRouterLing26FlashOff is
a distinct durable population identity; it does not qualify the native
execution profile or permit a provider call on its own.

## Authorized spend

The authorized total is exactly 250,000 micro-USD ($0.25).

| Stage | Allocation | Fixed topology | Per-actor ceiling |
| --- | ---: | --- | ---: |
| Cheapest paid adapter smoke | 50,000 micro-USD ($0.05) | 16 noncanonical adapter actors | 3,125 micro-USD ($0.003125) |
| Feasibility pilot | 200,000 micro-USD ($0.20) | 2 pairs × 2 arms × 16 source/successor actors = 64 lifetimes | 3,125 micro-USD ($0.003125) |
| Substantive study | no allocation | selected only after pilot analysis | requires new approval |

For the pilot, one arm/episode is capped at 50,000 micro-USD and one
retained/reset pair at 100,000 micro-USD. AuthorizedPilotBudget contains the
exact integer arithmetic; there is no floating-point allocation or remainder.
The adapter-smoke runner rounds a binary64 cost upward to micro-USD before
applying its guardrail.

## Stage protocol

### 1. Optional cheapest paid adapter smoke

make run-society-cheapest-paid-smoke is a fixed $0.05 adapter smoke. It is pinned to the
candidate treatment above and rejects a provider/model override. It retains
qualification-artifact.json with provider usage, cost, Forum calls, artifact
digests, and integer guardrails.

Its topology remains one in-memory Forum shared across 16 reduced-role cells;
treatment labels are report metadata only, and it has neither PostgreSQL nor
daemon custody. It can expose adapter failure, provider availability, runtime
drift, or cost infeasibility. It cannot qualify the generic native profile yet
and cannot estimate a CL-001 treatment effect. It is not a release gate: run
it only when that adapter diagnosis is needed, and never enter it in a CL-001
estimator.

### 2. Feasibility pilot

After a real native qualification path and canonical runner exist, construct a
LiveRunPlan with exactly two independently generated pair IDs and world-seed
digests. Wrap it in FeasibilityPilotPlan; construction rejects any other pair
count or a plan under a different runtime identity.

For each pair, run one retained and one reset arm using the exact eight source
and eight successor roles. Preserve the source freeze, successor-exposure
barrier, atomic matched correction release, terminal actor disposal, truth
reveal, typed measurements, and replay validation in VERTICAL-SLICE.md. Keep
the eleven pre-registered metric slots, complete-case rule,
retained-minus-reset estimand, and precision targets in the sealed LiveRunPlan.

Retain raw receipt/transcript/content identities, the sealed plan and runtime
digests, pair/seed registration, both cost measurements, every unavailable or
invalidated state, and the planned analysis artifact. The pilot is a
feasibility and variance observation, not a confirmatory estimate.

### 3. Substantive study

Analyze the closed pilot only under its pilot plan. Use observed paired variance
and the declared per-metric precision targets to choose a new finite pair count
and request a new spend authorization. Before new outcomes are visible, seal a
new LiveRunPlan and construct SubstantiveStudyPlan with the pilot
analysis-artifact digest.

SubstantiveStudyPlan rejects a changed runtime actor policy, a reference to
another pilot, and reuse of either a pilot pair ID or world seed. It deliberately
does not assign a sample size, confidence claim, or spend ceiling. Those are a
post-pilot, separately authorized decision.

## Release gates before a canonical pilot

1. The dependency lock, Pi host build, and Rust peer agree on the sealed
   0.84.1 runtime identity.
2. The native profile obtains an actual trusted qualification transition and
   retained qualification evidence. Its current Unqualified state is never
   relabeled by an adapter artifact.
3. The daemon exposes a public generic admission/scheduling route that can
   seal the application plan, construct all task starts, drive paired barriers,
   and close the run. `DaemonComposition` now reaches the narrow in-process
   `StudyAdmissionAuthority`: it can seal the opaque `LiveRunPlan` and accept
   only closed generic `StudyCommand` transitions without handing the
   application a database, content writer, or process handle. It is not yet a
   TaskAttempt scheduler or paired-barrier runner, so this gate remains open.
4. A fresh PostgreSQL schema at society-kernel-postgres-schema-v12, content
   custody, daemon replay, and all focused provider-free judges pass.
5. The exact pilot seeds, pair labels, runtime artifact identities, analysis
   targets, and $0.20 pilot cap are sealed before pilot actors are admitted.

Until all five gates hold, do not treat the adapter smoke, provider-free
harness, or a planning digest as live-study data.
