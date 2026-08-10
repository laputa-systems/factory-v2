# Correction-latency laboratory guide

This directory owns the CL-001 experimental world. Root documents own generic
authority, process, content, experiment, and replay contracts.

## Rules

- Keep world ground truth, evidence-card semantics, actor-local obligations,
  fixtures, and measurement interpretation here.
- Do not import or depend on `societyd`, `societyctl`, SQLite, process custody,
  or generic mutation authority.
- A future application crate may consume public generic domain/content types;
  it cannot write the ledger or choose native executable paths.
- Ordinary tests are deterministic, provider-free, and network-free.
- Test the null result and missing-data paths as carefully as a measured effect.
- Never preserve actor-local context across replacement. Only an admitted
  institutional-memory record may reach successor actors.
- No application file may mention a product repository, patch, or software-
  delivery objective. This is a synthetic information-propagation study.

## Owned documents

- `README.md` states the world in plain language.
- `VERTICAL-SLICE.md` is the exact CL-001 protocol and acceptance plan.

No implementation is authorized until the generic episode/treatment boundary
is defined. Do not fake that authority in application-local storage.
