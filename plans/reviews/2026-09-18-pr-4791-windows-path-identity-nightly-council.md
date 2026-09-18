# Council Review — PR #4791 Windows path identity for Nightly Cross smoke

**Status:** Converged
**Tier:** full
**Target:** `origin/main...HEAD` on `fix/windows-path-identity-nightly`
(protected: `crates/anvil-intercept/src/registry.rs`, `save_time.rs`)
**Date:** 2026-09-18
**PR:** https://github.com/eddacraft/anvil-001/pull/4791
**Contract:** CIB-419 residual after #4703 / #4702. Registry and registration
keys are already dunce-plain. Nightly Cross (`x86_64-pc-windows-msvc`) smoke
still failed ~15 `anvil-intercept` tests because fence, rule_cache, save_time
(gctx/originating session), and save_time_driver maps persisted or compared with
`std::fs::canonicalize` (verbatim `\\?\` on Windows). Align those maps on
dunce persist/lookup and `path_identity::same_path` for leftover mixed forms.
Workspace-package and `workspace_admission` git-parity canonicalize are out of
scope for this residual.
**Session:** `council-pr4791-full` (assurance / full)
**Code head reviewed:** `ba51c772ef95b3224fbeade28265720cdb23cd3c`
(feature + clippy/docs cascade + governance triage). Protected `registry.rs`
diff is test-expectation-only (dunce); protected `save_time.rs` production
change is `dunce::canonicalize` for `set_originating_session`,
`canonical_root`, and `authorise_gctx_root`. Later commits on this branch
after the label must be this review only; re-council if `registry.rs` or
`save_time.rs` production paths move after the label.

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `fence.rs` | Persist/lookup via `dunce`; `FenceRecord`/`CascadeRecord`/`session` filter via `same_path` | Leftover verbatim fence files still match dunce-plain registry keys |
| `rule_cache.rs` | `WorktreeKey::canonicalise` + parent invalidation use `dunce` | Unregister hooks and cache keys share one Windows identity |
| `save_time.rs` | Originating session, `canonical_root`, `authorise_gctx_root` use `dunce`; GCTX TOCTOU test split for clippy | GCTX/admission keys match registry dunce form |
| `save_time_driver.rs` | `driver_map_key` via `dunce` for map get/spawn/stop | Membership hooks no longer miss drivers under plain vs `\\?\` |
| `registry.rs` | Test expectations use `dunce::canonicalize` | No production registry algorithm change on this PR |
| `unregistered.rs` / `watcher.rs` / `lib.rs` | Test lookups/expectations use `dunce` + `same_path` | Host-free identity assertions |
| ARCHITECTURE + downstream docs | Freshness cascade for `src/**` | diagram-impact satisfied; diagrams unchanged |

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general | approve | Completes the #4703 contract for the maps Nightly actually missed. Persist and lookup now share dunce; leftover mixed forms still join via `same_path`. |
| adversarial | approve | No widening of admission or fence triggers. Failed canonicalize still fails closed (originating session cleared; fence path errors unchanged). Driver fallback to raw path on canonicalize failure matches prior unwrap_or pattern — dual-key risk only when the path vanishes mid-flight, same as before. |
| security | approve | Comparison stays location-identity only. No auth/IPC/confinement/save-time policy change. Prefix boundaries unchanged. |
| operations | approve (GO) | Driver map, fence overlay, and rule-cache invalidation now agree with registry keys on Windows. Clippy `too_many_lines` / `implicit_clone` and diagram-impact cascade cleared on the reviewed tip. |
| pragmatic | approve | Residual scope is tight: the four Nightly-hot maps, not a wholesale `workspace_admission` rewrite. Test helper split for the GCTX TOCTOU case is justified against `clippy::too_many_lines`. |
| **judge** | **Ship** (decision PASS, gate PASS) | No `must_fix`. Considers recorded below; none block Nightly residual land. |

## Findings

### In-contract must_fix

None on `ba51c772e`.

### Consider (not must-fix)

- **C-001** (general): `workspace_admission.rs` and some `confinement.rs`
  paths still call `std::fs::canonicalize`. Git-parity and ACL admission are
  outside this Nightly residual; do not expand scope here. File a follow-on
  only if Cross smoke shows admission keys diverging from dunce registry keys.
- **C-002** (pragmatic): many `save_time.rs` unit fixtures still build
  `WorktreeKey` via `std::fs::canonicalize`. On Unix dunce ≡ std; on Windows
  Cross those fixtures may need dunce if they assert `PathBuf` equality
  against production keys. Not observed as a Nightly miss for this tip; leave
  unless Cross fails those tests after land.

## Evidence

- `cargo clippy -p eddacraft-anvil-intercept --all-targets -- -D warnings` —
  exit 0 on deus after the clippy split / `PathBuf::clone` fix.
- `cargo test -p eddacraft-anvil-intercept --lib path_identity` — 4 passed.
- `node scripts/docs/check-diagram-impact.mjs --since origin/main --head HEAD`
  — 0 errors after ARCHITECTURE + overview / save-to-validation /
  trust-and-deployment-boundaries + governance triage cascade.
- Contract continuity with #4703 council
  (`plans/reviews/2026-09-15-pr-4703-windows-readiness-identity-council.md`):
  this PR implements the prior "fence persist to dunce" consider for the
  Nightly-failing maps.

## Label

Maintainer applies `council:reviewed` to the **live** PR head after this
review file lands. Do not label a stale SHA. The label is dismissed on
synchronize.
