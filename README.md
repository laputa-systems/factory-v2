# Society

Society is an institutional model organism: a controlled laboratory for
studying how persistent institutions change the collective behavior of weak,
replaceable AI actors.

The project treats the control plane—not an individual model—as the primary
research object. Models perform narrow local obligations. Durable institutions
control information flow, authority, resources, memory, claim promotion, and
correction. The laboratory asks whether those structures create capabilities
or failures which the same actors do not exhibit alone.

**Individual agents should be cheap and disposable. Do not make actors
increasingly stateful; make the society increasingly stateful.** Anything worth
retaining must leave an actor-local session through a typed, attributable,
challengeable institutional transition.

The initial research claim is deliberately modest:

> Holding actor policy and inference budget fixed, retained institutional state
> measurably changes how a population incorporates a delayed correction after
> every actor is replaced.

The first project is the synthetic, provider-free-first correction-latency
study described in
[`applications/correction-latency/VERTICAL-SLICE.md`](applications/correction-latency/VERTICAL-SLICE.md).
It is not a software factory and does not attempt recursive self-improvement.

## Architecture at a glance

```text
experimental world
  known ground truth, evidence cards, application measurements
                         |
experimental control
  protocol, episode, treatment, population, intervention, fork
                         |
institutional substrate
  actors, work, claims, evidence, memory, propagation, policy
                         |
trusted physics
  ledger, content, authority, budget, process custody, replay
```

Only the bottom layer is substantially implemented today. The next work is to
make one complete experiment real, not to fill in a general-purpose swarm
framework.

## Documents

- [`RESEARCH-PROGRAM.md`](RESEARCH-PROGRAM.md) — thesis and research sequence
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — generic boundaries and invariants
- [`GLOSSARY.md`](GLOSSARY.md) — canonical terms and implementation status
- [`VERTICAL-SLICE.md`](VERTICAL-SLICE.md) — generic requirements for CL-001
- [`AGENTS.md`](AGENTS.md) — engineering contract
- [`DEPENDENCIES.md`](DEPENDENCIES.md) — trusted dependency allowance

Historical application work is intentionally absent from the current research
line. This branch starts from the smaller institutional experiment.
