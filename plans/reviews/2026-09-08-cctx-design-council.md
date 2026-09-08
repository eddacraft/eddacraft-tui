# Context Compiler (CCTX) — design council

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Advisory | CCTX | Draft (design-council decision; not an ADR) | 2026-09-08 — session `council-3120c442` / `plan-3120c442` against `8c48769c2`; corpus `23457dc6d2bf379791d587cf2dfdb5046ce51fc0` |

| Upstream | Downstream |
| -------- | ---------- |
| [Context Compiler spec](../specs/2026-09-08-context-compiler.md), [CCTX module](../modules/context-compiler.aps.md), [CCTX-003 report](../audits/2026-09-08-cctx-003-baseline-comparison.md), [GATT ADR-142](../decisions/142-graph-answer-attestation.md), [Local data and security](../../docs/public/anvil/operations/security.md) | Owner may open a later eval item; this note does not change APS work-item status |

Council **design** mode after the internal eval spike. No product code. This
note is not an ADR and does not unpark spec §15.

**Session:** `council-3120c442` (mode `design`, pack `risk-selected`). Planning
shape: `plan-3120c442`. Local session files live under `.council/sessions/`
(gitignored); this document is the durable publication.

## 1. Decision

| Field | Value |
| ----- | ----- |
| **Decision** | **PASS** |
| **Gate** | **WARN** |
| **Chosen grain** | Live spec §12.7 **V1** sessions (ordinary exploration). Owner may open that as a later APS item. |
| **Not authorised** | Product compiler, knowledge store, crate, feature flag, MCP tool, GCTX DTO change, GATT fork, numeric confidence, off-box synthesis, or any §15 unparking |

**PASS means the frozen eval protocol is fit to run the missing live V1
sessions.** It does **not** mean the product compiler is fit to implement.
CCTX-003 did not complete a live four-variant comparison. A Decision Brief
remains **never** allow / warn / block.

Kernel-maintainer voted **REPLAN** (comparison incomplete; do not read PASS as
“build it”). Pragmatic-lead voted **PASS** on the V1 grain only.
Security-analyst voted **BLOCK** on product unparking. Council-debate
adjudicated the label split as **PASS** for the V1 grain, with security’s
product BLOCK absorbed as a binding invariant of that PASS — not a session
`BLOCK` on eval.

## 2. Diversity actually achieved

| Property | Achieved? | What ran |
| -------- | --------- | -------- |
| Perspective | yes | `kernel-maintainer`, `pragmatic-lead`, `security-analyst` |
| Context independence | yes | Three separate reviewer agents; no shared finding list until synthesis |
| Model diversity | no | Same Grok inherit as the parent run |
| Provider diversity | no | One provider |
| Adversarial-reviewer | no | Not dispatched. The only material contradiction was the **decision label**, sent to `council-debate` |

Anvil developer functions were unavailable in this harness; reviewers read
the pinned files directly.

## 3. Problem and contract

**Problem.** After CCTX-003, is spike evidence enough to recommend a next
implementation grain, and should any spec §15 item be resolved, parked, or
taken to an ADR — without inventing ADRs the owner did not accept?

**Pinned contract**

- Spec:
  [`plans/specs/2026-09-08-context-compiler.md`](../specs/2026-09-08-context-compiler.md)
  (Draft; §9 and §12/§13 frozen enough to run; §15 parked).
- Module: [`plans/modules/context-compiler.aps.md`](../modules/context-compiler.aps.md)
  (Ready; CCTX-001 Merged, CCTX-002 Done, CCTX-003 Complete).
- Spike:
  [`plans/audits/2026-09-08-cctx-003-baseline-comparison.md`](../audits/2026-09-08-cctx-003-baseline-comparison.md).
- **Trust boundary (not parked):** synthesis / Decision Brief is advisory only
  — never allow / warn / block. Enforcement must keep working if Context
  Compiler is unavailable, stale, or wrong.

CCTX-003 actually ran: authored V3/V4 fixtures, mechanical §9 scoring
(20/20 `contract_ok`), corpus-path existence checks, honest blocked/unmeasured
labels. It did **not** run 20 × 4 live agent sessions.

## 4. Chosen approach

Authorise **eval follow-on only**:

1. Run spec §12.7 **V1** sessions: twenty independent ordinary-exploration
   agents against corpus `23457dc6d2bf379791d587cf2dfdb5046ce51fc0`. No
   pre-authored brief. No GCTX required. Record every §13 metric, including
   billed tokens (or explicit `unmeasured`, never zero), task-success, gold
   recall, authority leakage, recovery cost, and drill-down frequency.
2. Keep V2 and V4 **blocked** until GCTX tools are actually in the harness.
   Blocked is not a silent V1 substitute (spec §12.7 step 2).
3. Keep live V3 unmeasured until V1 (and, when unblocked, V2) exist. Do not
   treat fixture `contract_ok` as V3-beats-V2.
