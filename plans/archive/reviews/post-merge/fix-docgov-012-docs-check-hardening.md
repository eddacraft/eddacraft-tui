# Post-merge test plan — DOCGOV-012 docs-check hardening

Issue: [#2075](https://github.com/eddacraft/anvil-001/issues/2075) ·
Branch: `fix/docgov-012-docs-check-hardening`

Three robustness defects in the DOCGOV-owned `scripts/docs` gating tooling were
fixed. Verify post-merge on `main`:

## 1. Baseline data-loss guard (`docs-check.mjs --update-baseline`)

- A `--update-baseline` run where a baselineable surface fails to emit valid
  JSON must exit non-zero and leave `docs/governance/docs-check.baseline.json`
  byte-for-byte unchanged.
- A fully-successful run still writes the baseline and carries forward any
  non-regenerated (e.g. non-baselineable) keys.
- Repro covered by `docs-check.test.sh` case 11 via the `--root` / `--surfaces`
  test seam (stub surface scripts; never touches the live corpus or tracked
  baseline). To exercise the happy path against the real corpus, run
  `pnpm docs:check:update-baseline` and confirm `git diff` shows only intended
  drift before committing.

## 2. `--no-baseline` flag routing (`docs-check.mjs`)

- `pnpm docs:check -- --no-baseline` (orchestrator) must reach the
  `index-freshness` surface without an `ERR_PARSE_ARGS_UNKNOWN_OPTION` crash —
  baseline flags are only forwarded to surfaces where `surface.baselineable` is
  true.
- Covered by `docs-check.test.sh` case 10.

## 3. Malformed-link crash guard (`check-links.mjs resolveLink`)

- A link with a malformed percent escape (`./foo%zz.md`, `#sec%`) must yield a
  labelled `[links] ERROR: <file>:<line> — malformed link ...` finding and a
  non-zero exit, never an uncaught `URIError`.
- Covered by `docs-check.test.sh` case 12 (fixture root, no live-corpus impact).

## Gate re-run (CI-equivalent)

```
pnpm format:check
pnpm lint:check
pnpm typecheck
pnpm test:docs-check
node scripts/aps/index-counts.mjs --check
```

All must exit 0. The live `docs/governance/docs-check.baseline.json` must remain
unmodified by this change — the test seam isolates regeneration to temp roots.

## APS bookkeeping

- DOCGOV-012 flipped to `In Progress` in
  `plans/modules/documentation-governance.aps.md` (operator-authorised this
  session). DOCGOV count cell stays `9/12` (In Progress is non-terminal).
- On merge, the executing session should advance DOCGOV-012 to
  `Merged YYYY-MM-DD via PR #N`.
