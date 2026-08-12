# Dev Acceleration Benchmark Specification

| Type | Authority | Owner                                                                                                                 | Status | Freshness                                                                                                                                                                                                                                                  |
| ---- | --------- | --------------------------------------------------------------------------------------------------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Spec | Derived   | DEVACC ([`plans/modules/dev-acceleration-benchmarks.aps.md`](../../plans/modules/dev-acceleration-benchmarks.aps.md)) | Live   | Last reviewed 2026-08-13 against `docs/public/anvil/tutorials/developer-acceleration.md` — DOCFRESH-005 added only governance frontmatter to that tutorial stub, no content change; authored 2026-07-15, APS module Ready wave 001–006 promoted 2026-07-16 |

| Upstream                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Downstream                                                                                                                                                        |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`developer-acceleration.md`](../public/anvil/tutorials/developer-acceleration.md), [`ai-context-delivery.md`](../guides/ai-context-delivery.md), GCTX-031 (`token_reduction`), [`kernel-benchmarking-spec.md`](./kernel-benchmarking-spec.md), ADR-031, ADR-083 `docs/public/anvil/tutorials/developer-acceleration.md`, `docs/guides/ai-context-delivery.md`, `docs/architecture/kernel-benchmarking-spec.md`, `plans/decisions/031-validation-latency-rubric.md`, `plans/decisions/083-gctx-mcp-delivery-target.md` | [`plans/modules/dev-acceleration-benchmarks.aps.md`](../../plans/modules/dev-acceleration-benchmarks.aps.md) (DEVACC), marketing claims gate, optional nightly/CI |

**Status:** Live design authority for measuring the **Developer Acceleration**
surface end-to-end. Execution is authorised for **Ready** DEVACC work items
(first wave: DEVACC-001..006). **Default cadence is on-demand**; nightly and CI
gates are opt-in (DEVACC-011 / DEVACC-012) and are not required for module
Complete.

---

## Purpose

Define a **proper, claim-safe benchmark suite** for Anvil's developer
acceleration loop: the product surface that puts Anvil inside an AI coding
agent's workflow so the agent spends fewer tokens, fewer thrashing tool calls,
and fewer failed write loops to complete real work.

The suite answers one commercial and engineering question honestly:

> For fixed, real-world coding and planning tasks, how does **assistant cost and
> success** change **with Anvil on** versus **with Anvil off**?

**Primary metric family: token efficiency** (input, output, and total session
tokens per successful task). Secondary metrics capture quality, rework, latency,
and tool-call economy so token wins that produce worse code cannot be published
as "acceleration".

This is **not** another kernel/resource micro-benchmark. Engine latency and RSS
remain owned by [`kernel-benchmarking-spec.md`](./kernel-benchmarking-spec.md),
`anvil-bench` stress scenarios, and RLB. This spec owns **assistant-facing
value** of the acceleration loop.

---

## What "Developer Acceleration" means here

Product definition (from the public tutorial and live MCP surface):

| Loop stage          | Surface                                                  | Role in acceleration                                     |
| ------------------- | -------------------------------------------------------- | -------------------------------------------------------- |
| **Understand**      | GCTX tools + `graph://` resources                        | Replace blind full-file reads with bounded graph answers |
| **Plan / target**   | `impact_of_change`, `affected_tests`, callers/dependents | Narrow the change surface and test set before editing    |
| **Write safely**    | `anvil_validate_write`, `anvil_apply_patch`              | Catch secrets/anti-patterns/boundary issues pre-land     |
| **Skill behaviour** | `anvil-developer-functions` skill                        | Teach the agent _when_ to use the above                  |
| **Save-time close** | `anvil watch` / daemon save-time path                    | Fast feedback after land; out of band of MCP tokens      |

The benchmark suite must exercise **stages 1–4** for token claims. Stage 5 is
measured for wall-time / iteration cost, not model tokens.

---

## Relationship to existing measurement

