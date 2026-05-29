# SARIF Output Design — `anvil check` / `anvil gate` / `anvil audit`

| Type   | Authority | Owner | Status   | Freshness          |
| ------ | --------- | ----- | -------- | ------------------ |
| Design | Proposal  | —     | Accepted | Authored 2026-05-29; three decisions ratified 2026-05-29 |

| Upstream                                                                          | Downstream                          |
| --------------------------------------------------------------------------------- | ----------------------------------- |
| [CIB-014](../modules/continuous-improvement-backlog.aps.md), [Drako borrow assessment](../brainstorms/2026-05-24-drako-borrow-assessment.md) §4 Borrow A | `sarif-output.aps.md` (SARIFOUT module), candidate ADRs |

## Problem

Anvil produces deterministic findings from three finding-emitting commands —
`anvil check`, `anvil gate`, and `anvil audit` — but the only machine-readable
output is each command's bespoke JSON shape. Consumers that already speak SARIF
(GitHub Code Scanning, Sonar, DefectDojo, security dashboards) cannot ingest
Anvil findings without writing an adapter per command. CIB-014 proposes
`--format sarif` to close that gap as a pure additive output mode.

A readiness review deferred CIB-014 from direct implementation because three
design decisions were unresolved and the work is oversized for one PR:

1. **Flag surface** — `--format sarif` does not fit the existing three-variant
   global output enum (`OutputMode::{Tui, Plain, Json}`).
2. **Module home** — new module vs. an item under an existing module.
3. **Shared finding model** — the three commands emit three distinct JSON
   shapes from heterogeneous finding types, with no shared model.

This document resolves all three with scope-guard-compliant recommendations and
splits the work into a waved set of single-purpose PRs. It is a **proposal**;
the three headline decisions are flagged for human sign-off.

## Scope-Guard Alignment

SARIF output is a deterministic, machine-readable rendering of findings Anvil
**already produces**. Applying the `docs/vision/anvil-scope-guard.md` decision
framework:

- **Increases prevention capability?** Yes, indirectly — it removes adoption
  friction so Anvil findings reach the dashboards where teams gate merges, which
  is where the new-edges-only enforcement actually bites in CI.
- **Operates before/at execution time?** It renders findings produced
  pre-merge; it adds no runtime/post-hoc surface.
- **Strengthens deterministic control?** Yes — SARIF is emitted by construction
  from already-deterministic findings; same input, same output (ADR principle:
  Deterministic).
- **Enforces or just informs?** It is a transport for findings that already
  enforce; it is not a new advisory surface. It introduces no new dashboard,
  scoring scalar, or telemetry sink (those would trip scope-guard #5 and the
  "determinism scoring" reject in the Drako assessment §5).

It also honours the ADR principles: **warnings over blocks** (SARIF emission is
exit-code-neutral; the existing gate/threshold exit codes are unchanged) and
**new edges only** (baseline-suppressed findings render under SARIF
`suppressions[]`, preserving the posture-vs-regression distinction).

Drako is cited as **parallel evolution**, not a dependency: no Python import, no
rule import, no schema fork. Anvil consumes the upstream SARIF 2.1.0 schema
directly.

## Grounding Note — current code shapes (confirmed 2026-05-29)

The CIB-014 line citations drifted; corrected anchors on the
`docs/cib-014-sarif-planning` branch:

- **Output enum:** `crates/anvil-cli/src/output/mod.rs` — `OutputMode` enum
  (`Tui`/`Plain`/`Json`) with `resolve(json, no_tui, is_tty)` and
  `from_global(&GlobalArgs)`. `GlobalArgs` (`crates/anvil-cli/src/main.rs:78`)
  carries `--json`, `--no-tui`, `--verbose` as global flags; there is **no**
  `--format` flag today.
- **`anvil check`:** no `CheckResult` struct. JSON is built by
  `build_json_output(...)` (`check.rs:1030`) over `JsonWarning`
  (`check.rs:151`). Important: **`JsonWarning` itself has no `suppressed`
  field** — it is `id/category/severity/title/message/file/line/suggestion/
  nudge` only, and `antipattern_warning_to_json` (`check.rs:443`) drops
  suppression when projecting. The suppression signal lives on the **upstream
  `Warning` type** as `Warning.suppressed: Option<…>` (read at `check.rs:556`,
  `:675`, `:1118`+) — this is the baseline / `@anvil-ignore` signal. The SARIF
  adapter must therefore read suppression from the upstream `Warning` path
  **before** the JSON projection, not from `JsonWarning`.