4. Keep **all** spec §15 items parked. Do not file ADRs from this council.
5. Do not start a product compiler, store, crate, flag, or MCP tool.

Owner opens any new APS item after accepting this note. Council does not flip
CCTX work-item status.

## 5. Alternatives rejected

| Rejected | Why |
| -------- | --- |
| Build the product compiler / crate / flag / MCP now | Primary thesis unmeasured. Four of six §13 gates unmeasured. Spec still Draft for product architecture. Unanimous across lenses. |
| Treat 20/20 `contract_ok` as evidence briefs work | Authorship verification of human fixtures, not independent-agent recall or success. |
| Treat ~1200–1650 `gctx-simple-v1` tokens as efficiency | Planning-budget estimates from a Python port; crate not executed; no V1/V2 billed baseline. |
| Unpark some §15 items “to unblock product” | No §15 item blocks live V1. Resolving them now would decide without evidence. |
| Run V2/V4 in the same grain as V1 | GCTX was absent in the spike harness. Spec forbids silent V1 substitution. |
| Author more fixtures instead of live sessions | More author-written documents, not more comparison data. |
| Reuse or extend GATT `Attestation` for briefs | ADR-142 is counts-and-flags on GCTX projections and rejects ranking confidence. T19 must stay parked. A GATT-shaped block on synthesis would attest interpretation. |
| Off-box or hosted synthesis | Local-first boundary does not follow private source into a provider. T15 stays visible uncertainty. Snippet-egress consent is not a compiler waiver. |
| Numeric confidence / rank on briefs | Estimate dressed as measurement (the defect ADR-142 exists to prevent). Soft allow. |
| Knowledge store “just for cache” | Second authority; git or cross-workspace leak; agents may treat it as a gate. No store is authorised. |
| Session `BLOCK` on V1 because GCTX is missing | GCTX absence blocks V2/V4 only. V1 does not need GCTX. |
| Session `REPLAN` of the frozen §12.7 protocol | The protocol is already frozen enough to run. What is incomplete is **execution** of live V1, not the eval design. |

## 6. Invariants

These are binding. This PASS does not weaken them.

1. **Trust boundary.** A Decision Brief is advisory context. It is never
   allow / warn / block, never a substitute for `anvil_validate_write`, never
   policy authority. The spec §7 dotted edge is influence, not permission.
2. **Enforcement independence.** Launch validation fail-open / fail-closed is
   unchanged if Context Compiler is missing, stale, or wrong.
   `gateUnavailable` stays a validate-write outcome, not a brief outcome.
3. **No unlabelled synthesis as fact.** Kind split in spec §9.5 stands for
   eval fixtures; it is not a product schema.
4. **Fail visibly.** Missing, stale, conflicting, or partial evidence is
   reported. No quiet stale fallback. Parked §15 items that an agent might
   “decide” stay `Uncertainty` (T15, T19).
5. **Blocked means blocked.** If GCTX is unavailable, V2/V4 are blocked, not
   silent V1.
6. **Parking is not a decision.** Spec §15 stays parked until the owner
   resolves, keeps parked, or files an ADR.
7. **No GATT fork.** Brief evidence must not require, extend, or duplicate
   GATT’s `Attestation` block. Quoting a GCTX payload that already carries
   attestation is reuse, not a CCTX decision.
8. **No numeric confidence** until §15.4 is resolved with an owner ADR.
   Qualitative `unmeasured` / `estimated` / `unknown` only.
9. **Local-first.** Private source does not leave the machine for synthesis.
   Identity-only remains the GCTX default.
10. **Not a sixth Graph Trust Surfaces track.** Affinity, not membership. Not
    a GCTX/GATT/CEG replacement, not a general knowledge product.

## 7. Spec §15 — per item

No item is **resolve-now**. None is an ADR invented by this council.

| # | Item | Disposition | Notes |
| - | ---- | ----------- | ----- |
| 1 | Card granularity | **keep-parked** | Corpus scores file/crate recall. That is not a card model. |
| 2 | Canonical versus advisory inputs | **keep-parked** | §12.3 is a spike allowlist. Policy text stays evidence to read, not EVALCI. Do not freeze §9.5 “policy as fact” into product. |
| 3 | Task intent representation | **keep-parked** | §12.6 freezes eval prompts, not a task-router architecture. |
| 4 | Confidence model | **keep-parked; needs-ADR** before any product score | Fixtures correctly use qualitative `unmeasured`. No ranks, margins, or GATT-shaped confidence. |
| 5 | Conflict surfacing | **keep-parked** | Fail visibly; autonomous resolution stays out of scope. How it is shown can wait. T06/T07/T20 `partial` is fixture honesty, not a product mechanism. |
| 6 | Evidence format versus GATT | **keep-parked; needs-ADR** before any product evidence type | Distinct advisory contract. Do not reuse or extend `Attestation`. T19 remains parked. |
| 7 | Synthesis models under local-first | **keep-parked; needs-ADR** before any model path | No synthesis ran (human fixtures). T15 stays uncertainty. No off-box private source. |
| 8 | Knowledge location | **keep-parked; needs-ADR** before any store | No store authorised. Any later store must stay advisory, workspace-scoped, and must not become a gate. |

