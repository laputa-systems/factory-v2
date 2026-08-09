# Generic dependency contract

This file records the complete dependency allowance for the generic trusted
implementation. A version appearing here is permission to use it only in the
named boundary; it is not a reason to add it to every crate. An application
records any additional dependency decision beneath `applications/<product>/`.
Adding another direct dependency requires an explicit contract decision and an
update here.

## Rust workspace

Every direct Rust dependency is exact-pinned in `[workspace.dependencies]` and
the resolved transitive graph is committed in `Cargo.lock`.

| Crate | Exact version | Allowed responsibility |
| --- | ---: | --- |
| `rusqlite` | 0.40.2 | Sole SQLite binding in `society-kernel`; the canonical fresh schema is embedded SQL executed by the kernel rather than an ORM or migration framework. |
| `thiserror` | 2.0.20 | Closed, inspectable error enums at trusted boundaries. |
| `sha2` | 0.11.0 | SHA-256 identities for immutable content, command bodies, revisions, trees, execution artifacts, and the resident physical content store. |
| `tracing` | 0.1.44 | Typed spans and lifecycle events in `societyd`. |
| `tracing-subscriber` | 0.3.23 | Mandatory monitor and bounded diagnostic rendering; only `fmt`, `registry`, and `std` features are enabled. |
| `serde` | 1.0.229 | Serialization derives only in `society-pi`, at the closed Pi SDK-host protocol boundary. |
| `serde_json` | 1.0.151 | JSONL and closed submission parsing only in `society-pi`; SQLite and the local daemon protocol remain non-JSON. |
| `libc` | 0.2.177 | Narrow Unix process-group, signal, peer-credential, ownership, and content-store file-lock calls. The stable 0.2 line is used instead of the 1.0 prerelease. |

`rusqlite` disables its default feature set and enables `bundled`. This pins the
SQLite implementation through `libsqlite3-sys`, avoids dependence on the host's
SQLite feature/version drift, and does not enable Rusqlite's JSON support. Rust
crate sources are registry-resolved and lockfile-pinned, not copied into this
repository.

`society-content` and `society-product` are root-workspace members.
`society-content` provides physical byte seals; `society-product` provides
guarded local materialization mechanics, but does not yet bind a receipt to
resident authority. Product-specific observation adapters belong in isolated
application workspaces and may depend on the public content-identity boundary
by path. Separate lockfiles with the same exact dependency versions grant no
daemon or kernel authority.

The workspace deliberately has no async runtime, ORM, workflow framework,
process-control framework, tracing appender, UUID crate, time/date crate, or
generic schema/validation framework. Identifier generation, clocks, codecs,
state transitions, supervision, and canonical schema bootstrapping are trusted
kernel contracts rather than delegated policy.

## TypeScript SDK host

`packages/society-pi-host/package.json` and its exact npm lockfile own the
JavaScript dependency surface. The production dependency is
`@earendil-works/pi-coding-agent` 0.83.0. The build/test-only dependencies are
`typescript` 5.9.3 and `@types/node` 24.12.4. All three direct dependencies are
exact-pinned in `package.json`; the full transitive graph is integrity-pinned
in `package-lock.json`. `node_modules` and compiled `dist/` output are never
committed.

The package's engine floor is Node 22.19.0 because that is Pi 0.83.0's runtime
contract. That range is not execution-profile authority: qualification records
and admits one exact Node executable version and digest, and the Rust
supervisor rejects any other executable before a paid session is constructed.

The host imports the SDK directly and calls `createAgentSession()`. It does not
shell out to the Pi CLI. It may serialize JSON only across its versioned,
closed stdin/stdout boundary and Pi's canonical session files. It receives no
SQLite, Git, capability, budget, scheduling, or cancellation-policy authority.

### Paid-admission advisory gate

The exact Pi 0.83.0 lock currently resolves `undici` 8.5.0 and
`brace-expansion` 5.0.7. `npm audit` reports high-severity advisories against
both. `undici` is imported by Pi's HTTP-dispatcher path and is therefore treated
as reachable during a real provider session; the provider-free construction
and test suites do not exercise that path. `brace-expansion` is reached through
Pi's package/model discovery dependencies, which this host disables, but it
remains part of the admitted package graph.

This does not authorize a silent transitive override or a move away from Pi
0.83.0. Native qualification and every paid attempt remain blocked until the
authorized office explicitly chooses a dependency resolution, the lock delta is
reviewed, and the full host/Rust-peer qualification suite is rerun. The known
fixed transitive releases are `undici` 8.9.0 and `brace-expansion` 5.0.9; those
versions are candidates for that decision, not current execution authority.

## Upgrade rule

An upgrade is a contract change. It must record the reason, inspect the direct
and transitive delta, update both lockfiles where relevant, rerun the nearest
boundary and replay judges, and create a new typed execution-profile revision
before a paid session can use it. A lockfile refresh with no reviewed contract
change is not accepted.