| Existing artefact                                    | What it proves today                                                                                    | Gap this spec fills                                                               |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| **GCTX-031 `token_reduction`**                       | Identity-only `ImpactOutcome` payload is ~87% smaller than neighbourhood file reads on synthetic graphs | Single query shape; no agent loop; no planning; no pre-write path; synthetic only |
| **HTML vs Markdown APS token experiment** (archived) | Plan _format_ changes token cost of loading APS modules                                                 | Not Anvil-on vs Anvil-off; format experiment only                                 |
| **EVAL / EVALCI**                                    | Policy eval harness regression                                                                          | Trust/policy correctness, not agent productivity                                  |
| **Criterion / RLB / `anvil-bench` stress**           | Engine latency, CPU, RSS                                                                                | Not assistant context cost                                                        |
| **`validate_write_tier` Criterion**                  | Pre-write path wall time                                                                                | Not agent token cost of using (or skipping) the gate                              |

**Rule:** GCTX-031 remains the **payload micro-benchmark** and must stay green.
This suite is the **task-level macro-benchmark**. Macro claims must never cite
only micro numbers.

---

## Design principles

1. **Apples-to-apples tasks.** Control and treatment solve the same fixed task
   against the same fixture repo and the same success rubric.
2. **Honest baselines.** The control is not "read the whole monorepo". It is a
   competent graph-less agent using normal tools (`grep`, `read_file`,
   `list_dir`, tests). Whole-repo is reported only as a pathological ceiling
   (same honesty rule as GCTX-031).
3. **Token primary, quality veto.** A run that burns fewer tokens but fails the
   task, or lands known anti-patterns, does **not** count as a win.
4. **Determinism where possible; statistical honesty where not.** Tier A is
   fully deterministic. Tier B reports n≥N runs with confidence intervals.
5. **No secret egress in fixtures.** Fixture code has no real credentials.
   Snippet egress stays default-off unless a scenario explicitly opts in and
   documents the privacy cost.
6. **Same-hardware / same-model rule.** Cross-run comparisons require pinned
   model id, temperature (0 where available), client harness version, Anvil
   binary SHA, and host class — same discipline as `benchmarks/README.md`.
7. **Claims gate.** External/marketing claims may only cite numbers produced by
   this suite (or GCTX-031 for the narrower payload claim), with the caveats
   table from the report attached.

---

## Measurement model

### Arms

| Arm id          | Anvil MCP             | Skill installed             | Daemon / graph warm    | Notes                                       |
| --------------- | --------------------- | --------------------------- | ---------------------- | ------------------------------------------- |
| `control`       | off                   | no Anvil skill              | n/a                    | Standard agent tools only                   |
| `gctx-only`     | GCTX tools only       | optional thin prompt        | warm graph required    | Isolates context savings without validation |
| `full-accel`    | GCTX + validate/apply | `anvil-developer-functions` | warm graph + gate live | Full Developer Acceleration path            |
| `validate-only` | validate/apply only   | skill subset                | gate live              | Isolates rework / blocked-bad-write effects |

Default published comparison is **`control` vs `full-accel`**. The partial arms
exist to attribute _why_ a win happened (context vs gate vs skill).

### Primary metrics (token efficiency)

Per task, per arm, per successful run:

| Metric                       | Definition                                                                | Source                               |
| ---------------------------- | ------------------------------------------------------------------------- | ------------------------------------ |
| `tokens_in`                  | Sum of prompt/input tokens across the session                             | Provider usage API or harness meter  |
| `tokens_out`                 | Sum of completion/output tokens                                           | Provider usage API or harness meter  |
| `tokens_total`               | `tokens_in + tokens_out`                                                  | derived                              |
| `tokens_tool_results`        | Tokens attributable to tool/function result payloads                      | harness classification               |
| `tokens_file_reads`          | Subset of tool-result tokens from whole-file reads                        | harness classification               |
| `tokens_gctx`                | Subset from GCTX tool/resource payloads                                   | harness classification               |
| `context_budget_peak`        | Max observed context window occupancy mid-task                            | harness if available; else estimate  |
| `token_reduction_vs_control` | `1 - tokens_total_treatment / tokens_total_control` on **paired** success | derived; only when both arms succeed |

**Estimator policy:**

- **Tier A (scripted):** use `estimate_gctx_tokens` (`gctx-simple-v1`) for GCTX
  payloads **and** the same estimator for file-read baselines so ratios stay
  internally consistent. Disclose estimator bias (lexical vs `bytes/4`) exactly
  as GCTX-031 does.
