# Post-merge: feat-scan-004-files-skipped-by-ignore

PR: #NNN
Branch: `feat/scan-004-files-skipped-by-ignore`
APS: SCAN-004
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Advance SCAN-004 `In Progress → Merged YYYY-MM-DD via PR #NNN` in
      `plans/modules/scan-performance.aps.md`, and update the index row note in
      `plans/index.aps.md` (count moves 3/5 → 4/5). (agent: yes)
- [ ] `cargo test -p eddacraft-anvil --bins gitignore_skip` passes on `main`
      (the three temp-git-repo counting tests). (agent: yes)
- [ ] `cargo test -p eddacraft-anvil-tui --lib discovery` passes on `main`
      (render show/omit + filter propagation). (agent: yes)
- [ ] Manual smoke (human required): in a repo with a gitignored directory
      holding a source file, run the welcome discovery flow and confirm the
      continue screen shows "N file(s) skipped by .gitignore — set
      ANVIL_SCAN_ALL=1 to scan them"; then re-run with `ANVIL_SCAN_ALL=1` and
      confirm the note disappears. (human required)

## Notes

Scope is deliberately the welcome/`ScanResults` discovery surface only — it is
the sole secret-scan path that honours `.gitignore` (`standard_filters(!scan_all)`).
gate/audit/check/drift/policy/baseline use `standard_filters(false)` by design,
so the count would always be 0 there; do NOT thread it through `SecretCheckResult`.

The count is derived from the set difference between the gitignore-blind Phase 1a
walk and the gitignore-respecting scanned set, and is suppressed to 0 when the
scan is truncated by the file cap or when `ANVIL_SCAN_ALL` is set.
