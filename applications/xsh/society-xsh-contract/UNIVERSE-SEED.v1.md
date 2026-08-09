# XSH Universe Seed, revision 1

Make XSH a practical, coherent, easy-to-learn, token-efficient, and trustworthy systems-glue language for humans and coding agents, capable of replacing fragile Unix glue with typed paths, explicit process and effect boundaries, structured streams and errors, reproducible execution, and inspectable policy while preserving the composability that makes Unix systems useful.

## Domain scope and non-goals

XSH is a clean-slate systems scripting language for modern Linux userspace: strong glue between processes, files, paths, byte streams, structured data, and system state.

XSH is not a POSIX compatibility shell, an interactive terminal, or a claim to be the best general application runtime.

## Preserved Unix properties

- Coarse-grained composability.
- Ordinary files and visible process boundaries.
- Pipeline flow.
- The ability for a script to grow into a tool.

## Principles

- Make ordinary process composition typed and discoverable.
- Prefer executable, reproducible evidence over narrative confidence.
- Preserve explicit command fields, owned child lifecycles, structured setup errors, default stream inheritance, and ordinary path sinks.
- Reject quoting puzzles, ambient state, implicit evaluation, text-only boundaries, and stacked private DSLs.
- Revise the direction when bounded reviews or real use-site evidence contradict the current contract.

## North-star questions

1. What XSH capability or actor behavior would change?
2. What evidence distinguishes a general improvement from a local workaround, movement of complexity, or noise?
3. How does the change honor clarity, explicit boundaries, composability, and XSH's systems-glue scope?
4. At which review, replay, outcome horizon, or Grand Architect decision will the claim be revisited?

## Active Grand Architect Office contract

`TheGrandArchitect` is the highest constitutional Office and the final decision authority inside the running XSH society. Its occupant may be a user or an assigned coding agent; durable authority comes only from authenticated occupancy and exact capability grants.

Its reserved powers are to ratify or amend the active seed; govern Projects and resource envelopes inside hard ceilings; govern subordinate Offices and organization configurations; decide or accept risk for consequential changes; require review, postmortem, replay, or outcome observation; resolve documented conflicts and exceptions; reopen preserved work; and designate a successor.

The Office cannot write raw SQL, mutate the content store directly, access secrets outside an execution profile, forge evidence, alter prior events, create unreserved spend, force an invalid state transition, or deploy a replacement kernel through an ordinary command.
