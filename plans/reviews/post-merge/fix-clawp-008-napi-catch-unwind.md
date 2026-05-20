# Post-merge: fix/clawp-008-napi-catch-unwind

PR: #TBD
Branch: `fix/clawp-008-napi-catch-unwind`
APS: CLAWP-008 (`plans/modules/clawpatch-pre-tag-v0.7.0-beta.aps.md`)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Advance CLAWP-008 status `In Progress` → `Merged` in
      `plans/modules/clawpatch-pre-tag-v0.7.0-beta.aps.md` (agent: yes)
- [ ] Bump CLAWP module progress count `1/64 → 2/64` in the module
      header table and in the `Continuous Improvement` row of
      `plans/index.aps.md` (agent: yes)
- [ ] Close GH issue #1650 if it does not auto-close from the
      `Closes #1650` trailer (agent: yes)
- [ ] Re-confirm that the release-council pass-2 obligation list in
      `plans/reviews/release-council/2026-05-20-v0.7.0-beta-pre-tag.md`
      (§"Tag-blocking obligations remaining" item 2) shows CLAWP-008
      patch landed; cross-link this PR (agent: yes)

## Notes

- Streaming Council (`quick` pack) session `council-315fe3a2` ran
  pre-push with the single `council-reviewer` reviewer; 0 findings,
  closed `converged`. Reviewer confirmed the `??` double-unwrap is
  semantically correct (`scan_artifact_rust` returns bare `ScanResult`)
  and the pattern matches the existing `get_default_patterns_json` /
  `get_pattern_json` entry points in the same file.
- Validation run on `2a214184` rebased onto `origin/main` `0ad26068`:
  - `cargo fmt --all --check` — clean
  - `cargo clippy --workspace --all-targets -- -D warnings` — clean
  - `cargo test --workspace` — green, no FAILED suites
  - `pnpm format:check && pnpm lint:check && pnpm typecheck && pnpm test`
    — green
  - `pnpm --filter @eddacraft/anvil-checks-native test` — 15/15 pass
    (covers C1 throw contract + #1630 embedded-catalogue fallback +
    diagnostics-on-known-bad-input contract, which together exercise
    the success path through the rearranged `catch_unwind` closure).
- No regression test for a registry-load *panic* is added: the
  `load_compiled_registry` chokepoint exposes no panic-injection seam,
  and adding one for this single boundary would expand scope past the
  release-council ~+5/-5 verdict shape. The structural symmetry with
  `get_default_patterns_json` (already audited under council review
  X3/2026-04-24) is the load-bearing argument that the boundary now
  holds for `scan_artifact_json` too.
- This PR is the second of the three docs-unrelated residual
  obligations listed in
  `plans/reviews/release-council/2026-05-20-v0.7.0-beta-pre-tag.md`
  §"Tag-blocking obligations remaining" (item 2). CLAWP-028 (item 3)
  remains open under PR #1741.
