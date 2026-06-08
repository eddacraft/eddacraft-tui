# Post-merge: feat-tuir-010-mirror-drift-check

PR: #NNN
Branch: `feat/tuir-010-mirror-drift-check`
APS: TUIR-010 (module: tui-reintegration)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Dispatch the new workflow on `main`:
      `gh workflow run mirror-drift-check.yml --ref main`, then confirm the run
      concludes **success** with the step summary reading
      "**Result: clean**". This is the live green run TUIR-008's drift-check
      validation line ("mirror drift check (D-TUIR-018) reports a clean tree")
      consumes. (agent: yes — `gh run list`/`gh run view`)
- [ ] Step 2 — Flip TUIR-010 from `In Progress` to
      `Merged YYYY-MM-DD via PR #NNN` in
      `plans/modules/tui-reintegration.aps.md`, bump the module header and
      index row `8/10 → 9/10`, and confirm `node scripts/aps/index-counts.mjs
      --check` stays green. (agent: yes)
- [ ] Step 3 — Start the TUIN 7-consecutive-green-runs counter: TUIN's Ready
      Checklist requires the drift check green for 7 consecutive runs before
      any TUIN task may be promoted to `Ready`. From the first post-merge
      scheduled/dispatch run onward, this clock is running. No action now —
      noted so the next TUIN planning pass checks
      `gh run list --workflow mirror-drift-check.yml` for ≥7 green. (human:
      tracking only)

## Notes

- The workflow's drift algorithm was verified locally before merge: the
  reconstructed subtree tree (`bb6cba45…`) was byte-identical to
  `eddacraft/eddacraft-tui:main`'s tree (green), and a simulated out-of-band
  mirror commit was correctly flagged (red).
- The first scheduled run lands at 05:27 UTC daily. The propagation-lag step
  means a run that coincides with an in-flight mirror push will report
  "propagation lag" (neutral, non-failing) rather than drift — that is expected
  behaviour, not a failure, and does NOT count as a non-green run for the TUIN
  gate's purposes (it is a skip, not a red).
- Deferred follow-up (Council F-002): the two workflows' transforms are kept in
  sync by a comment contract, not a CI gate. A tree-equality lint (or revisiting
  the shared-script option) is tracked in the TUIR-010 item; not blocking.
