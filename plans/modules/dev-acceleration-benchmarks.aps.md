<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# Dev Acceleration Benchmarks

| ID     | Owner  | Priority | Status | Progress |
| ------ | ------ | -------- | ------ | -------- |
| DEVACC | @aneki | medium   | Ready  | 0/12     |

**Last reviewed:** 2026-07-16 — design at
[`docs/architecture/dev-acceleration-benchmark-spec.md`](../../docs/architecture/dev-acceleration-benchmark-spec.md);
first wave DEVACC-001..006 promoted **Ready**. Measures assistant-facing
**Developer Acceleration** (graph context + pre-write validation + skill loop)
for token efficiency and task success **with Anvil on vs off**. Distinct from
RLB/kernel resource benches and from GCTX-031 payload micro-numbers.

## Purpose

Give Anvil a **claim-safe, on-demand** benchmark suite for real coding and
planning tasks so token-efficiency and acceleration claims are reproducible —
not only synthetic payload ratios (GCTX-031).

## Default cadence (product decision)

| Cadence | Default | Notes |
| ------- | ------- | ----- |
| **On-demand local** | **On** (the default product path) | Operator runs Tier A / Tier B when needed; documented command surface |
| **Nightly scheduled** | **Off** — opt-in | DEVACC-011 only; not required for module Complete |
| **CI / PR gate** | **Off** — opt-in | DEVACC-012 only; not required for module Complete |

GCTX-031 goldens and other free deterministic unit tests that already live in
`cargo test -p anvil-bench` may remain in PR CI; they are **not** this module's
task-level suite and do not make DEVACC a default CI gate.

## Background

Developer Acceleration is the public loop: graph context in, pre-write
validation on agent edits, skill-guided tool use, and a fast save-time path
([tutorial](../../docs/public/anvil/tutorials/developer-acceleration.md)).

GCTX-031 (`token_reduction` in `anvil-bench`) already proves identity-only
impact payloads are ~87% smaller than neighbourhood file reads on synthetic
graphs. That is a **payload micro-benchmark**. It does not measure full agent
tasks, planning workflows, rework after bad writes, or quality-conditioned
token means.

This module implements the task-level suite specified in the architecture doc.

## In Scope

- Scenario catalogue and report schema (`devacc-bench-1`) for navigate / edit /
  plan / guard / multi-stage tasks
- **Tier A** — deterministic scripted tool traces (no LLM); unit goldens
  available as tests, suite run on demand
- **Tier B** — pinned-model agent runs, on-demand (credentials required)
- Fixtures (`mini-ts-service`, `mini-rs-lib`, `mini-aps-plan`) and gold rubrics
- Claims policy packaging (scenario-id, arm, tier, model, Anvil SHA)
- Opt-in nightly and opt-in CI gate wiring (explicit work items; off by default)

## Out of Scope

- Engine CPU/RSS/latency (owned by RLB, Criterion, ADR-031 kernel benches)
- Changing GCTX-031 payload micro-bench ownership (stays GCTX / `anvil-bench`)
- Live customer-repo or fleet telemetry as claim evidence (Tier C optional later;
  USAGE/ADR-107 consent)
- Multi-vendor "context product" bake-offs (fairness protocol would be separate)
- Making nightly or PR-blocking CI the default posture

## Interfaces

**Depends on:**

- GCTX surface shipped (GCTX-010..032 Released/Shipped via v0.9.0-beta) —
  identity tools, estimator, impact/tests APIs
- `anvil_validate_write` / `anvil_apply_patch` MCP gate (RMCP)
- `anvil-developer-functions` skill bundle
- `crates/anvil-bench` harness patterns + `estimate_gctx_tokens` (GCTX-020)
- Design authority:
  [`docs/architecture/dev-acceleration-benchmark-spec.md`](../../docs/architecture/dev-acceleration-benchmark-spec.md)

**Coordinates with:**

- **RLB** — process resource benches; do not mix token claims into RLB history
- **TCOV-026** — only if DEVACC elects history schema under `benchmarks/history/`
- **EVALCI** — policy eval CI is a separate gate; do not reuse its default
  report-only PR step for DEVACC unless DEVACC-012 is intentionally promoted

**Exposes:**

- On-demand runner command(s) for Tier A and Tier B
- JSON run records + aggregate report under `benchmark-results/devacc-*`
  (gitignored) and optional reviewed history under `benchmarks/history/devacc/`
- Scenario ids `DEVACC-SCN-*` as the only ids allowed in external acceleration
  claims (per design claims policy)

## Acceptance Criteria

- [ ] Operator can run Tier A on demand without model credentials and get
      deterministic token/tool metrics for navigation scenarios
- [ ] Operator can run Tier B on demand with a pinned model and get paired
      `control` vs `full-accel` results with success rubrics
