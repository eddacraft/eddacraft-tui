# ADR-058: Shared SARIF emitter + per-command adapters, no unified finding model

## Status

Accepted (2026-05-29). One of the two SARIFOUT candidate ADRs; ratified by the
operator on 2026-05-29 alongside [ADR-056](056-format-flag-output-selector.md)
(the `--format` selector).

## Date

2026-05-29

## Context

The `SARIFOUT` module adds SARIF 2.1.0 output to the three finding-emitting
commands as a pure additive machine format. Those commands produce **three
genuinely heterogeneous finding shapes** (confirmed against the code on
2026-05-29):

- `anvil check` — per-location `Warning`s carrying suppression state
  (baseline / `@anvil-ignore`); the serialized `JsonWarning` projection drops
  the suppression flag.
- `anvil gate` — per-check pass/fail/score **aggregates** (`GateResult.checks[]`),
  not per-location findings.
- `anvil audit` — per-issue `file`/`line` records (`AuditOutput.issues[]`).

The readiness review flagged "no shared finding model" as an open question:
should the engine crates (`anvil-checks` / `anvil-policy-engine` /
`anvil-rules`) be refactored onto a unified in-process finding model before
SARIF, or should each command map independently? See the design doc's
Decision 3.

## Decision

Introduce a **thin shared SARIF serialisation layer** at
`crates/anvil-cli/src/output/sarif.rs` that owns the SARIF 2.1.0 document shape
for the pinned subset (`runs[]` / `tool.driver` / `rules[]` / `results[]` /
`locations[]` / `suppressions[]` / `partialFingerprints`), and feed it from
**three small per-command adapter functions** (SARIFOUT-003/004/005) that map
each command's existing result shape into these types.

Explicitly **do not** refactor the engine crates onto a unified finding model
as part of this work:

1. **SARIF itself is the shared target model.** Mapping each command's shape
   *into SARIF* is the right level of abstraction. A second intermediate
   in-process model would be over-abstraction (DRY's "don't over-abstract").
2. **The shapes carry genuinely different information** (per-location +
   suppression; per-check aggregate; per-issue). A premature shared model risks
   lossy flattening.
3. **A cross-crate finding-model refactor is large and hard to reverse** — it
   would dwarf the additive output goal and touch the engine crates.
4. **Per-command adapters keep each command's SARIF slice an independent,
   single-purpose PR.**

The emitter is a pure serialisation layer with no command wired (it ships in
SARIFOUT-002 ahead of its consumers). The bundled upstream SARIF 2.1.0 JSON
Schema (`crates/anvil-cli/src/output/sarif-schema-2.1.0.json`, vendored verbatim
from schemastore — no fork) is the validation gate, enforced by an in-repo
schema-validation test.

## Rationale

SARIF is a transport for findings the commands already produce. The bounded
shared piece is the *document emitter*; the per-command mapping stays
distributed because the inputs are irreducibly different. This keeps the change
proportional to "make existing findings consumable" and avoids an engine
refactor that nothing else currently needs.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Shared emitter + per-command adapters (chosen)** | Bounded shared surface; each adapter is an independent PR; no lossy intermediate model; SARIF is the shared target | Three small mapping functions instead of one; rule/level vocab mapped per command |
| **A. Unified in-process finding model first** | One mapping into SARIF | Large, hard-to-reverse engine-crate refactor; risks lossy flattening of three distinct shapes; out of proportion to additive output |
| **B. Per-command bespoke SARIF, no shared emitter** | Smallest first step | Duplicates the SARIF document shape three times; schema-validation + fingerprint logic copy-pasted; drift between commands |

## Consequences

- **Positive:** A reusable SARIF emitter shared across the finding-emitting
  commands (and any future SARIF output);
  per-command adapters land independently; no engine churn; deterministic
  `partialFingerprints` + a schema-validation gate by construction.
- **Negative:** `level` / `ruleId` vocabulary is mapped per command (three small
  mappers). The emitter's public surface is unused until the first adapter
  (SARIFOUT-003) wires it.
- **Future:** if a genuine need for an in-process shared finding model emerges
  (e.g. a unified `anvil findings` surface), that is its own decision with its
  own ADR — note it, do not pre-build it.

## References

- Design: [`../specs/2026-05-29-sarif-output-design.md`](../specs/2026-05-29-sarif-output-design.md) (Decision 3 — Shared Finding Model)
- APS: `SARIFOUT-002` (emitter + schema harness), `SARIFOUT-003/004/005` (adapters), `SARIFOUT` module
- Related ADRs: [ADR-056](056-format-flag-output-selector.md) (the `--format` selector)
- Code: `crates/anvil-cli/src/output/sarif.rs`, `crates/anvil-cli/src/output/sarif-schema-2.1.0.json`