- **Tier B (live agent):** prefer the provider's official token counts for the
  pinned model. Optionally re-score tool payloads with `gctx-simple-v1` for
  cross-model comparability of _payload_ cost only — never mix the two into one
  ratio without labelling.

### Secondary metrics

| Metric                     | Definition                                     | Why it matters                                    |
| -------------------------- | ---------------------------------------------- | ------------------------------------------------- |
| `tool_calls`               | Total tool invocations                         | Thrash / exploration cost                         |
| `file_reads`               | Count of full-file or large partial reads      | Naive exploration                                 |
| `gctx_calls`               | Count of GCTX tool calls                       | Adoption of graph surface                         |
| `validate_calls`           | Count of validate/apply calls                  | Gate engagement                                   |
| `blocked_writes`           | Writes refused by gate (`block`)               | Caught bad landings (positive when true positive) |
| `false_block_rate`         | Blocks on gold-good patches (fixture-labelled) | Gate must not thrash the agent                    |
| `wall_ms`                  | End-to-end task wall time                      | Human-felt acceleration                           |
| `turns`                    | Agent turn count                               | Loop length                                       |
| `task_success`             | Boolean against fixed rubric                   | Quality veto                                      |
| `rubric_score`             | 0–1 composite (correctness, scope, tests)      | Graded success                                    |
| `rework_cycles`            | Write → fail verify → rewrite loops            | Without Anvil often higher                        |
| `tests_run` / `tests_pass` | Verification evidence                          | Must not claim success without evidence           |
| `anvil_tool_p95_ms`        | Latency of Anvil tools in-session              | Acceleration must not die to slow MCP             |

### Derived headline stats (publishable only under conditions)

For each scenario with ≥N successful paired runs:

- Median `token_reduction_vs_control` for `full-accel`
- Median `wall_ms` ratio
- Success-rate delta (`success_full-accel − success_control`)
- Attribution split: fraction of token delta explained by `tokens_file_reads` ↓
  vs other

**Publish gate:** n ≥ 10 paired successes **or** Tier A deterministic golden
(where applicable); model id + Anvil SHA recorded; quality veto not triggered.

---

## Tiered methodology

### Tier A — Deterministic scripted tool traces (CI-safe unit goldens; suite on-demand)

**What:** No LLM. A fixed _tool script_ (ordered list of tool calls + expected
arguments) is executed against a fixture workspace. The harness records the
exact bytes returned by each tool and scores them with `estimate_gctx_tokens`
(and byte counts). Control scripts use only filesystem/grep-shaped tools;
treatment scripts use GCTX + optional validate.

**Why first:** Reproducible, free locally, extends GCTX-031 from one query shape
to a catalogue of **task scripts** without model noise.

**Pass criteria:** Golden token tables stay stable within tolerance 0 (exact)
for fixed fixtures; intentional product payload changes update goldens in the
same commit.

**Home:** New scenarios under `crates/anvil-bench` (alongside
`token_reduction`), or a sibling binary if the surface grows large. Prefer
extending `anvil-bench` until the suite needs a real agent runtime.

**Cadence:** run **on-demand** by default. Unit goldens may ship in `cargo test`
when present; that is not a named DEVACC PR gate (see DEVACC-012).

### Tier B — Agent-in-the-loop harness (on-demand)

**What:** A pinned coding agent runs each scenario prompt with arms above.

**Controls:**

- Fresh worktree / clean fixture clone per run
- Warm graph once before the timed window (`anvil start` + `graph://stats`
  ready)
- Temperature 0 / deterministic sampling where the model allows
- Max turn budget and max wall budget (hard stop → `task_success=false`)
- No network except the model provider (fixtures are self-contained)

**Outputs:** JSONL run records + aggregated report (markdown + JSON) under
`benchmark-results/devacc-<timestamp>/` (gitignored) with optional golden
summaries committed under `benchmarks/history/devacc/` after review.

**Cadence:** **on-demand only** by default (credentials required). Nightly is
opt-in (DEVACC-011). Not every PR.

### Tier C — Human / fleet observational (optional, consent-gated)

Usage analytics and human diary studies can _corroborate_ Tier B but never
replace it for public claims. ADR-107 / USAGE consent rules apply. Out of scope
for v1 of this harness beyond a hook point for consented session metrics.

---

## Scenario catalogue

Each scenario has:

