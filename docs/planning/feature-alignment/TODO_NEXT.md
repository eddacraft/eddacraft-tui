# TODO_NEXT — Anvil (Execution Tasks)

Aligned to PLAN_NEXT.md.

---

## EPIC A — Refocus the product surface (Phase 0)

- [ ] A1. Rewrite README/landing narrative (developer-first trust broker)
  - Acceptance: headline promise, planless-first, local-first quickstart, “what
    it is / is not”.
- [ ] A2. Define v1 warning schema (payload contract)
  - Includes: id/fingerprint, category, severity, confidence, pattern/rule,
    location, drift, impact, explanation, suggestions, suppression.
- [ ] A3. Decide v1 surfaces and artifacts
  - CLI output + JSON artifact; IDE diagnostics; PR/CI mirroring.

## EPIC B — On-save runner (Phase 1)

- [ ] B1. File-save trigger (CLI watcher + editor integration hook)
- [ ] B2. Performance baseline + caching (dependency graph reuse)
- [ ] B3. Deterministic JSON artifact per run (stable ordering)

## EPIC C — Architecture baseline + new-edge detection (Phase 1)

- [ ] C1. Build directed dependency graph (v1 language/framework scope)
- [ ] C2. Identify runtime entry points (routes/handlers/jobs/etc.)
      deterministically
- [ ] C3. Detect new cross-boundary edges introduced by the change
- [ ] C4. Legacy drift handling: warn on NEW; acknowledge existing
- [ ] C5. `anvil init` v1-lite: exploratory models + descriptive fallback; store
      baseline

## EPIC D — AI anti-pattern library (Phase 1)

- [ ] D1. Define v1 anti-pattern catalogue (built-in) with explanation +
      suggestions
- [ ] D2. Implement deterministic detectors (high-confidence warnings)
- [ ] D3. Repo/org customisation mechanism (enable/disable/override
      thresholds/messages)

## EPIC E — Actionable messaging (Phase 1)

- [ ] E1. Standard warning formatting + grouping/deduping
- [ ] E2. Confidence rules: careful phrasing; show explicit confidence only when
      low

## EPIC F — Suppression + provenance (Phase 1)

- [ ] F1. Inline suppression annotations (rule-scoped) requiring note
- [ ] F2. Structured provenance record (author/time/rule/scope/note)
- [ ] F3. Suppression discoverability (report command / drift report
      integration)

## EPIC G — PR/CI fail-safe mirroring (Phase 1–2)

- [ ] G1. PR comment or check output: mirror warnings; stable ordering; dedupe
- [ ] G2. CI output + artifact publishing; configurable exit behaviour

## EPIC H — Drift reporting (Phase 2)

- [ ] H1. Snapshot report (counts, top crossings, new vs existing)
- [ ] H2. Compare report (baseline vs now; new edges; new suppressions)

## EPIC I — AI visibility artifacts (Phase 3)

- [ ] I1. Export constraints file for AI tools (patterns, boundaries,
      anti-patterns)
- [ ] I2. Export structured feedback feed (warnings schema as stable contract)
