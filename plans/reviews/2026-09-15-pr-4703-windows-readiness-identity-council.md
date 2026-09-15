# Council Review — PR #4703 Windows readiness identity

**Status:** Converged
**Tier:** full
**Target:** `origin/main...HEAD` on `fix/cib-419-windows-readiness-identity`
(protected: `crates/anvil-intercept/src/registry.rs`)
**Date:** 2026-09-15
**PR:** https://github.com/eddacraft/anvil-001/pull/4703
**Contract:** GitHub issues #4702 (CIB-419: dunce-plain and leftover `\\?\`
worktree paths are one location; doctor fails closed on a failed readiness
component; workspace list distinguishes live membership from `--persist`
register-on-start) and #4701 (CIB-420: `anvil validate` accepts canonical
`## Work Items` and legacy `## Tasks`, without claiming `aps lint` parity).
The CIB module file is not edited on this feature PR. Workspace-package
specifier resolution is out of scope.
**Session:** `council-9ad220cb` (assurance / full)
**Code head reviewed:** `81c6d0a7ea22ffe2f942be48750ccbcc9118709f` (bounded
repair after the first pass). Protected `registry.rs` last changed in
`3ac2ad6ce403a5bc12b57b9eb27f783b1fdc792c`. Later commits on this branch, if
any, must be this review only; re-council if `registry.rs` moves after the
label.

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `registry.rs` | Register/lookup via `dunce::canonicalize`; stored keys compared with `path_identity::same_path` | Leftover `\\?\` keys still match dunce-plain queries; missing-dir `sessions_for_worktree` stays empty (ADR-090) |
| `path_identity.rs` | Local mirror of CLI `same_path` / `strip_verbatim_prefix` / `relative_to` | Intercept cannot depend on CLI; one identity algorithm |
| `status.rs` | Claim/filter use `same_path`; `build_status` overlays fence/cascade/driver with `same_path` | Windows fences no longer miss dunce sessions; claims stay `DegradedProtection` when fenced |
| `registration_store.rs` / `lib.rs` prune | Upsert/remove/prune use `same_path` | Leftover verbatim on-disk keys update and reap correctly |
| CLI doctor / status / validate | Doctor uses `measured_readiness` and fails closed; validate requires a heading line | Matt's false ready / heading mismatch |

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general | repair then approve | First pass: `build_status` fence/cascade overlay still used `PathBuf` equality. After `same_path` overlay + mixed-form test: identity holds. |
| adversarial | repair then approve | Independent corroboration of the overlay miss on every Windows fence, not only leftover rows. Heading-line APS check and missing-dir fail-closed look sound. |
| security | approve | Comparison is bounded to spelling variants of the same location. No auth/IPC/save-time/confinement widening. Prefix boundaries and Unix case-sensitivity fail closed. |
| operations | approve (GO) | Query, deleted-dir split, and doctor fail-closed are ready. Reaper prune `PathBuf` equality was a leftover-disk shadow; repaired with `same_path`. |
| pragmatic | approve | `path_identity.rs` is a justified local mirror, not a second algorithm. Host-free plain-vs-verbatim tests are enough for the contract. |
| **judge** | **Ship** (decision PASS, gate PASS) | First pass `REPAIR`. Bounded overlay/prune repair applied on `81c6d0a7e`. No remaining `must_fix`. |

## Findings

### In-contract must_fix (fixed on `81c6d0a7e`)

- **C-001** (general + adversarial): `build_status` joined dunce session keys to
  `std::fs::canonicalize` fence/cascade records with `HashSet`/`HashMap`
  equality (`status.rs` overlay). A fenced Windows worktree could claim
  `PreWriteDaemon`. Repair: overlay with `path_identity::same_path`. Test:
  `build_status_overlays_legacy_verbatim_fence_onto_plain_session`.

### In-contract should_fix (fixed on `81c6d0a7e`)

- **C-002** (operations): `prune_registrations` used `Vec::contains` on
  `PathBuf`, so a leftover verbatim on-disk key could survive a dunce
  in-memory reap. Repair: filter with `same_path`.

### Consider (not must-fix)

- `fence.rs` still *writes* `std::fs::canonicalize` keys. Overlay `same_path`
  is sufficient for mixed-form identity. Switching fence persist to dunce is
  optional later work, not required for #4702.

## Evidence

- `cargo test -p eddacraft-anvil-intercept --lib` (path identity, claim,
  filter, overlay, registry match) — 8 passed after the repair.
- `cargo clippy -p eddacraft-anvil-intercept --tests -- -D warnings` — exit 0.
- Prior CLI tests on `3ac2ad6ce`: activation verbatim snapshot promotion and
  APS heading-line check — 7 passed.

## Label

Maintainer applies `council:reviewed` to the **live** PR head after this
review file lands. Do not label a stale SHA. The label is dismissed on
synchronize.
