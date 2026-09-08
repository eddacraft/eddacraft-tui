# CCTX-003: baselines versus brief variants

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Advisory | CCTX | Draft (internal eval report) | 2026-09-08 — scored against corpus `23457dc6d2bf379791d587cf2dfdb5046ce51fc0`; GCTX tools unavailable in this harness |

| Upstream | Downstream |
| -------- | ---------- |
| [Context Compiler spec](../specs/2026-09-08-context-compiler.md) §9 / §12 / §13, [V3/V4 fixtures](../evals/context-compiler/2026-09-08/README.md) | [CCTX module](../modules/context-compiler.aps.md), [index Graph Substrate row](../index.aps.md#graph-substrate) |

Internal evaluation only. **No product ships.** This report does not authorise
a compiler, knowledge store, crate, feature flag, GATT fork, or enforcement
change. A Decision Brief is **never** allow / warn / block.

## 1. What was run

| Field | Value |
| ----- | ----- |
| Protocol | Spec §12, frozen 2026-09-08 (CCTX-001) |
| Brief contract | Spec §9, frozen 2026-09-08 (CCTX-002) |
| Corpus revision | `23457dc6d2bf379791d587cf2dfdb5046ce51fc0` (throwaway worktree `.worktrees/cctx-003-corpus`) |
| Feature branch | `cursor/cctx-003-eval-comparison-7269` at later protocol-docs commits; eval checkout is the corpus SHA, not this branch |
| Harness | Cursor Cloud Agent; grok adapter choreography (shell identity: `CLAUDECODE`, `CODEX_THREAD_ID`, `OPENCODE`, `GROK_AGENT` all empty; `CURSOR_AGENT` set) |
| Model | cursor-grok-4.6-high-fast |
| GCTX egress consent | off (default) |
| GCTX tools available | **no** (`anvil` CLI absent; no GCTX MCP tools in this harness) |
| Claim | private GitHub issue #4461 (degraded git-ref `refs/claims/CCTX-003`) |

Spec §12.7 requires one session per `(task, variant)` pair against the corpus
revision. This run **did not** execute 20 × 4 live agent sessions. It did:

1. Author one V3/V4 Decision Brief fixture per corpus task (T01–T20) against
   frozen §9.
2. Check V3 drill-down paths exist at the corpus revision.
3. Mechanically score fixtures for envelope, claim kinds, evidence handles,
   gold-path mention, T13/T15/T19/T20 honest outcomes, and authority labelling.
4. Probe GCTX availability (absent → V2 and V4 **blocked**, not silent V1).

Machine record:
[`plans/evals/context-compiler/2026-09-08/runs/2026-09-08-fixture-score.json`](../evals/context-compiler/2026-09-08/runs/2026-09-08-fixture-score.json).

## 2. Measured versus unmeasured versus blocked

| Variant | Status | What that means |
| ------- | ------ | --------------- |
| **V1** Ordinary exploration | **unmeasured** | No independent file-search agent session per task. Corpus tree is readable; that is not a §12.7 session. |
| **V2** GCTX-assisted | **blocked** | GCTX tools unavailable. Spec §12.7 step 2: blocked, not a silent V1 substitute. |
| **V3** Brief only | **measured as fixture artefacts; live sessions unmeasured** | Fixtures exist and were scored. Independent brief-only agents were not dispatched. |
| **V4** Brief + GCTX drill-down | **blocked** for live GCTX calls; **same fixtures as V3** | Drill-down handles are listed; they were not executed. |

Do not read "fixtures exist" as "V3 beat V2". There is no V2 baseline session
to compare against.

## 3. Fixtures

Twenty briefs live under
[`plans/evals/context-compiler/2026-09-08/fixtures/`](../evals/context-compiler/2026-09-08/fixtures/).
One file per task is shared by V3 (cited paths only) and V4 (plus GCTX handles),
matching spec §9.7 / §12.4.

Mechanical score (2026-09-08): **20/20 `contract_ok`**. Every required §9.3
section is present. Every parsed claim has a kind in the §9.5 split and at
least one evidence handle. Every brief has an `Authority` section forbidding
allow / warn / block. Confidence is qualitative `unmeasured` (spec §15.4
parked — no numeric scores). T06, T07, and T20 mark invalidation `partial`
because gold paths sit outside the §12.3 allowlist; that is visible, not hidden.

### Gold-path recall *in the fixture* (not a live agent)

| Result | Tasks |
| ------ | ----- |
| mentioned | T01–T20 (scorer gold needles all present) |
| not mentioned | none |
| contradicted | none observed in scoring |

Recall here is "did the brief mention the gold keys", which is the V3 artefact
property. It is **not** V1/V2 agent recall.

### Honest outcomes encoded in fixtures

| Task | Required honest result | Fixture |
| ---- | ---------------------- | ------- |
| T13 | Keep `anvil_validate_write`; never skip because a brief looks safe | Recommendation "No. Do not skip…" |
| T15 | Visible uncertainty; do not invent an ADR (§15.7) | `Uncertainty` claim; forbids inventing an ADR |
| T19 | Parked (§15.6); do not fork GATT | `Uncertainty` claim; parked; no GATT fork |
| T20 | Not a sixth Graph Trust Surfaces track | Answers **no** — affinity, not membership |

### Brief size (GCTX-020 planning budget)

Estimator: `gctx-simple-v1` Python port of
`crates/anvil-graph-cache/src/tokens.rs` (`estimate_gctx_tokens`). Planning
budget, **not billed tokens**. The crate was not executed in this harness
(`anvil` / a targeted `cargo test` pin was not completed); treat counts as
upper-leaning estimates.

| Task | gctx-simple-v1 tokens | Task | tokens |
| ---- | --------------------: | ---- | -----: |
| T01 | 1377 | T11 | 1328 |
| T02 | 1333 | T12 | 1435 |
| T03 | 1424 | T13 | 1254 |
| T04 | 1482 | T14 | 1197 |
| T05 | 1278 | T15 | 1285 |
| T06 | 1364 | T16 | 1350 |
| T07 | 1354 | T17 | 1204 |
| T08 | 1647 | T18 | 1426 |
| T09 | 1352 | T19 | 1276 |
| T10 | 1239 | T20 | 1277 |

Synthesis / refresh amortised cost: fixture authoring once (this report). No
knowledge store, so no refresh loop was run.

## 4. §13 metrics (this run)

| Metric | Result |
| ------ | ------ |
| Billed tokens | **unmeasured** (harness did not expose per-task provider totals; not recorded as zero) |
| Synthesis and refresh amortised cost | V1/V2: zero synthesis (and V1 unmeasured / V2 blocked). V3/V4: one-shot fixture authoring; refresh **unmeasured** (no store) |
| Tool calls | Fixture scoring used file reads of briefs + corpus path existence. Live GCTX tool calls: **none** (blocked). `anvil_validate_write`: **not invoked** (no product edit) |
| Time to first useful action | **unmeasured** (`none` for live sessions) |
| Latency | Fixture score wall time was seconds; live session latency **unmeasured** |
| Task success | Fixture honest-outcomes **passed** for T13/T15/T19/T20. Live V1/V2/V3/V4 success **unmeasured** / V2+V4 **blocked** |
| Correct-file / symbol recall | Fixtures: gold needles mentioned (table above). Live agents: **unmeasured** |
| Missed blast-radius | **unmeasured** for live T08–T11 and T16–T18 sessions. Fixtures name the gold crates/tests for those tasks |
| Stale or incorrect claims | No hidden stale use on the fixture run (section 5). Live sessions not run |
| Drill-down frequency | **unmeasured** (no V3 live session). V3 handles are listed so a future run can count opens |
| Recovery cost | **unmeasured** |
| Brief size against budget | Measured as table above. No numeric budget threshold is set (CCTX-001) |

No numeric thresholds are proposed. Spec §13 allows this report to propose them
after baselines exist; **V1/V2 baselines do not exist yet**, so thresholds stay
unset.

## 5. Zero hidden stale use (what was actually run)

| Check | Result |
| ----- | ------ |
| Corpus worktree HEAD | `23457dc6d2bf379791d587cf2dfdb5046ce51fc0` |
| Every §12.3 selected input exists at corpus | yes (pre-authoring probe) |
| Every V3 drill-down path exists at corpus | yes (scorer `v3_handles_missing_at_corpus` empty) |
| Every fixture pins source revision to that SHA | yes |
| Quiet stale fallback | none — T06/T07/T20 declare `partial` where gold paths are not selected inputs |
| Claims checked against corpus text (spot) | `ImpactSummary.truncated` still a single bool; `heuristic` still OR-across-edges; `.markdownlintignore` still excludes `plans/**`; ADR-125 CIB-398 nested-root rule still present; CE-1 identity-only default test still present |

**Gate: passed** for the fixture + corpus existence run. Not a claim about
unrun V1/V2 sessions.

## 6. §13 initial gates

Each gate is passed, failed, or unmeasured. None is a silent skip.

| Gate | Verdict | Why |
| ---- | ------- | --- |
| Material reduction versus GCTX alone (V3/V4 vs V2 on tokens and tool calls) | **unmeasured** | V2 blocked; no live V3/V4 sessions; fixture sizes cannot be compared to an unrun V2 |
| No material regression in success or recall versus V2 | **unmeasured** | No V2 success/recall baseline |
| Zero hidden stale use on whatever was actually run | **passed** | Section 5 |
| Every material claim evidence-linked (V3/V4) | **passed** (fixtures) | 20/20 briefs; all parsed claims have evidence handles. Live V3 sessions unmeasured |
| Refresh cost amortisable (V3/V4 only) | **unmeasured** | No store, no refresh loop. One-shot authoring is not an amortisation proof |
| Zero authority leakage on every run | **passed** (fixtures); **unmeasured** (unrun live sessions) | Fixtures forbid allow / warn / block; T13 keeps `anvil_validate_write`. No live run treated a brief or GCTX answer as a gate. V2/V4 blocked so they did not leak by substitution |

## 7. Residual map (post-spike)

CCTX-003 completes the authorised Ready spike **as an internal eval**. It does
**not** start product work.

**Still parked (spec §15) — not unparked, not ADRs:**

1. Card granularity
2. Canonical versus advisory inputs
3. Task intent representation
4. Confidence model
5. Conflict surfacing
6. **Evidence format versus GATT** (still parked; fixtures do not require,
   extend, or fork GATT's `Attestation` block)
7. Synthesis models under local-first constraints
8. Knowledge location

**Follow-on product work not started:** compiler, knowledge store, feature
flag, MCP tool, GCTX DTO change, GATT implementation, EVALCI, Graph Trust
Surfaces membership, replacement of GCTX-031 `token_reduction`.

**Eval follow-on (not this PR):** live §12.7 V1 sessions; V2/V4 when GCTX is
available in the harness; then, and only then, any numeric §13 thresholds.

Trust boundary remains binding: synthesis / Decision Brief is advisory only,
never allow / warn / block.

## 8. Change-impact (docs)

| Concern | Disposition |
| ------- | ----------- |
| New documentation unit `plans/evals/context-compiler/` | Eval artefacts; advisory; linked from this report, the spec, and the CCTX module. Not a component in the DOCRB inventory |
| Public contracts / GCTX / GATT / CEG / enforcement | Unaffected — no product behaviour change |
| Diagrams | Unaffected — no topology or trust-boundary diagram change |
| Graph Trust Surfaces shortlist | Unaffected — T20 restates affinity, not membership |
