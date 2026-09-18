# Council Review — PR #4791 Windows path identity for Nightly Cross smoke

**Status:** Converged
**Tier:** full
**Target:** `origin/main...HEAD` on `fix/windows-path-identity-nightly`
(protected: `crates/anvil-intercept/src/registry.rs`,
`crates/anvil-intercept/src/save_time.rs`,
`crates/anvil-intercept/src/workspace_admission.rs`,
`crates/anvil-intercept/src/confinement.rs`)
**Date:** 2026-09-18
**PR:** https://github.com/eddacraft/anvil-001/pull/4791
**Contract:** Residual of CIB-419 / #4703. After registry keys became
dunce-plain, fence / rule-cache / save-time / driver maps still keyed or
compared with `std::fs::canonicalize` (verbatim `\\?\` on Windows). Nightly
Cross (`x86_64-pc-windows-msvc`) smoke on `eddacraft-anvil-intercept --lib`
failed ~15 tests at `0c7669a31` because PathBuf equality and HashMap lookups
missed. Production must persist and look up with `dunce::canonicalize`, and
join leftover verbatim records with `path_identity::same_path`. When save-time
moves to dunce, admission and confinement allow-entry keys must follow or
Windows Allowlist/GCTX and mixed-verb connections split. No auth driver-binary
policy change; IPC unchanged; CIB-414 nested-repo residual unchanged.
**Session:** `council-pr4791-full` (assurance / full)
**Code head reviewed:** repair tip including admission/confinement dunce align
(after `66d4deec1` fence-fixture repair and `e54b9c24f` rustfmt). Re-council if
protected paths move after the label.

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `fence.rs` | `canonicalise_worktree` / `lookup_path` → dunce; matches + fanout → `same_path`; fixtures expect dunce | Fence/cascade join registry sessions on Windows |
| `rule_cache.rs` | `WorktreeKey` + invalidate parent → dunce | Unregister hooks invalidate the same key the cache stores |
| `save_time.rs` | `canonical_root`, `authorise_gctx_root`, `set_originating_session` → dunce; test fixtures → dunce | GCTX / assurance keys match registry identity |
| `save_time_driver.rs` | `driver_map_key` (dunce) on handle / `driver_status` | Driver status no longer looks absent after register |
| `registry.rs` | Test expectations: dunce forms | Test-only; production registry already dunce from #4703 |
| `workspace_admission.rs` | Admission API + graph-root helper canonicalize → dunce | Admitted map keys match save-time on Windows |
| `confinement.rs` | Allow-entry canonicalize → dunce | Allowlist `permits` matches dunce GCTX/save-time keys |
| `unregistered.rs` / `lib.rs` / `watcher.rs` / `status.rs` | Test expectations / same_path; status comment | Align fixtures; leftover verbatim disk only |
| Docs | ARCHITECTURE + downstream freshness | diagram-impact owns `src/**` |

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general | repair then approve | Production call sites switch to dunce + `same_path` for the #4703 residual. Pass 1: fence fixtures still expected `Path::canonicalize` (`66d4deec1`). Pass 2: save-time dunce while `AdmittedRoots` / confinement allow entries stayed on `std::fs::canonicalize` — Windows plain vs `\\?\` map/allowlist split. |
| adversarial | repair then approve | HashMap miss on verbatim vs plain is the Nightly failure mode. Mixed `authorise_within_budget` (std) vs GCTX dunce caller on one connection double-admits or false-refuses on Windows; graph-root `starts_with` breaks if helpers re-canonicalise with std against a dunce input. |
| security | approve | Dunce only strips the verbatim prefix; OS resolution unchanged. Allowlist stays explicit-entries-only. No auth/IPC policy widening. CIB-414 nested-repo residual unchanged. |
| operations | approve (GO) | Unregister → rule-cache invalidate, fence fanout, and driver status share registry keying. Admission/confinement follow save-time so Open and Allowlist stay coherent on Windows. |
| pragmatic | approve | Narrow residual of #4703 plus the necessary admission/confinement spelling align. No new features. |
| **judge** | **Ship** (decision PASS, gate PASS) | Two REPAIR passes. Bounded repairs landed on reviewed tip. No remaining must_fix. |

## Findings

### In-contract must_fix (fixed on reviewed tip)

- **C-001** (general + adversarial): `fence.rs` fixtures still used
  `worktree.path().canonicalize()` while production persists dunce-plain.
  Repair at `66d4deec1`: expect `dunce::canonicalize` in cascade/observation
  fixtures; refresh status overlay comment.
- **C-002** (operations): `status.rs` comment still claimed fence persists
  `std::fs::canonicalize`. Repair at `66d4deec1`.
- **C-003** (general + adversarial): `save_time` production identity moved to
  `dunce::canonicalize` while `workspace_admission::AdmittedRoots` and
  confinement allow-entry canonicalize still used `std::fs::canonicalize`.
  On Windows, GCTX admit stores plain `C:\...` and non-GCTX
  `authorise_within_budget` / Allowlist `permits` use `\\?\C:\...` — PathBuf
  map equality misses (double-admit, false refuse, budget skew). Repair:
  admission API + graph-root helpers + confinement allow-entry canonicalize
  → dunce; align save_time test fixtures.

### Consider (not must-fix)

- `auth.rs` driver allowlist / workspace-roots claim still uses
  `Path::canonicalize` on both sides of the comparison; both sides stay
  consistent. Optional later: dunce or `same_path` for session-root claims.
- `driver_map_key` falls back to the raw path when canonicalize fails — same
  as prior callers; missing-dir empty lookup remains intentional.

## Evidence

- `cargo fmt -p eddacraft-anvil-intercept -- --check` — clean.
- `cargo test -p eddacraft-anvil-intercept --lib` — 1235 passed (one
  unrelated ipc traceparent flake observed once, passed on retry).
- `cargo clippy -p eddacraft-anvil-intercept --tests -- -D warnings` — exit 0.
- Focused path_identity + workspace_admission + gctx_handlers — 35 passed.

## Label

Maintainer applies `council:reviewed` to the **live** PR head after this
review file lands with the admission/confinement repair. Do not label a
stale SHA. The label is dismissed on synchronize.
