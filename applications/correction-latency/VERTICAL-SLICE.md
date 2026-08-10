# CL-001: correction latency through a disposable-actor Forum

## Research question

Holding actor policy, evidence, Forum tools and prompt, correction publication,
role topology, and total resource budget fixed, does exposing fresh actors to
the pre-replacement chronological Forum history change correction adoption
latency or final correctness?

This is a two-sided question. Retained history may help, harm, or have no
measurable effect.

## World

One world instance contains:

- a hidden binary proposition committed before treatment assignment;
- eight signed evidence cards whose joint relation determines the proposition;
- an early plausible false claim from a declared high-authority synthetic
  source;
- one exact corrective evidence package;
- one episode Forum with one chronological discussion Thread; and
- a deterministic ground-truth evaluator unavailable to actors.

No actor view contains enough cards to establish the proposition alone. The
world validator proves that structural condition before episode admission.

Evidence cards and the correction are immutable, content-addressed, and
episode-scoped. A synthetic signature describes the world source; it grants no
generic capability or epistemic authority.

## Population

Each arm admits a source population of eight cheap, disposable actors from one
fixed weak actor-policy revision:

- four observers, each receiving one private card;
- two challengers, each receiving a disjoint Forum/read obligation;
- one synthesizer asked to relate available claims and conflicts; and
- one decision actor receiving only its bounded Forum view.

Every source actor terminates. A fresh population with the same role
distribution and actor-policy revision is admitted after replacement. Roles
are obligations and visibility rules, not personalities or durable identities.

Provider-free tests use deterministic actor doubles. A later live pilot pins
one exact weak model/runtime, sampling contract, generic Forum prompt revision,
application role-prompt revision, tool schema, and aggregate budget.
Provider-free and live results are never pooled.

## Forum baseline

CL-001 uses Forum F0 from the root `FORUM.md`:

- immutable chronological Messages;
- custody-derived actor-occurrence authorship;
- Finding, Question, Challenge, Correction, and Synthesis kinds;
- explicit reply and supersession links;
- explicit bounded `society_forum_read` and `society_forum_post` tools;
- exact exposure frontiers and read receipts; and
- deterministic untrusted-content rendering.

There is no subscription, notification, live `Steer`, rank, consensus badge,
reputation, karma, reference curator, persistent member persona, or global
feed.

The generic Forum-awareness system-prompt fragment is byte-identical across
arms. It explains public durability, untrusted peer content, tool use, and
bounds. The application role fragment differs only by pre-registered role, not
treatment. Mutable Messages never enter a system prompt.

## Timeline

```text
t0  admit matched episodes, assignments, Forums, prompts, and source populations
t1  source observers inspect private cards and post local findings/questions
t2  source challengers read bounded ranges and post challenges
t3  source synthesizer posts; source decision actor records early belief
t4  freeze exact pre-replacement Thread head
t5  close every source obligation, Pi session, actor authority, and native child
t6  admit fresh populations and atomically install Retained or Reset exposure
t7  deterministic service publishes the identical correction into both Threads
t8  successor actors read/post under the same role topology and budgets
t9  successor decision actor records final decision
t10 derive measurements, close all authority/resources, replay each arm
```

The paired harness waits for complete replacement and exposure installation in
both episodes before issuing one `ReleaseMatchedCorrection` transition. That
single service-custodied ledger transition validates both ready states, writes
the same correction bytes into both Threads, and advances both arms' eligible
frontiers atomically. It removes correction-timing divergence without creating
cross-arm actor authority.

## Treatment arms

The matched pair differs in exactly one field.

### Retained

Successor Forum exposure starts at ordinal 1. Fresh actors may read the exact
pre-replacement Thread, including authorship, replies, challenges,
supersessions, and the later correction.

### Reset

Successor Forum exposure starts after the frozen pre-replacement Thread head.
Fresh actors may read the later correction and subsequent Messages but cannot
obtain earlier Messages through Forum reads, content aliases, transcript
construction, search, or another role's context.

The reset intervention never deletes history. Experiment authority retains it
for audit, measurement, and integrity replay. It is simply outside successor
actor authority.

Both arms have distinct episode, actor, Forum, Message, authority, budget, and
result identities. Matched fixtures and deterministic services establish
equivalence; identity reuse does not.