- [ ] Published claims cite scenario id, arm, tier, model, and Anvil SHA
- [ ] Nightly schedule remains off unless DEVACC-011 is deliberately enabled
- [ ] PR CI does not block on DEVACC task suites unless DEVACC-012 is
      deliberately enabled
- [ ] Quality veto: failed tasks are not averaged into token-win headlines

---

## Work Items

### Phase 0 — Catalogue and contracts

#### DEVACC-001: Freeze scenario catalogue and report schema in-repo

- **Status:** Ready
- **Intent:** Land the scenario id catalogue, arm vocabulary, and
  `devacc-bench-1` report schema as committed artefacts so harness and docs
  share one contract.
- **Expected Outcome:** Catalogue YAML (or equivalent) lists SCN-01..04, 10–14,
  20–22, 30–32, 40 with class, fixture, arms, tiers, and primary metrics;
  report schema is validated by a unit test.
- **Validation:** `cargo test -p anvil-bench -- devacc_catalogue` (or the
  package that owns the catalogue once chosen)
- **Files:** `benchmarks/devacc/`, `docs/architecture/dev-acceleration-benchmark-spec.md`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** none

### Phase 1 — Tier A spine (on-demand + free unit goldens)

#### DEVACC-002: Tier A runner spine

- **Status:** Ready
- **Intent:** Add a deterministic no-LLM runner that executes tool scripts,
  scores payloads with `estimate_gctx_tokens`, and emits `devacc-bench-1`
  records for control vs treatment arms.
- **Expected Outcome:** One documented on-demand command runs Tier A for a
  scenario id and writes JSON under `benchmark-results/devacc-*`.
- **Validation:** documented local command exits 0 on a smoke scenario; unit
  tests cover report emission
- **Files:** `crates/anvil-bench/`, `crates/anvil-bench/README.md`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** DEVACC-001

#### DEVACC-003: Navigation Tier A scenarios (SCN-01, 02, 04)

- **Status:** Ready
- **Intent:** Implement find-symbol, blast-radius, and affected-tests scripts
  with goldens so graph context token savings are reproducible without a model.
- **Expected Outcome:** Gold token/tool tables for SCN-01/02/04 on the S-scale
  fixture; intentional payload drift fails tests until goldens update in-commit.
- **Validation:** `cargo test -p anvil-bench -- devacc_scn_navigate`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** DEVACC-002, DEVACC-006

#### DEVACC-004: Guard Tier A scenarios (SCN-30, 31, 32)

- **Status:** Ready
- **Intent:** Script secret near-miss, boundary-violation, and clean-patch
  validation-tax cases so safety wins and token tax are measured separately.
- **Expected Outcome:** True-positive block rates and validation-tax token
  deltas report for the three guard scenarios; no silent mix into pure token
  means.
- **Validation:** `cargo test -p anvil-bench -- devacc_scn_guard`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** DEVACC-002, DEVACC-006

#### DEVACC-005: Edit ceiling scripts (SCN-10, 11, 12)

- **Status:** Ready
- **Intent:** Capture ideal (ceiling) tool-use scripts for small fix, cross-layer
  feature, and public rename so maximum achievable savings are known before
  free-form agent runs.
- **Expected Outcome:** Ceiling token tables labelled `ceiling` (not
  `achieved`); paths and rubrics reuse fixture gold.
- **Validation:** `cargo test -p anvil-bench -- devacc_scn_edit_ceiling`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** DEVACC-002, DEVACC-006

#### DEVACC-006: Fixtures and gold rubrics

- **Status:** Ready
- **Intent:** Commit self-contained fixtures (`mini-ts-service` S/M,
  `mini-rs-lib` S, `mini-aps-plan` S) with gold locations, impact sets, and
  success rubrics; no real secrets.
- **Expected Outcome:** Fixtures live under `benchmarks/fixtures/devacc/`;
  graph warm + ready assert is documented for treatment arms.
- **Validation:** fixture smoke command or tests that load gold JSON and build
  graphs where applicable