- **id** — stable (`DEVACC-SCN-…`)
- **class** — `navigate` | `edit` | `plan` | `guard` | `multi`
- **fixture** — named repo snapshot
- **prompt** — fixed user brief (Tier B) or tool script (Tier A)
- **success rubric** — objective checks (tests, AST/file assertions, plan JSON)
- **arms** — which arms are required
- **primary claim** — what token story it supports

### A. Navigation / understanding (token-heavy without graph)

| ID              | Title                           | Task                                                   | Success rubric (sketch)                                                        | Why it is real                    |
| --------------- | ------------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------ | --------------------------------- |
| `DEVACC-SCN-01` | Find symbol + callers           | "Where is `X` defined and who calls it?"               | Correct file:line set; caller set matches gold within heuristic mark           | Daily "how does this work"        |
| `DEVACC-SCN-02` | Blast radius before rename      | "What breaks if I rename public API `Y`?"              | Dependent file set ⊇ gold depth-2 set; no full-repo read required in treatment | Pre-refactor impact               |
| `DEVACC-SCN-03` | Cold repo orientation           | "Explain the module map of this service in ≤N bullets" | Mentions gold modules; does not invent paths                                   | New-joiner / new-agent cold start |
| `DEVACC-SCN-04` | Affected tests for a change set | Given 3 changed paths, list tests to run               | `known_tests` / fixture gold; coverage gaps called out                         | Pre-verify targeting              |

**Token thesis:** Treatment replaces multi-file reads with identity graph
payloads. Closest extension of GCTX-031.

### B. Coding / edit loops

| ID              | Title                        | Task                                                | Success rubric                                                    | Why it is real         |
| --------------- | ---------------------------- | --------------------------------------------------- | ----------------------------------------------------------------- | ---------------------- |
| `DEVACC-SCN-10` | Small pure-function fix      | Fix failing unit test in one leaf module            | Tests green; diff limited to allowed paths                        | Minimal edit           |
| `DEVACC-SCN-11` | Cross-layer feature slice    | Add endpoint → service → store method with fixtures | Contract tests green; no illegal layer import                     | Classic vertical slice |
| `DEVACC-SCN-12` | Public API refactor          | Rename exported symbol and update call sites        | Build + tests green; no leftover old name in non-allowed paths    | Graph-guided rename    |
| `DEVACC-SCN-13` | Bugfix with test targeting   | Failing integration symptom; agent must pick tests  | Gold test set run; bug fixed; no shotgun full suite unless needed | Uses `affected_tests`  |
| `DEVACC-SCN-14` | Multi-file consistent change | Thread a new required field through DTO + handlers  | Typecheck/tests green; all call sites updated                     | Fan-out edits          |

**Token thesis:** Savings from fewer exploratory reads **and** fewer rework
turns after wrong-file edits. Measure both `tokens_*` and `rework_cycles`.

### C. Planning / APS-shaped work

| ID              | Title                            | Task                                                                            | Success rubric                                              | Why it is real                 |
| --------------- | -------------------------------- | ------------------------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------ |
| `DEVACC-SCN-20` | Next Ready item selection        | Given a small APS module fixture, pick the next Ready item and justify deps     | Correct id; deps cited accurately                           | Planning council / loop pickup |
| `DEVACC-SCN-21` | Implementation outline from plan | Produce a step list for one Ready work item without loading whole monorepo docs | Steps cover gold checklist; cite only allowed context files | Plan → code handoff            |
| `DEVACC-SCN-22` | Impact of plan change            | "If we drop item Z, what Ready items unblock?"                                  | Gold unblock set                                            | Planning graph reasoning       |

**Token thesis:** Planning burns tokens on large markdown. Treatment may use
structured export / graph / targeted reads. Reuse lessons from the archived
HTML-vs-Markdown experiment: **measure the task**, not only the format.

### D. Guard / pre-write (quality + rework, secondary tokens)

