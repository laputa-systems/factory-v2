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
- The application may use generic public crates for typed mission/alignment and
  eventually product authorization. It must not write SQLite, own the resident
  control channel, supervise Pi, reap children, assign capabilities, settle
  budgets, move a delivery ref, import `societyd` or `societyctl`, or receive a
  `ContentObjectId` for its mission source.
- `society-xsh-contract` owns the XSH mission rendering and constructs the
  generic typed mission/alignment inputs. It returns only
  `ApplicationMissionInput`, canonical bounded `MissionSourceRendering` bytes,
  and their BLAKE3 digest; it does not admit them, assign durable identifiers,
  or receive daemon authority. The daemon-private path checks the digest,
  side-effect-free preflights the outer command, seals the bytes, records the
  receipt/object chain, and lets the kernel derive the mission's private object
  binding. The supervisor carries this typed rendering only beside
  `InstallFoundingMission`; it has neither generic content mutation nor
  content-writer authority. Deterministic internal operation identities make
  the content primitive retry-stable while the daemon authority is retained;
  they do not resume a failed supervisor handler. The request ends on handler
  failure, and restart is `RecoveryFenced`, not source-operation recovery. This
  custody boundary establishes neither provenance nor semantic/evidence meaning.
  Durable authorized product-change output remains unimplemented. The XSH
  VS-001 contract records its intended end-to-end gates; it does not claim that
  the present foundation completed them.
- No application JSON, metadata map, EAV relation, or stringly discriminator
  may be introduced as a shortcut around a typed evaluator or generic boundary.
- The XSH evaluator port owns the canonical VS-001 evaluator programs,
  application profile names, invocation grammar, fixture/case manifests,
  expected output contracts, and semantic parsers. It constructs only bounded
  canonical program/input renderings, their declared BLAKE3 identities, and a
  closed application invocation description. A later daemon-private bridge may
  revalidate those artifacts against an already durable generic admission, use
  the native custody core, and seal fully reaped output before invoking a
  separate closed evidence path. No such resident scheduler path exists yet.
  The port cannot choose a durable child identity, invoke or supervise a child,
  seal bytes, write SQLite, or admit evidence; generic crates must not name XSH
  evaluator semantics.

## Application map

```text
AGENTS.md                         XSH ownership and boundary rules
ARCHITECTURE.md                   preserved XSH application architecture/history
GLOSSARY.md                       XSH canonical vocabulary
VERTICAL-SLICE.md                 exact XSH VS-001 executable contract
README.md                         isolated workspace entry point
society-xsh-circuit/              closed XSH observation adapter and evaluator port
society-xsh-contract/             XSH mission and north-star input factory
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
cargo test -p society-xsh-contract
cargo clippy -p society-xsh-contract --all-targets -- -D warnings
tests/run-boundary.sh
```

The application circuit README names its provider-free evaluator invocation
and external prerequisites. Do not run a provider as part of ordinary tests.