- **`anvil gate`:** `GateResult` (`gate.rs:134`) holds `Vec<CheckResult>`
  (per-check pass/score/message, `gate.rs:142`); the AI-profile JSON envelope is
  `AiGateResultEnvelope` (`gate.rs:1896`) built by
  `build_ai_gate_result_envelope` (`gate.rs:1935`). Gate findings are
  per-check aggregates, not per-location warnings.
- **`anvil audit`:** `AuditOutput` (`audit.rs:573`) holds `Vec<IssueOutput>`
  (`audit.rs:583`) with `severity`/`category`/`message`/`file`/`line`/`fixable`.

The three shapes are genuinely heterogeneous: `check` is per-location warnings
with suppression state; `gate` is per-check pass/fail aggregates; `audit` is
per-issue file/line records. This confirms the readiness review's "no shared
finding model" finding and drives the Decision 3 recommendation below.

## Decision 1 — Flag Surface (recommended: `--format` extends, not replaces, the enum)

**Recommendation:** add a new global `--format <FORMAT>` flag whose value space
is `auto | tui | plain | json | sarif`, and **fold the existing `--json` /
`--no-tui` booleans into it as compatibility aliases** rather than introducing a
parallel, fourth-state output path.

Why not just add a `Sarif` variant straight onto `OutputMode`? Because the
current `resolve(json, no_tui, is_tty)` signature is a 2-boolean + TTY truth
table. SARIF is neither TTY-driven nor a degrade target — it is an explicit
opt-in machine format that must never be auto-selected by TTY detection. Bolting
a third boolean on produces an ambiguous precedence matrix (`--json --format
sarif`?). A single `--format` enum with explicit precedence is cleaner and
extensible.

**Composition contract (backward-compatible):**

- `OutputMode` gains a `Sarif` variant.
- A new `resolve_format(format: Option<Format>, json: bool, no_tui: bool,
  is_tty: bool) -> OutputMode` becomes the single resolver. Precedence:
  explicit `--format` wins; then legacy `--json` → `Json`; then `--no-tui` /
  non-TTY → `Plain`; else `Tui`.
- `--json` continues to mean exactly `--format json` (kept as a documented
  alias; **no deprecation, no behaviour change** — the existing `--json` tests
  in `output/mod.rs` stay green and gain a `--format json` parity test).
- `--format sarif` is **only** valid on the finding-emitting commands
  (`check`, `gate`, `audit`); on other commands it is a clap-level error, not a
  silent fallback (honours the no-silent-defaults memory: propagate `Err`, never
  degrade-to-default).
- SARIF is **never** auto-selected: `--format auto` (the default) resolves
  through the existing TUI/Plain/JSON truth table; `sarif` must be named
  explicitly.

This is a new global-flag convention (a value-enum `--format` replacing two
booleans as the canonical selector) → **candidate ADR** (see ADR candidates).

## Decision 2 — Module Home (recommended: new dedicated module `SARIFOUT`)

**Recommendation:** file a **new dedicated APS module** `sarif-output.aps.md`
(scope ID `SARIFOUT`) under the **Engineering Platform** section of
`plans/index.aps.md`.

Justification against `aps-rules.md` + scope-guard:

- The work **spans three commands and a shared output layer** — it is exactly
  the "cross-cutting concern that spans all packages and releases" that the
  Engineering Platform section exists for (siblings: api-governance,
  command-safety-surfaces, notification-framework all live there as multi-command
  output/contract surfaces).
- It is "a product feature large enough to need its own APS module" — the CIB
  Out-of-Scope rule explicitly routes such work out of the backlog, and the CIB
  intake rule says to "promote into a dedicated APS module" when a cluster is
  large or domain-specific. SARIF mapping for three distinct shapes plus a
  suppressions path is multi-PR work.
- It is **not** Observability Export (EXPORT): that module is a *telemetry sink*
  for the tracing pipe (spans, sampling, retention) per ADR-035. SARIF is a
  *findings transport*, not telemetry — wiring it into EXPORT would violate the
  three-pipe rule. Confirmed by reading `observability-export.aps.md`.
