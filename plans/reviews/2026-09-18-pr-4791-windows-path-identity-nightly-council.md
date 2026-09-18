# Council Review — PR #4791 Windows path identity for Nightly Cross smoke

**Status:** Converged
**Tier:** full
**Target:** `origin/main...HEAD` on `fix/windows-path-identity-nightly`
(protected: `crates/anvil-intercept/src/registry.rs`,
`crates/anvil-intercept/src/save_time.rs`)
**Date:** 2026-09-18
**PR:** https://github.com/eddacraft/anvil-001/pull/4791
**Contract:** Residual of CIB-419 / #4703. After registry keys became
dunce-plain, fence / rule-cache / save-time / driver maps still keyed or
compared with `std::fs::canonicalize` (verbatim `\\?\` on Windows). Nightly
Cross (`x86_64-pc-windows-msvc`) smoke on `eddacraft-anvil-intercept --lib`
failed ~15 tests at `0c7669a31` because PathBuf equality and HashMap lookups
missed. Production must persist and look up with `dunce::canonicalize`, and
join leftover verbatim records with `path_identity::same_path`. No
auth/confinement widening; workspace admission and IPC unchanged.
**Session:** `council-pr4791-full` (assurance / full)
**Code head reviewed:** `66d4deec1c3857b0c690b9f3f313bade0922a776` (bounded
repair after the first pass). Protected `save_time.rs` production identity
changed in the feature commit; `registry.rs` test expectations only. Later
commits on this branch, if any, must be this review only; re-council if
`registry.rs` or `save_time.rs` production paths move after the label.

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `fence.rs` | `canonicalise_worktree` / `lookup_path` → dunce; `FenceRecord` / `CascadeRecord::matches` + fanout filter → `same_path`; fixture expects dunce | Fence/cascade join registry sessions on Windows; leftover verbatim disk rows still match |
| `rule_cache.rs` | `WorktreeKey::canonicalise` + invalidate parent → dunce | Unregister hooks invalidate the same key the cache stores |
| `save_time.rs` | `canonical_root`, `authorise_gctx_root`, `set_originating_session` → dunce | GCTX / save-time admission keys match registry identity |
| `save_time_driver.rs` | `driver_map_key` (dunce) on handle / `driver_status` | Driver status no longer looks absent after register |
| `registry.rs` | Test expectations: dunce forms | Test-only; production registry already dunce from #4703 |
| `unregistered.rs` / `lib.rs` / `watcher.rs` / `status.rs` | Test expectations / same_path lookup; status comment | Align fixtures; document leftover verbatim only |
| Docs | ARCHITECTURE + downstream freshness | diagram-impact owns `src/**` |

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general | repair then approve | Production call sites correctly switch to dunce + `same_path` for the #4703 residual. First pass: fence observation / cascade fixtures still expected `Path::canonicalize` while `fence_worktree` persists dunce — Windows Nightly display/equality miss. Repaired at `66d4deec1`. |
| adversarial | repair then approve | HashMap miss on verbatim vs plain is the Nightly failure mode; `driver_map_key` unwrap_or raw path matches prior canonicalize miss semantics. Leftover verbatim fence files still matched via `same_path`. No admission bypass. |
| security | approve | Comparison stays location-identity only. No auth, IPC, confinement, or workspace-admission widening. |
| operations | approve (GO) | Unregister → rule-cache invalidate, fence fanout, and driver status share registry keying. Stale status comment corrected. |
| pragmatic | approve | Narrow residual of #4703; clippy GCTX test split is test-only. Docs freshness is bookkeeping. |
| **judge** | **Ship** (decision PASS, gate PASS) | First pass REPAIR. Bounded repairs landed on reviewed tip. No remaining must_fix. |

## Findings

### In-contract must_fix (fixed on reviewed tip)

- **C-001** (general + adversarial): `fence.rs` fixtures still used
  `worktree.path().canonicalize()` while production `canonicalise_worktree`
  persists dunce-plain. On Windows Nightly, `include_paths=true` observation
  assert compared display strings to a verbatim form. Repair: expect
  `dunce::canonicalize` in the four cascade/observation fixtures.
- **C-002** (operations): `status.rs` comment still claimed fence persists
  `std::fs::canonicalize` after this PR moved fence to dunce. Repair: comment
  now describes leftover verbatim disk rows only.

### Consider (not must-fix)

- Unrelated `save_time.rs` tests still call `std::fs::canonicalize` when
  building local `WorktreeKey` fixtures. Fine on Linux; only matters when
  asserted against production keys on Windows — out of this Nightly residual
  unless Cross smoke names them.
- `driver_map_key` falls back to the raw path when canonicalize fails — same
  as prior `fs::canonicalize` callers; missing-dir empty lookup remains
  intentional.

## Decision

PASS — ship after `council:reviewed` on the live tip that includes the bounded
repair and this evidence commit only (no further protected-surface edits after
the label).
