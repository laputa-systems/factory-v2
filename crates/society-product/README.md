# `society-product` — isolated guarded materialization core

This temporary standalone workspace implements only the provider-free local
Git mechanics in Milestone 6. It is not a durable authority and does not claim
Milestone 6 completion.

The public boundary accepts already-authorized identities and returns typed
receipts for:

- clean source qualification at a local `refs/heads/*` head;
- a builder-owned branch `ProductWorktree`, portable binary patch, exact base
  and candidate trees, and changed paths;
- one temporary-index snapshot sealed as an immutable Git tree, from which the
  portable binary patch and changed-path relation are both derived;
- fresh-worktree patch application and exact-tree verification, followed by
  either bounded internal Git validation or a typed externally supervised
  XSH/Xsht validation receipt bound to that prepared worktree/tree; and
  `git commit-tree` construction with explicit author, committer, timestamp,
  and message inputs;
- guarded local compare-and-swap fast-forward, with an exact no-rebase refusal
  on a moved target head. If the branch CAS succeeds but the checkout update
  fails, the core returns `DeliveryCheckoutRecoveryRequired` and does not
  silently repair the checkout on an idempotent retry; the supervisor must
  record and explicitly recover that exact delivered commit first; and
- worktree-cleanup evidence, idempotent already-delivered receipts, and a
  non-mutating descendant reopen directive.

Every Git call is argv-based with a caller-supplied absolute Git path, an empty
ambient environment, disabled global/system config, disabled replace refs and
hooks, disabled fsmonitor/global attributes/external diff, no text conversion,
and no remote operation. Repository config that enables a filter or included
configuration is rejected. Git stdout (32 MiB), stderr (1 MiB), patch artifacts
(32 MiB), and operation time (30 seconds) are bounded. The timeout owns only
the directly spawned, trusted Git executable; this standalone crate has no
process-group supervisor and does not claim to terminate arbitrary descendants
of a compromised Git binary.

Product-generated qualifications, worktree states, captures, validation,
commit, materialization, and delivery receipts have private fields with narrow
read-only accessors. Delivery also independently rechecks every
authorization-to-tree/patch/profile/validation/commit edge before its target
branch CAS, so a receipt reconstructed by a future persistence boundary cannot
mix an authorization from one candidate with another candidate's tree.

The core deliberately does not spawn XSH or Xsht: without a process-group and
cancellation owner it could not safely impose a deadline or bound descendant
processes. `prepare_materialization` exposes a single-use prepared-worktree
cleanup responsibility;
`finalize_materialization` accepts an exact typed
`ExternallySupervisedValidationReceipt` from the eventual supervisor. The
receipt is structurally bound to the profile and tree, but its authenticity and
the supervisor's process/liveness evidence remain a future `societyd` concern.
Dropping `PreparedMaterialization` deliberately performs no cleanup because a
destructor cannot return durable cleanup evidence: its holder must call exactly
one of `finalize_materialization` or `abandon_prepared_materialization` and
persist the resulting receipt/evidence.

Run the provider-free contract suite from the repository root:

```text
cargo test --manifest-path crates/society-product/Cargo.toml
cargo clippy --manifest-path crates/society-product/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path crates/society-product/Cargo.toml --check
```

The tests require a local `/usr/bin/git` (the current macOS host path) and
create/retire only uniquely named temporary repositories. They never access or
modify the authoritative XSH checkout.

Deliberate omissions for later `societyd`/kernel integration: SQLite-backed
workflow authority and idempotency, capabilities/C2 authorization, content
object sealing, XSH build provisioning, budget/cancellation supervision,
outcome scheduling, projections, external-validation receipt authentication and
process-group/liveness evidence, and any remote delivery.