- It is **not** Compliance Reporting (COMPLY): the CIB already records that SARIF
  is *upstream* of framework-mapped compliance evidence, not a substitute. A
  `Coordinates with: COMPLY` callout captures the seam without colonising it.

The brainstorm's alternative ("an item under Engineering Platform") is rejected
because a single work item cannot carry three independent command-mapping slices
plus a flag-surface slice plus a schema-validation harness at Ready quality. A
dedicated module with per-slice work items is the correct granularity.

## Decision 3 — Shared Finding Model (recommended: thin shared SARIF *emitter*, per-command *adapters*; NO upstream finding-model refactor)

**Recommendation:** introduce a **thin shared SARIF serialisation layer**
(`crates/anvil-cli/src/output/sarif.rs`) that owns the SARIF 2.1.0 document
shape (`runs[]`, `tool.driver`, `rules[]`, `results[]`, `locations[]`,
`suppressions[]`) and is fed by **three small per-command adapter functions**
that map each command's existing result shape into the shared SARIF types.

Explicitly **do not** refactor `anvil-checks` / `anvil-policy-engine` /
`anvil-rules` onto a unified finding model as part of this work. Reasons:

- A cross-crate finding-model refactor is a large, hard-to-reverse architectural
  change that touches the engine crates and would dwarf the additive output
  goal. It is out of proportion to "make existing findings consumable".
