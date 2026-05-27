# Planning Council — CIB-025 (Direction Validate)

**Date:** 2026-05-27
**Mode:** Direction Validate (before Draft → Ready)
**Item:** CIB-025 — "Generate the index module rows so PRs don't touch the shared count"
**Decision:** **AMEND** (unanimous across all four lenses) — direction valid, the
proposed mechanism is not Ready. Item advanced **Draft → Proposed**, not Ready.

## Repo reality checked

- Base: `origin/main` (planning worktree off current main).
- `plans/index.aps.md` — module-status table, ~151 rows, 107 with a derived
  `N/M` count cell; Progress cells carry derived count **and** curated prose.
- Prior art: CIB-022 — `scripts/aps/index-counts.mjs` + `scripts/aps/lib/modules.mjs`
  derive the counts + CI-enforce via `aps:index:check` (`.github/workflows/ci.yml`
  Docs Lint job), but the row stays hand-editable.
- Precedent cited: `scripts/docs/docs-index.mjs` (generates `docs/indexes/`).
- Authority rule: `.claude/rules/aps-index.md` declares `index.aps.md` the
  "single source of truth".

## Lens verdicts (all AMEND)

| Lens | Verdict | Strongest point |
| --- | --- | --- |
| pragmatic | amend | Observed contention was *same-module*; proposal targets cross-module — a problem we didn't have. Cheaper shape: drop index prose, count-only cells. Full generation = 2–3 waves. |
| operations | amend | `aps:index:check` must check the *whole* table; malformed module must fail loud (not silent row drop); 107-row cutover is a merge bomb → must be staged; section membership + per-section columns undefined; `drift-check.mjs` + `active-lint.mjs` are co-consumers needing co-changes. |
| adversarial | amend | Central flaw: generation **moves** same-module contention from the index row into the module file; it is not removed. Per-row narrative is not round-trippable from module data. Migration PR is an acute form of the disease. |
| integrity/security | amend | No cell escaping (`|`/newline) — `docs-index.mjs` has `escapeTable`, the APS parser does not; parser exposes counts but not module-level Status; 13+ heterogeneous table schemas; failure-closed behaviour unspecified. |

## The central flaw (convergent)

The four serialised rebases on 2026-05-26 were **same-module** (four CIB items →
the one CIB row). Generating the table from module files relocates that collision
into the module file (its `Status:` lines + header count) — it does not eliminate
it. CIB-025's original validation ("two PRs in *different* modules merge clean")
tests a case that already passes today, so it would have given false confidence.

## Risks / unresolved questions → captured as CIB-025 Design Gates

1. Same-module mechanism (or scope to cross-module only).
2. Prose custody — where ~40 KB of curated index narrative lands, or drop it.
3. Schema heterogeneity — section membership + per-section columns.
4. Integrity — cell escaping, module-Status parsing, fail-closed, archive-skip.

## Required deterministic checks (before Ready)

- `aps:index:check` extended from count-token to full-table/full-cell comparison.
- Fixture: `|` in a module title must not corrupt the table.
- Fixture: unparseable module → generator exits non-zero, names the file.
- Determinism: stable sort (already `listModulePaths().sort()`), byte-stable CI.
- `scripts/aps/drift-check.mjs` index cross-check verified or repointed to module
  sources; `scripts/aps/active-lint.mjs` index handling verified.

## Plan updates made

- CIB-025 → **Proposed**; intent reframed to same-module; candidate shapes
  reordered (count-only cheapest first); four Design Gates added; validation
  corrected to same-module; co-changes + waved migration recorded.

## Recommendation to operator

Two paths to Ready:
- **(a)** Split off shape 1 (drop index prose → count-only cells) as a small,
  separately-Ready win that directly shrinks the observed conflict surface, and
  keep full generation (shapes 2/3) Proposed pending the gates; or
- **(b)** Run a `plan-create` design pass that resolves Gates 1–4, then promote
  the whole item to Ready as a waved plan.

Do **not** start implementation while the decision is AMEND.