| ID              | Title                      | Task                                                    | Success rubric                                                             | Why it is real     |
| --------------- | -------------------------- | ------------------------------------------------------- | -------------------------------------------------------------------------- | ------------------ |
| `DEVACC-SCN-30` | Secret near-miss           | Agent is steered to write a "sample" secret-like string | Control may land it; `full-accel` must `block` or rewrite cleanly          | Real agent footgun |
| `DEVACC-SCN-31` | Boundary violation import  | Prompt asks for a forbidden layer import                | Treatment refuses or rewrites to legal path; control often lands violation | Architecture guard |
| `DEVACC-SCN-32` | Clean patch validation tax | Gold-good small edit                                    | Both arms succeed; measure extra tokens/latency of validate calls          | Cost of safety     |

**Token thesis:** Guard scenarios may **increase** tokens slightly (extra tool
calls) while reducing **rework and bad lands**. Report both; never hide a token
tax that buys safety.

### E. Multi-stage "day in the life" (headline scenario)

| ID              | Title             | Task                                                                       | Success rubric                       | Why it is real            |
| --------------- | ----------------- | -------------------------------------------------------------------------- | ------------------------------------ | ------------------------- |
| `DEVACC-SCN-40` | Feature afternoon | Orient → impact → implement SCN-11-sized slice → validate → targeted tests | Composite rubric; wall time + tokens | Marketing-grade full loop |

Run SCN-40 only on Tier B; too expensive for every CI. It is the **only**
scenario allowed in external "developer acceleration" hero claims unless a
narrower claim explicitly names its scenario id.

---

## Fixtures

### Requirements

1. **Self-contained** mini-repos under `benchmarks/fixtures/devacc/` (or
   generated deterministically like GCTX-031 when graph topology is the point).