## 8. Risks and required validation before any **build**

“Build” here means a product compiler, store, crate, flag, or MCP tool — not
live V1 eval.

| Risk | Required before any product build |
| ---- | --------------------------------- |
| Comparison vacuum becomes a launch narrative | Actual billed-token V1 baselines (and V2 when GCTX exists). No “brief reduces tokens” claim from fixture sizes. |
| Fluent brief as allow (T13) | Live T13: agent still calls `anvil_validate_write`; leakage scorer fails the run if a brief or GCTX answer is cited as allow / warn / block. Fixture labels do not replace that run. |
| False completeness / shared hallucination | Live scoring of stale or incorrect claims against the corpus revision; visible `partial` / conflict. |
| Cost displacement | Refresh and inference cost counted; recovery cost instrumented. One-shot fixture authoring is not amortisation. |
| GATT-shaped attestation of interpretation | §15.6 still parked; no Attestation on synthesis. |
| Estimated confidence read as measurement | No numeric confidence on briefs. |
| Off-box exfil | If any synthesis is even prototyped: prove source stayed on-box. `anvil gctx egress enable` is not a waiver. |
| Store as second authority | No store in the next grain. |
| V2/V4 silent V1 | Harness records blocked when GCTX is absent. |
| T06/T07 V3 handle gaps | Before live V3: add missing handles or document those tasks as structural partial-brief cases. |
| Estimator fidelity | Before any numeric §13 threshold: cross-check the Rust `estimate_gctx_tokens` path against the Python port. |

Live V1 itself still needs: independent sessions (no memory reuse across
variants), corpus checkout not a dirty feature branch, provider totals or
honest `unmeasured`, and zero authority leakage on whatever is actually run.

## 9. Landing strategy (this PASS)

1. Land **this note** on a docs PR against `main`. Do not merge from Council.
2. Do **not** add a CCTX-004 work item in this change. Owner may open it after
   accepting the grain: live §12.7 V1, twenty tasks, billed tokens, §13
   metrics, no product surface.
3. Treat GCTX-in-harness as a **separate** unblocker for V2/V4, not as a
   substitute for V1.
4. After V1 data exists, reconvene before any product-shaped item. §15 stays
   parked through that point.
5. Rollback of the eval grain is deletion of run records. No user-facing
   surface is exposed.

## 10. Judge synthesis

**Target judged:** post-spike CCTX design (spec + Ready module + CCTX-003),
HEAD `8c48769c2`.

**Corroborated (all three lenses):** do not unpark §15; do not authorise a
compiler; fixtures are honest; trust boundary remains binding; next cheap
loop is live V1.

**Label split:** REPLAN vs PASS vs BLOCK. Debate verdict **B** (pragmatic-lead
PASS on the V1 grain). Security’s product BLOCK is a `must_fix` **invariant**
of that PASS, not a session `BLOCK` on eval. Kernel-maintainer’s REPLAN is
accepted as a warning against misreading PASS as a compiler licence — hence
gate **WARN**.

**must_fix (invariants on this PASS, already encoded above):** do not unpark
product architecture; do not claim §11 live acceptance from fixtures; do not
fork GATT; do not mint numeric confidence; do not send private source
off-box.

**should_fix (next eval grain, non-blocking here):** live V1 sessions with
billed tokens; do not quote 20/20 as agent recall; score live T13; instrument
recovery cost.

**consider:** Rust vs Python token-estimator cross-check before thresholds;
T06/T07 V3 handle adequacy before live V3.

**later_items:** owner APS item for live V1; GCTX harness unblocker for V2/V4;
owner ADRs for §15.4/6/7/8 if and when a product grain is in scope.

**root_causes:** CCTX-003 completed an internal **fixture** eval, not a live
variant comparison. That is correctly labelled in the report; the residual
risk is treating those labels as a product licence.

## 11. Change-impact (docs)

| Concern | Disposition |
| ------- | ----------- |
| New documentation unit `plans/reviews/2026-09-08-cctx-design-council.md` | Advisory council decision note; linked from the spec residual map, CCTX module, CCTX-003 residual, and index Graph Substrate row. Not a DOCRB inventory component. |
| Public contracts / GCTX / GATT / CEG / enforcement | Unaffected — no product behaviour change |
| Diagrams | Unaffected — spec §7 topology and trust boundary unchanged |
| ADRs | Unaffected — no ADR filed; §15 remains parked |
| APS work-item status | Unaffected — CCTX-001/002/003 statuses unchanged; no new item added |

## 12. Final decision

**PASS** (gate **WARN**).

Fit to implement **live §12.7 V1 eval sessions** only. Not fit to implement a
Context Compiler product. Spec §15 stays parked. Trust boundary stays
binding.
