# XSH application boundary

This workspace owns the XSH-specific VS-001 process circuit and its closed
observation adapter. The application boundary keeps XSH APIs, source claims,
fixtures, and evaluator contracts out of generic society crates.

```text
applications/xsh/
├── Cargo.toml                         isolated XSH Rust workspace
├── society-xsh-circuit/               parsing-only XSH VS-001 adapter
├── society-xsh-contract/              typed mission/alignment input factory
└── circuits/vs-001-spawn-stderr/      deterministic XSH fixtures and judges
```

`society-xsh-circuit` may use the generic `society-content` byte-seal type by
path. It does not make an observation authoritative: it cannot admit evidence,
write a ledger, schedule, settle, or disclose. The circuit judges likewise
produce deterministic observations beneath their caller-selected `--out`
directory; the durable authority remains outside this application workspace.

Run the adapter from this workspace:

```text
cargo test -p society-xsh-circuit
cargo clippy -p society-xsh-circuit --all-targets -- -D warnings
```

The application-owned evaluator invocation and its provider-free contracts are
documented in `circuits/vs-001-spawn-stderr/README.md`.

Application ownership is recorded in [`AGENTS.md`](AGENTS.md). The preserved
application architecture, canonical vocabulary, executable VS-001 contract,
and dependency boundary are [`ARCHITECTURE.md`](ARCHITECTURE.md),
[`GLOSSARY.md`](GLOSSARY.md), [`VERTICAL-SLICE.md`](VERTICAL-SLICE.md), and
[`DEPENDENCIES.md`](DEPENDENCIES.md). The root documents define only the
generic authority. `society-xsh-contract` now owns the concrete XSH mission and
alignment inputs, but this workspace cannot admit them or assign their durable
revision identity. The daemon-private sealed mission-source binding is
implemented; authorized product-output binding remains open. The source binding
is byte custody only, not provenance, semantic admission, or an end-to-end XSH
execution claim.
