# Post-merge: fix-rlb-per-save-check-scope

PR: #2184
Branch: `fix/rlb-per-save-check-scope`
APS: RLB (RLB-001, RLB-007)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — On a quiet 16-core box, build release `anvil` and run
      `bash scripts/bench/load-ramp.sh --agents 1,2,4` against it and against
      the `v0.7.3-beta` baseline binary; confirm steady-state `check` CPU drops
      well below the baseline tipping point (~7 cores/agent → expect a large
      reduction for single-file churn). (agent: no — needs quiet hardware)
- [ ] Step 2 — On macOS, run `anvil watch` in a temp repo under `/tmp` (which
      resolves through the `/private/tmp` symlink), save a file, and confirm the
      dispatched `anvil check <path>` resolves and scans that file (no
      "No files specified" / no full `--all` walk). (agent: no — needs macOS)
- [ ] Step 3 — On a real multi-file repo, run bare `anvil watch`, save one
      file, and confirm via the process list that the child is
      `anvil check <abs-path>` (scoped), not `anvil check --all`; then delete a
      file and confirm the delete still triggers a full `--all` re-walk.
      (agent: no — interactive)

## Notes

RLB-007 surfaces the changed file's absolute path on the internal
`EventPayload::Snapshot.changed_path` field (kept OFF the
`anvil.watch.event.v1` wire envelope) so the CLI dispatcher scopes the per-save
`anvil check` to the saved file instead of re-walking the whole repo. Empty
scope (deletes, initial scan) falls back to `--all`. `gate` is unchanged
(self-scopes via git status). The load-ramp harness (RLB-001) is the
before/after measurement tool; wiring it into CI + per-process SLOs is the
follow-on RLB-008. Cross-platform perf coverage is the separate RLB-006.