- **Files:** `benchmarks/fixtures/devacc/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** DEVACC-001

### Phase 2 — Tier B on-demand agent harness

#### DEVACC-007: Tier B on-demand agent runner

- **Status:** Draft
- **Intent:** Provide a pinned-model, credentialed, on-demand runner for
  `control` / `gctx-only` / `full-accel` / `validate-only` arms with hard turn
  and wall budgets.
- **Expected Outcome:** Documented command runs one scenario × arm, records
  provider token usage and rubric results; default posture is manual invoke
  only (no schedule, no PR gate).
- **Validation:** dry-run / smoke path without network where possible; full path
  documented with required env vars; one successful local paired run recorded
  in validation notes when landing
- **Files:** `crates/anvil-bench/` or harness under `scripts/bench/devacc/`,
  README
- **Confidence:** low
- **Priority:** High
- **Dependencies:** DEVACC-001, DEVACC-006
- **Notes:** Headless client choice (Claude Code / custom MCP host / other) is
  an open design question in the architecture spec; resolve before promoting
  this item to Ready.

#### DEVACC-008: Tier B MVP evidence (SCN-01, 02, 10)

- **Status:** Draft
- **Intent:** Produce internal paired evidence (n≥10 where practical) for
  navigation and one edit scenario so achieved token reduction is known.
- **Expected Outcome:** Reviewed report under `benchmark-results/` with optional
  history candidate; success rates and quality veto applied; no public hero
  claim yet.
- **Validation:** report schema validates; operator checklist in module or
  README satisfied
- **Confidence:** low
- **Priority:** High
- **Dependencies:** DEVACC-007, DEVACC-003

### Phase 3 — Planning, headline, claims

#### DEVACC-009: Planning scenarios (SCN-20, 21, 22)

- **Status:** Draft
- **Intent:** Cover APS-shaped planning tasks (next Ready item, outline, unblock
  set) so planning token cost is measured, not only code edits.
- **Expected Outcome:** Tier A and/or Tier B coverage for SCN-20–22 on
  `mini-aps-plan` with gold rubrics.
- **Validation:** scenario tests and/or on-demand Tier B report
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** DEVACC-006, DEVACC-002

#### DEVACC-010: Headline scenario and claims package (SCN-40)

- **Status:** Draft
- **Intent:** Land the multi-stage "feature afternoon" scenario and the public
  claims packaging rules (ids, caveats, history pointer) so marketing and docs
  can cite evidence safely.
- **Expected Outcome:** SCN-40 runnable on-demand on Tier B; claims section in
  `anvil-bench` README or `docs/testing/` states which numbers are publishable
  and the caveats table; GCTX-031 remains the only payload-micro citation.
- **Validation:** docs check + one internal SCN-40 report (or explicit deferral
  note if cost-blocked, keeping claims package text complete)
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** DEVACC-008

### Phase 4 — Opt-in automation (not required for Complete)

#### DEVACC-011: Opt-in nightly schedule

- **Status:** Proposed
- **Intent:** Optional workflow to run a bounded DEVACC slice on a schedule or
  `workflow_dispatch`, **disabled by default** (no automatic nightly cost).
- **Expected Outcome:** Workflow file or documented enablement path that
  operators can turn on; default repository configuration leaves it off.
- **Validation:** workflow parses; default path does not schedule DEVACC on
  main; enablement documented
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** DEVACC-002 (Tier A minimum); DEVACC-007 if nightly includes
  Tier B
- **Notes:** Opt-in. **Not required** for module Complete.

#### DEVACC-012: Opt-in CI / PR gate

- **Status:** Proposed
- **Intent:** Optional PR or required-check wiring for a cheap Tier A subset,
  **off by default** so PRs do not gain a new blocking or noisy gate without
  an explicit decision.
- **Expected Outcome:** Documented opt-in (flag, workflow_call, or non-required
  report-only step behind an explicit enable). Default CI remains unchanged.
- **Validation:** with opt-in off, `pnpm validate` / rust CI path unchanged;
  with opt-in on (doc-only or test workflow), Tier A smoke runs
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** DEVACC-003
- **Notes:** Opt-in. **Not required** for module Complete. Prefer report-only
  before any hard-fail posture (align with ADR-002 / EVALCI phasing lessons).

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Tier B flakiness / model drift | High | Medium | Pin model; n-run stats; history invalidates on model change |
| Strawman control arms | Medium | High | Competent search+read control required by design |
| Nightly/CI cost surprise | Medium | Medium | Default off; 011/012 opt-in only |
| Claims cite only GCTX-031 micro | Medium | Medium | Claims package (010) separates micro vs task-level |
| Gate false blocks thrash agents | Medium | High | SCN-32 validation tax + false-block rate metrics |
| Confusing RLB vs DEVACC history | Low | Medium | Separate history path and report schema |

## Decisions

1. **New vertical module (`DEVACC`)** — not an RLB extension; assistant token
   value is a different product question than process CPU/RSS.
2. **On-demand is the default cadence** — nightly and CI are opt-in work items
   and are not required for Complete.
3. **GCTX-031 stays the payload micro-benchmark** — DEVACC does not absorb or
   replace it.
4. **Quality veto on token means** — failed runs cannot inflate reduction %.
5. **Design authority** lives at
   `docs/architecture/dev-acceleration-benchmark-spec.md` until superseded by
   as-built notes.
6. **First Ready wave is Tier A** (DEVACC-001..006). Tier B stays Draft until
   the headless agent driver is chosen (open question in the architecture
   spec).

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 0 — Catalogue | 1 | Ready |
| 1 — Tier A spine | 5 | Ready |
| 2 — Tier B on-demand | 2 | Draft |
| 3 — Planning + claims | 2 | Draft |
| 4 — Opt-in automation | 2 | Proposed (optional) |
| **Total** | **12** | **0/12** (011/012 optional for Complete) |
