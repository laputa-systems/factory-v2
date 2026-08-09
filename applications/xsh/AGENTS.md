# XSH application engineering guide

This directory owns the XSH application contract. The repository-root
[`AGENTS.md`](../../AGENTS.md), [`ARCHITECTURE.md`](../../ARCHITECTURE.md), and
[`VERTICAL-SLICE.md`](../../VERTICAL-SLICE.md) define the generic authority
which this application may consume but may not modify through application code.

## Application contract

- Use spellings from [`GLOSSARY.md`](GLOSSARY.md). XSH-specific concepts do not
  become generic concepts merely because an evaluator emits them.
- Keep source claims, XSH APIs, fixtures, evaluators, validation profiles, and
  product conclusions inside this application directory.
- The application may use generic public crates for content identity and
  eventually typed mission/alignment and product authorization. It must not
  write SQLite, own the resident control channel, supervise Pi, reap children,
  assign capabilities, settle budgets, or move a delivery ref.
- The current generic mission/alignment input and durable authorized
  product-change output remain unimplemented. The XSH VS-001 contract records
  its intended end-to-end gates; it does not claim that the present foundation
  has completed them.
- No application JSON, metadata map, EAV relation, or stringly discriminator
  may be introduced as a shortcut around a typed evaluator or generic boundary.

## Application map

```text
AGENTS.md                         XSH ownership and boundary rules
ARCHITECTURE.md                   preserved XSH application architecture/history
GLOSSARY.md                       XSH canonical vocabulary
VERTICAL-SLICE.md                 exact XSH VS-001 executable contract
README.md                         isolated workspace entry point
society-xsh-circuit/              closed parsing-only XSH observation adapter
circuits/vs-001-spawn-stderr/     XSH fixtures and deterministic judges
```

The source and evaluator history belongs here even where the current generic
kernel has not yet integrated its results. The generic boundary records only
typed IDs, sealed content, authority, and receipts; successful parsing or a
passing application judge never admits evidence or authorizes delivery by
itself.

## Nearest application judges

```text
cargo test -p society-xsh-circuit
cargo clippy -p society-xsh-circuit --all-targets -- -D warnings
```

The application circuit README names its provider-free evaluator invocation
and external prerequisites. Do not run a provider as part of ordinary tests.
