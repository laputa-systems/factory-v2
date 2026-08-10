# XSH application boundary

This workspace owns the XSH-specific VS-001 process circuit and its closed
observation adapter. The application boundary keeps XSH APIs, source claims,
fixtures, and evaluator contracts out of generic society crates.

```text
applications/xsh/
├── Cargo.toml                         isolated XSH Rust workspace
├── society-xsh-circuit/               XSH VS-001 adapters and C1 direct candidate
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

`society-xsh-circuit` contains the compiled
`vs001-direct-evaluator-adapter` binary. Its only direct-executable candidate
is the self-contained `CurationContract` evaluator: it accepts exactly
`--input-manifest <verified absolute path>`, reads one bounded length-framed
file, evaluates the seven fixed curation TSV members in Rust, and writes the
canonical two-member curation output package to stdout. Its fixed members are
the aggregate curation observation and the raw-evidence-escalation observation,
so a named request retains its question/object identity rather than being
reduced to a flag. The adapter has no child process,
shell, external path, XSH/Xsht binary, source checkout, host-tool, durable ID,
receipt, or authority.

The other seven VS-001 judges, scripts, fixtures, and external-artifact
requirements remain wholly application-owned and are explicitly pending the
direct profile. No application evaluator is registered or scheduled yet: a
future generic bridge may seal the direct adapter and this one manifest file,
then invoke the fixed ABI, but that bridge and evidence admission remain
separate.

The generic direct-executable custody coordinator and claim path are
daemon-private and provider-free integration-tested. They have no resident
serving-loop call site, and no XSH evaluator is registered or scheduled, so
this application construction cannot enter them yet.

`interpret_curation_direct_stdout_v1` is the matching application-owned output
port for that one direct C1 candidate. Given only completed stdout bytes and a
declared BLAKE3 value, it bounds the rendering, rejects a byte mismatch, and
maps the exact two-member curation package grammar to the closed
`CurationDirectSemanticResultV1` vocabulary. It does not receive a custody
identifier, path, process result, receipt, or generic authority, and its
`Accepted` value is not evidence admission. This pure interpreter is an
application consumer/check of the direct evaluator's canonical package, not a
second process or a generic authority.

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
