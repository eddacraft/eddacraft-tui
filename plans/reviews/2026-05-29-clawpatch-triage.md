# Clawpatch triage — 2026-05-29 (periodic scan)

**Scan command:** `clawpatch map && clawpatch review --limit 81 --jobs 8`
**Status command:** `clawpatch status`
**Findings input:** `plans/audits/2026-05-29-clawpatch-periodic-scan.json`
**Run:** `20260528T190747-1a75b2` (codex provider, codex-cli 0.133.0)

## Why this run

The last clawpatch audit was `2026-05-25` against the rolling `v0.7.0-beta`
corpus. Tags `v0.7.1-beta` and `v0.7.2-beta` both shipped after it without a
fresh sweep, so newly added surface went unreviewed for two releases. This is a
catch-up periodic scan on current `main`, not a pre-tag gate.

## Scan Summary

- Features mapped: 340 (map delta vs prior corpus: **10 new, 4 changed, 1 stale**)
- Features reviewed this wave: 10 (the delta set)
- New findings this run: **7** — 0 high, 4 medium, 3 low
- Corpus totals: 338 findings — 20 high, 196 medium, 122 low (325→332 open)
- Prior corpus (2026-05-25): 331 findings — 20 high, 192 medium, 119 low

**No new high-severity findings.** The 20-high umbrella
([#1826](https://github.com/eddacraft/anvil-001/issues/1826)) is unchanged and
must not be re-filed.

## Verdict

The two-release gap introduced **no new high-severity risk**. The 7 new findings
are tooling-robustness and test-hardening items, concentrated in three places:
the `scripts/docs` gating tools, `scripts/aps/drift-check.mjs`, and two Rust
integration tests. Two clusters are genuinely actionable and well-scoped; the
rest are advisory-quality improvements that should batch rather than each spawn
an issue.

## Filed / Routed

### Cluster A — docs-check tooling robustness (3 findings, `scripts/docs`)

All three are confirmed/contract bugs in DOCGOV-owned gating tooling. Same file
family, so routed as **one** APS item: **DOCGOV-012**
(`plans/modules/documentation-governance.aps.md`), tracked by GH issue
[#2075](https://github.com/eddacraft/anvil-001/issues/2075).

- `fnd_sig-feat-library-34bc4660c0-5f3f` — **[medium] data-loss, confirmed-bug.**
  `--update-baseline` can overwrite `docs-check.baseline.json` after a partial
  surface failure, dropping entries for surfaces that failed to produce valid
  JSON. Fix: collect failures, preserve/reload existing entries for failed
  surfaces, skip the final write unless regeneration fully succeeded; exit
  non-zero on any baselineable-surface failure. (Corroborates the hand-noted
  friction in `continuous-improvement-log.md` — never filed.)
- `fnd_sig-feat-library-34bc4660c0-d3a8` — **[medium] api-contract, contract-mismatch.**
  `docs-check.mjs` forwards `--no-baseline` to `check-index-freshness` /
  `docs-index.mjs`, which reject the unknown flag. Fix: only append baseline
  flags to `surface.baselineable` surfaces.
- `fnd_sig-feat-library-34bc4660c0-2aa7` — **[low] bug, confirmed-bug.**
  Malformed percent escapes in a link crash `check-links` with an uncaught
  `URIError` instead of producing a labelled ERROR finding. Fix: wrap
  percent-decoding in `resolveLink`, return a validation error on failure.

### Cluster B — drift-check advisory robustness (1 finding, `scripts/aps`)

- `fnd_sig-feat-library-ae662c437a-fe21` — **[low] bug, risk.** Invalid or
  unreadable release records crash `scripts/aps/drift-check.mjs` instead of
  emitting an advisory finding. This violates the **warnings-over-blocks / exit
  0** architecture principle. Genuinely new (distinct from CIB-023, which is the
  implemented-but-draft drift class). Routed as **CIB-035**
  (`plans/modules/continuous-improvement-backlog.aps.md`). Fix: catch read/parse
  errors, emit `release-record-unreadable` / `release-record-invalid-json` as a
  JSON advisory finding, preserve exit 0.

## Deferred / Batch

- `fnd_sig-feat-test-suite-dc9672d02c-d` — **[medium] test-gap.** The
  `json_stdout_clean_when_warn_fires_at_default_filter` policy_eval test can pass
  with polluted stdout. Worth hardening (parse `stdout.trim()` as exactly one
  `serde_json::Value`, assert the failure shape) but it is test-quality, not a
  product defect. Batch under test-hardening; promote to its own item only if a
  policy-eval slice opens.
- `fnd_sig-feat-test-suite-c547e79b69-3` — **[low] test-gap.**
  `pre_push_subprocess` smoke tests under-assert the documented stdout/stderr
  contract. Low-value hardening; defer to batch.
- `fnd_sig-feat-library-54e695c50d-2c22` — **[medium] build-release, risk
  (medium confidence).** Observability package `build` script points at a
  non-owned tsconfig. **Verify relevance before investing** — the JS/TS
  workspace is on a deliberate retirement track; do not harden build config for
  a package that may be bitrotted out. Defer pending that call.

## Covered Existing

- The 20 high-severity findings remain covered by umbrella
  [#1826](https://github.com/eddacraft/anvil-001/issues/1826). This run added no
  highs; no duplicate high-severity issues opened.

## Evidence

- `clawpatch status`: 340 features, 338 findings, 332 open, 0 active locks,
  last run `20260528T190747-1a75b2`.
- Map delta: 10 new + 4 changed + 1 stale feature vs the prior corpus — the
  surface added across `v0.7.1-beta` / `v0.7.2-beta`.
- New-finding severity delta vs 2026-05-25: high +0, medium +4, low +3.
- `clawpatch fix` was **not** run (working tree dirty + on `main`; non-trivial
  fixes need APS authorisation per the clawpatch daily-workflow brainstorm).