## Baselines

Before interpreting an institutional effect, run:

1. isolated actors with the same private views and no Forum tools;
2. an unstructured population with ephemeral direct exchange and no durable
   Forum history; and
3. fresh actors with the reset chronological Forum exposure under the same
   aggregate budget.

The report states actor-level and baseline uncertainty. “Emergent” is permitted
only if a pre-registered criterion excludes these baselines.

## Raw occurrences

Retain at minimum:

- ground-truth commitment and later authorized reveal;
- actor-policy, role, private view, prompt/tool revision, budget, and
  replacement identities;
- exact Forum charter, Thread head, exposure frontier, and rendering revision;
- every Message body digest, author occurrence, kind, ordinal, reply,
  supersession, and retraction;
- every Forum read range and exact returned rendering digest;
- deterministic correction publication;
- actor completion, expiry, session disposal, process reconciliation, and
  closure;
- early and final decisions with uncertainty; and
- measurement inputs, exclusions, unavailable facts, and invalidations.

No actor or analysis step can delete or rewrite raw occurrences. A measurement
excludes an occurrence only through a named pre-registered rule.

## Measurements

### Primary

- `CorrectionAdoptionLatency`: admitted steps from deterministic correction
  publication to the first decision-relevant corrected Forum statement or
  final belief.
- `FinalDecisionCorrect`: whether the final decision matches hidden truth.

### Secondary

- `FalseClaimPersistence`: whether the early false claim remains unrebutted or
  cited without supersession at closure.
- `CorrectionVisibility`: fraction of successor roles for which the correction
  entered an eligible and returned read range before acting.
- `DissentSurvival`: whether a valid pre-replacement challenge remains readable
  and is consulted in the retained arm.
- `ForumHistoryUtilization`: pre-replacement Messages explicitly cited,
  replied to, challenged, or superseded by successors divided by visible
  pre-replacement Messages.
- `ForumAttentionCost`: returned Forum bytes, actor turns, and runtime cost.

Every measurement has `Observed`, `Unavailable`, or `Invalidated`. No missing
value defaults to zero or a favorable outcome.

## Analysis

The engineering acceptance run proves exact paired execution over deterministic
fixtures and makes no statistical claim. The first live scientific run
pre-registers independent world seeds, pair count, exclusion rules, estimand,
and precision before inspecting results.

Report raw arm values, the paired retained-minus-reset difference, missingness,
and confidence interval. Report both operational and amortized institutional
cost when retained history was produced before the measured interval.

## Falsification and invalidation

The institutional-effect hypothesis is not supported when retained and reset
arms are indistinguishable within pre-registered precision. The result remains
scientifically useful.

An episode pair is invalid rather than negative if:

- treatment or Forum history leaks before successor exposure admission;
- any source actor, Pi session, private context, capability, or child survives
  replacement;
- reset successors obtain a pre-replacement Message;
- Forum prompt/tool bytes, correction bytes/timing, actor policies, evidence,
  or budgets differ unexpectedly across arms;
- actors observe ground truth outside their cards;
- a Forum Message silently becomes authority, evidence, or knowledge;
- a Message visibility/read distinction is missing;
- measurement code changes after outcomes are visible; or
- isolated actors already reliably exhibit the target behavior.

## Explicit omissions

CL-001 has no software repository, product patch, endogenous institution
mutation, actor reproduction, RL update, open network, hidden collaboration
channel, autonomous scientific conclusion, live Forum interrupt, subscription,
rank, consensus, reputation, karma, or attention currency.

Authority amplification, alternative propagation topology, reputation, karma,
ranking, and interruption are later matched protocols. They are not optional
fields which may silently enter this baseline.

## Implementation order

1. Canonical world fixtures, evidence partition validator, and ground-truth
   evaluator.
2. Generic protocol, episode, treatment, population, and institution identity.
3. Forum F0 identity, Message, exposure, read receipt, and replay contracts.
4. Exact Forum Pi prompt fragment and explicit read/post tool boundary.
5. Replacement plus retained/reset exposure transaction.
6. Post-replacement deterministic correction publication.
7. Measurement derivation and independent integrity replay.
8. Deterministic paired harness and manipulation negative controls.
9. Only then, a separately authorized weak-model pilot.
