# Clawpatch triage council — 2026-05-25

**Scan command:** `clawpatch map && clawpatch review --limit 81 --jobs 8`
**Status command:** `clawpatch status`
**Findings input:** `plans/audits/2026-05-25-clawpatch-v0.7.0-beta.json`
**Council mode:** full OpenCode council pack

## Scan Summary

- Features mapped: 330
- Incrementally reviewed in this run: 2
- Findings in report: 331 total — 20 high, 192 medium, 119 low
- Finding states: 325 open, 5 fixed, 1 uncertain
- High-severity umbrella: GH [#1826](https://github.com/eddacraft/anvil-001/issues/1826)

## Council Verdict

The 2026-05-25 scan is primarily a delta check over the prior
`v0.7.0-beta` clawpatch corpus. The high-severity corpus remains covered by
the existing umbrella issue #1826; it should not be mass-filed again. The two
new findings from `feat_route_9b3fea7b74` are actionable as one broadcast API
contract follow-up plus bundled docs closeout.

## Filed / Routed

- `fnd_sig-feat-route-9b3fea7b74-465993_c717310cda` — **file-gh-issue + APS item**.
  Filed GH [#1926](https://github.com/eddacraft/anvil-001/issues/1926) and
  added `EMAIL-010` to `plans/modules/email-broadcast.aps.md`. Real-send must
  accept preview-token-only requests and use the consumed snapshot as source of
  truth for `template`, `audience`, `audienceParams`, and `templateProps`.
- `fnd_sig-feat-route-9b3fea7b74-916bab_c69264c5ba` — **bundle with #1926 / EMAIL-010**.
  The README endpoint-table omission is real but docs-only; it should land with
  the broadcast contract fix rather than as a separate issue.

## Covered Existing

- The 20 high-severity findings are covered by GH
  [#1826](https://github.com/eddacraft/anvil-001/issues/1826). Searches for the
  high finding titles returned the umbrella issue, so no duplicate high-severity
  issues were opened in this pass.
- `fnd_sig-feat-test-suite-73ba6156c4-e_b30c969c73` remains covered by
  `CLAWP-001`, which is already marked Merged in
  `plans/modules/clawpatch-pre-tag-v0.7.0-beta.aps.md`.

## Deferred / Batch

- Medium and low pre-existing test-hygiene/docs findings remain deferred under
  the existing CLAWP and release-council dispositions unless they become part of
  an active implementation slice.
- If #1826 is broken down later, Council recommends deduping by defect cluster:
  policy bundle trust boundary, shared-storage symlink escapes, runtime lock
  concurrency, package publish integrity, Kindling redaction, and website
  activation contract drift.

## Evidence

- `clawpatch status`: 330 features, 331 findings, 325 open findings, 0 active
  locks, last run `20260524T203341-faca6e`.
- Existing-issue search found #1826 for representative high-severity titles.
- New issue filed: #1926.