2. **Languages:** at least TypeScript and Rust (Anvil's first-class pair);
   optional Python later.
3. **Scale bands:**
   - `S` — ≤15 source files (fast local Tier A)
   - `M` — ~50–80 files (default Tier B)
   - `L` — ~300 files (opt-in nightly only; proves neighbourhood ≠ whole-repo)
4. **Gold artefacts** committed beside fixtures:
   - expected symbol locations
   - expected depth-2 dependent sets
   - expected test attribution
   - allowed path globs for diffs
   - graded rubric JSON
5. **No real secrets, no private customer code.** Synthetic names only.
6. **Pinned graph readiness.** Fixture ships with a script that warms the graph
   and asserts `graph://stats` ready before treatment arms start.

### Suggested initial fixture set

| Fixture id        | Scale | Contents                                                               | Scenarios               |
| ----------------- | ----- | ---------------------------------------------------------------------- | ----------------------- |
| `mini-ts-service` | S/M   | Layered HTTP service (routes/services/store), unit + integration tests | 01–04, 10–14, 30–32, 40 |
| `mini-rs-lib`     | S     | Small Rust crate with public API + callers                             | 01–02, 12               |
| `mini-aps-plan`   | S     | Tiny APS module + stub code map                                        | 20–22                   |

---

## Harness architecture (intended)

```text
                    ┌─────────────────────────────┐
                    │  Scenario catalogue (YAML)  │
                    │  id, prompt, arms, rubric   │
                    └─────────────┬───────────────┘
                                  │
              ┌───────────────────┼───────────────────┐
              ▼                   ▼                   ▼
        Tier A runner       Tier B agent runner   Report aggregator
        (no LLM)            (pinned model+MCP)    (JSON + MD)
              │                   │                   │
              ▼                   ▼                   ▼
        tool script exec    control | gctx-only |   history/devacc/
        + gctx estimator    full-accel | validate   golden summaries
```

### Scenario definition sketch (YAML)

```yaml
id: DEVACC-SCN-02
class: navigate
fixture: mini-ts-service
scale: M
arms: [control, gctx-only, full-accel]
tier: [A, B]
prompt: |
  In this repository, report the blast radius of renaming the exported
  function `createOrder`. List dependent files and direct callers. Do not
  modify files.
rubric:
  type: structured_json
  schema: blast_radius_v1
  gold: fixtures/mini-ts-service/gold/createOrder.blast.json
tier_a_scripts:
  control: scripts/scn-02-control.json
  gctx-only: scripts/scn-02-gctx.json
metrics:
  primary: [tokens_total, tokens_tool_results, tokens_file_reads]
  secondary: [tool_calls, file_reads, gctx_calls, wall_ms, task_success]
```

### Report schema (v1)

```json
{
  "schema_version": "devacc-bench-1",
  "anvil_sha": "...",
  "model": "…",
  "host_class": "…",
  "scenario": "DEVACC-SCN-02",
  "arm": "full-accel",
  "tier": "B",
  "task_success": true,
  "rubric_score": 1.0,
  "tokens_in": 0,
  "tokens_out": 0,
  "tokens_total": 0,
  "tokens_tool_results": 0,
  "tokens_file_reads": 0,
  "tokens_gctx": 0,
  "tool_calls": 0,
  "file_reads": 0,
  "gctx_calls": 0,
  "validate_calls": 0,
  "blocked_writes": 0,
  "rework_cycles": 0,
  "wall_ms": 0,
  "turns": 0
}
```

Paired reductions are computed offline by the aggregator, never by inventing a
control number inside a treatment-only run.

---

## Phased delivery (APS: DEVACC)

Execution authority lives in
[`plans/modules/dev-acceleration-benchmarks.aps.md`](../../plans/modules/dev-acceleration-benchmarks.aps.md).
Mapping:

| Phase                     | Goal                                     | APS items                                | Exit criteria                                |
| ------------------------- | ---------------------------------------- | ---------------------------------------- | -------------------------------------------- |
| **0 — Catalogue**         | Scenario ids + report schema committed   | DEVACC-001 (**Ready**)                   | Catalogue + schema unit-tested               |
| **1 — Tier A spine**      | Deterministic scripts, fixtures, goldens | DEVACC-002..006 (**Ready**)              | On-demand Tier A; free unit goldens          |
| **2 — Tier B on-demand**  | Pinned-model agent runner + MVP evidence | DEVACC-007..008 (Draft)                  | Internal paired report; no public hero claim |
| **3 — Planning + claims** | SCN-20–22 + SCN-40 + claims packaging    | DEVACC-009..010 (Draft)                  | Claims may cite scenario ids with caveats    |
| **4 — Opt-in automation** | Nightly schedule and/or PR CI            | DEVACC-011..012 (**Proposed**, optional) | Enabled only by deliberate operator choice   |

**Default cadence:** on-demand. Nightly and CI are **off by default** and are
**not** required for DEVACC Complete.

**Non-goals for v1:** multi-model leaderboards, live customer-repo benches,
snippet-egress on by default, comparing Anvil to third-party context products
(optional later under a separate fairness protocol), default PR-blocking DEVACC
gates.

---

## Honesty caveats (must ship with every report)

1. **Estimator ≠ billing tokens (Tier A).** `gctx-simple-v1` over-weights
   punctuation-dense source relative to sparse identity JSON; real BPE ratios
   are often a few points lower (GCTX-031 disclosure).
2. **Identity-only default.** Enabling snippet egress raises graph-side tokens
   and changes privacy posture; those runs are labelled `egress=on` and are not
   mixed into identity-only means.
3. **Competent control.** Beating a strawman whole-repo reader is not a claim.
   Control must use search + selective reads.
4. **Success-conditioned means.** Token averages that drop failed runs without
   reporting success rate are forbidden.
5. **Safety tax.** Guard arms may spend more tokens; report safety wins
   separately from pure token reduction.
6. **Model drift.** A new model version invalidates Tier B history until
   re-baselined.
7. **Skill compliance.** `full-accel` assumes the agent follows
   `anvil-developer-functions`. If the harness cannot install/enforce the skill,
   label the arm `tools-available` not `skill-followed`.
8. **Daemon readiness.** Treatment arms that run against a cold/unready graph
   are invalid for token claims (they measure degradation behaviour instead).

---

## CI and operator commands (intended)

Default posture is **on-demand**. Exact CLI shape is implementation detail
(DEVACC-002 / DEVACC-007); the contract:

```bash
# Tier A — free, deterministic, on-demand (default path)
cargo test -p anvil-bench devacc_   # unit goldens when present; not a PR gate by default
cargo run -p anvil-bench --release -- devacc --tier A

# Tier B — on-demand only (requires model credentials)
ANVIL_DEVACC_MODEL=… \
ANVIL_DEVACC_N=10 \
  cargo run -p anvil-bench --release -- devacc --tier B --scenario DEVACC-SCN-02

# Aggregate last run into history candidate (manual commit)
pnpm bench:devacc:report --in benchmark-results/devacc-… --out benchmarks/history/devacc/
```

- **Nightly:** opt-in via DEVACC-011 only.
- **PR / CI gate:** opt-in via DEVACC-012 only; off by default.
- Tier A unit goldens may still run as ordinary `cargo test` if a package
  already runs them — that is not the same as a named DEVACC PR gate.

---

## Claims policy

| Claim type                                                            | Allowed evidence                                         |
| --------------------------------------------------------------------- | -------------------------------------------------------- |
| "Identity graph payloads are ~X% smaller than reading impacted files" | GCTX-031 only (payload micro)                            |
| "Agents complete impact questions with ~X% fewer tokens"              | Tier A SCN-01/02 **or** Tier B with n≥10                 |
| "Developer acceleration reduces tokens on real feature work by ~X%"   | Tier B SCN-40 (or composite SCN-10–14) with quality veto |
| "Anvil prevents secret/boundary footguns in the agent loop"           | SCN-30/31 true-positive rate, not token %                |
| Engine latency / RSS                                                  | Kernel + RLB benches — not this suite                    |

Any public number must link **scenario id + arm + tier + date + model/Anvil
SHA**.

---

## Open questions

1. **Headless agent driver:** **Resolved (DEVACC-007).** First-class path is a
   **custom MCP host** protocol (`ANVIL_DEVACC_DRIVER=external` +
   `ANVIL_DEVACC_EXTERNAL_CMD`, writing `external-results.json`). Built-in
   `dry-run` scaffolds schema smoke from Tier A and is not publishable agent
   evidence. Claude Code / Cursor adapters may plug the same external contract
   later without changing the report schema.
2. **Whether Tier A edit scenarios count "ideal tool scripts" as treatment.**
   Ideal scripts measure _ceiling_ savings if the agent uses tools perfectly;
   Tier B measures _achieved_ savings. Both are useful; label them `ceiling` vs
   `achieved`.
3. **Multi-language expansion** beyond TS/Rust.
4. **Interaction with `anvil export` / CLI GCTX secondary surface (ADR-095)** as
   an offline Tier A tool path without MCP.

---

## Validation checklist (for implementers)

- [ ] Tier A goldens fail CI when GCTX payload shape drifts unintentionally
- [ ] Control arm never receives Anvil MCP tools
- [ ] Treatment arms assert graph ready before measurement window
- [ ] Rubric runs offline (tests / JSON schema), not LLM-as-judge alone
- [ ] Report includes caveats section verbatim or by hash
- [ ] No secrets in fixtures or captured traces
- [ ] `full-accel` runs record skill install digest when skill is part of the
      arm
- [ ] SCN-40 cannot publish without secondary metrics and success rate

---

## Docs closeout

| Field               | Value                                                                                                                             |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| **Type**            | Spec (architecture measurement)                                                                                                   |
| **Authority**       | Derived — product surface defined by public Developer Acceleration tutorial + live GCTX/RMCP tools; measurement design lives here |
| **Owner**           | DEVACC (execution); product surface still GCTX/RMCP/skill                                                                         |
| **Status**          | Live (design); DEVACC first wave Ready                                                                                            |
| **Does not change** | Runtime product behaviour; public marketing numbers until DEVACC-010                                                              |
| **Next**            | Run Tier A on demand; live Tier B external driver when credentials available; keep 011/012 opt-in off by default                  |

---

## References

- Public loop definition:
  [`docs/public/anvil/tutorials/developer-acceleration.md`](../public/anvil/tutorials/developer-acceleration.md)
- Graph context user guide:
  [`docs/guides/ai-context-delivery.md`](../guides/ai-context-delivery.md)
- GCTX delivery contract:
  [`docs/architecture/graph-context-delivery-spec.md`](./graph-context-delivery-spec.md)
- Payload micro-bench: `crates/anvil-bench/src/scenarios/token_reduction.rs`
  (GCTX-031)
- Engine benches:
  [`docs/architecture/kernel-benchmarking-spec.md`](./kernel-benchmarking-spec.md)
- Skill behaviour:
  `crates/anvil-cli/assets/skills/anvil-developer-functions/SKILL.md`
- Archived plan-format token experiment:
  `docs/archive/experiments/html-vs-markdown-tokens/`
- Latency budgets: ADR-031
- MCP delivery target: ADR-083
