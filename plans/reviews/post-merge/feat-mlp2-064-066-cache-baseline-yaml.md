# Post-merge: feat-mlp2-064-066-cache-baseline-yaml

PR: <!-- filled on push -->
Branch: `feat/mlp2-064-066-cache-baseline-yaml`
APS: MLP2-064, MLP2-065, MLP2-066
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Advance MLP2-064 to `Merged` in
      `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [ ] Advance MLP2-065 to `Merged` in
      `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [ ] Advance MLP2-066 to `Merged` in
      `plans/modules/multilayer-protection-v2.aps.md` (agent: yes)
- [ ] Bump Group M progress counter from `3/6` (after PR1 merges) to
      `6/6 (Complete)` in the module's `## Stats` table and the
      `index.aps.md` mirror (agent: yes)
- [ ] Confirm `pnpm adr:check` still reports `47` indexed ADRs after the
      module-status update (agent: yes)
- [ ] **Calendar reminder:** review ADR-046 (YAML parser deferral) on
      `2026-08-15`. Owners: `kernel-maintainer`, `security-analyst`.
      Triggers for earlier review: new `serde_yaml` advisory, or a
      maintained drop-in surfacing (human required)

## Notes

This PR closes Group M (the full-codebase Council corrective follow-ons
filed after the 2026-05-15 audit):

- **MLP2-064** (High) — `RuleSetCache` now tracks a per-worktree
  invalidation token that survives entry removal. `get_or_resolve`
  snapshots the token before resolving outside the lock and refuses
  to insert the freshly-resolved entry when the token has advanced —
  closing the stale-reinsert window the module-level comment
  acknowledged before the fix.
- **MLP2-065** (High) — partial baselines now carry a
  `pre_cursor_fingerprint` (SHA-256 of the sorted pre-cursor relative
  paths). Resume runs recompute the hash and force a restart on
  drift; older partials without the field also restart conservatively.
  Operator messaging distinguishes "file list changed" from "no
  scannable files visible at resume time" so the restart reason is
  honest.
- **MLP2-066** (Medium) — ADR-046 defers the `serde_yaml` →
  maintained-parser migration with a recorded owner pair (kernel-
  maintainer + security-analyst) and a 2026-08-15 review date. The
  byte-level pre-pass (size cap, alias reject, depth cap) is the
  load-bearing defence; a migration today would churn 14 typed-config
  readers for minimal correctness gain.

Council quick review surfaced one MAJOR finding (TOCTOU on the
stale-insert counter via double-lock pattern) and three MINORs (token-
bump comment scope, case-insensitive FS limitation, empty-tree
operator message). All four were addressed before push; the
`insert_if_generation_unchanged` body now uses a single lock scope so
the logged fields can't drift relative to one another under
concurrent rejections.