- The three shapes carry genuinely different information (per-location warning +
  suppression state; per-check aggregate; per-issue file/line). A premature
  shared model risks lossy flattening — and SARIF *is itself* the shared target
  model. Mapping each shape **into SARIF** is the right level of abstraction; a
  second intermediate model is over-abstraction (DRY caveat: "don't
  over-abstract").
- Keeping adapters per-command means each command's SARIF slice is an
  independent, single-purpose PR.

If a future need for an in-process shared finding model emerges (e.g. a unified
`anvil findings` surface), that is its own decision with its own ADR — note it,
do not pre-build it. The shared SARIF emitter is the bounded shared piece; the
per-command mapping stays distributed. This shared-emitter convention is a
**candidate ADR** (lightweight) because it sets the pattern for any future
machine-output format.

### Mapping sketch (bounded, not prescriptive)

| Command | Source shape | SARIF `results[]` unit | `ruleId` source | `locations[]` | `suppressions[]` |
| ------- | ------------ | ---------------------- | --------------- | ------------- | ---------------- |
| `check` | upstream `Warning` (per warning; read before the `JsonWarning` projection) | one result per warning | warning/pattern id | file + line/region from warning | `Warning.suppressed` → one `suppression` (kind `external`/`inSource`) — note `JsonWarning` drops this field, so the adapter reads the upstream `Warning` |
| `gate`  | `GateResult.checks[]` | one result per failed/needs-config check | check `name` | repo-level or config-file location | config-gap (`requires_config`) → suppression or `notification`, decided at impl |
| `audit` | `AuditOutput.issues[]` | one result per issue | `category` | `file` + `line` | none in v1 (audit has no suppression field today) |

`tool.driver.rules[]` is populated from the distinct rule/pattern/category ids
encountered, deduplicated. `level` maps from each command's severity vocabulary
to SARIF `error|warning|note`.

## Supported SARIF 2.1.0 Subset (bounded)

Pinned to the GitHub Code Scanning ingest subset. **In:**

- `version`, `$schema`, single `runs[]` entry.
- `runs[].tool.driver` — `name` (`anvil`), `informationUri`, `version`,
  `rules[]` (id + shortDescription + helpUri where available).
- `runs[].results[]` — `ruleId`, `level`, `message.text`, `locations[]`.
- `locations[].physicalLocation` — `artifactLocation.uri` (repo-relative) +
  `region` (`startLine`, optional `startColumn`).
- `runs[].results[].suppressions[]` — SARIF §3.35, `kind` + optional
  `justification`, for baseline / `@anvil-ignore` suppressed findings.
- `runs[].results[].partialFingerprints` — stable fingerprint so Code Scanning
  dedupes across runs (deterministic from rule id + path + region + message).

**Out (explicitly not in scope):** full SARIF 2.1.0 conformance, `codeFlows`,
`taxonomies`, `relationships`, `graphs`, `webRequest`/`webResponse`,
`fixes[]`, multi-run documents, baseline-state `baselineState` diffing.

## Validation Strategy

- **In-repo, deterministic (CI):** fixture tests that emit SARIF from each of
  the three commands and validate the output against the bundled SARIF 2.1.0
  JSON Schema. Golden-file snapshot tests pin the exact document shape per
  command (including a suppressed-finding fixture for `check`). This half is
  fully testable in-repo and is the CI gate.
- **Manual / out-of-band (NOT a CI test):** upload an emitted SARIF file to a
  GitHub Code Scanning sandbox repo and confirm findings render. This is a
  non-deterministic external dependency (network + GitHub ingest behaviour) and
  is scoped as a documented manual smoke check in the module, **not** a CI test.

## PR Wave Breakdown (single-purpose PRs — never one mega-PR)

| Wave | PR (single purpose) | Work item | Depends on | Validation |
| ---- | ------------------- | --------- | ---------- | ---------- |
| 1 | `--format` flag surface + `OutputMode::Sarif` + resolver, `--json` alias parity, clap-reject `sarif` on non-finding commands | SARIFOUT-001 | — | unit tests on `resolve_format`; `--json`/`--format json` parity; reject test |
| 1 | Shared SARIF emitter (`output/sarif.rs`): document shape + bundled 2.1.0 schema + schema-validation harness, no command wired yet | SARIFOUT-002 | — | schema-validation test on a hand-built fixture document |
| 2 | `anvil check` → SARIF adapter incl. `suppressions[]` for suppressed warnings | SARIFOUT-003 | 001, 002 | golden + schema fixture incl. a suppressed finding |
| 2 | `anvil audit` → SARIF adapter | SARIFOUT-004 | 001, 002 | golden + schema fixture |
| 3 | `anvil gate` → SARIF adapter (per-check results; config-gap handling) | SARIFOUT-005 | 001, 002 | golden + schema fixture |
| 4 | Docs + manual GH Code Scanning upload smoke check runbook + CHANGELOG | SARIFOUT-006 | 003, 004, 005 | manual upload check recorded; `pnpm docs:check`/`format:check` |

Waves 1's two PRs are independent (flag surface vs. emitter scaffold) and can run
in parallel. Waves 2/3 command adapters are independent of each other once the
Wave 1 floor lands. Wave 4 is the closeout.

## Coordination

- **COMPLY (compliance-reporting):** SARIF is upstream of framework-mapped
  compliance evidence, not a substitute. `Coordinates with`, not a dependency.
- **CIB-008 / CIB-009 (both Merged):** `check` / `audit` dispatcher consistency
  already landed, so SARIF reflects the corrected finding set, not the old bug.

## Candidate ADRs (recommended next step — not authored here)

Per `docs/guides/adr-process.md`, two decisions warrant ADRs. They are flagged as
**candidates** for human sign-off rather than authored speculatively:

1. **`--format` value-enum as the canonical output selector** (Decision 1):
   establishes a convention (value-enum replaces `--json`/`--no-tui` booleans as
   the selector; booleans become aliases) the team should follow for future
   machine-output formats. Hard-ish to reverse once the flag is public.
2. **Per-format shared emitter + per-command adapters, no unified finding model**
   (Decision 3): records the deliberate decision *not* to refactor the engine
   crates onto a shared finding model, and sets the pattern for future formats.

Run `pnpm adr:check` for the next available number before authoring. Both should
be filed **Proposed** alongside the SARIFOUT-001/-002 implementation PRs, not
ahead of sign-off on this design.

## Open Decisions Flagged For Human Sign-Off

All three were **ratified by the operator on 2026-05-29** with the recommended
options:

1. **Flag surface** — accepted the `--format` value-enum (with `--json` as
   alias). **Implementation note:** narrowed from a *global* flag to a
   **per-command** flag on `check` / `gate` / `audit`, because `--format` already
   collides with the existing domain flags on `anvil export` and `anvil validate`
   (`clap` rejects a colliding global arg). See ADR-056's Amendment. All other
   semantics (value space, precedence, `--json` alias, SARIF opt-in) are
   unchanged.
2. **Module home** — accepted the new dedicated `SARIFOUT` module.
3. **Shared model** — accepted the thin shared SARIF emitter + per-command
   adapters (no engine refactor); the second ADR for this lands with
   `SARIFOUT-002`.
