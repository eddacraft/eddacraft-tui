# Pre-Tag Release Review Doctrine + Canonical Council Template — Design

**Module:** RELORCH (Release Orchestration)
**Status:** Accepted (operator, 2026-07-02) — D3 = clawpatch (live tool); tax-fix
approach green-lit. Implemented by `docs/runbooks/release-process.md` + the
`plans/reviews/release-council/TEMPLATE*.md` templates in this change.
**Date:** 2026-07-02

## Overview

This design proposes reinstating the **pre-tag review doctrine** (adversarial
sweep → multi-persona council → human gate → durable artefact) into the
**authoritative** release process, and providing a **canonical template** so each
tag's council artefact is filled from a template rather than copied from the
prior tag. It exists so the operator can sign off the whole shape as one piece of
work before any of it is built.

It resolves GitHub issue [#1872](https://github.com/eddacraft/anvil-001/issues/1872)
(canonical template, Josh's request) and the **review-gate portion** of
[#1712](https://github.com/eddacraft/anvil-001/issues/1712) (promote the
v0.7.0-beta runbook doctrine to a general template). The non-review doctrine in
#1712 (§13 APS reconciliation, §14 release-record fill-in, §15 CHANGELOG bump)
is related but out of this design's core — see [Non-Goals](#non-goals).

## Problem

The pre-tag review doctrine is **dormant**:

- The adversarial sweep (§1 clawpatch) and multi-persona **council** (§2) steps —
  which write `plans/reviews/release-council/<date>-<tag>-pre-tag.md` — live
  **only** in the version-specific `docs/runbooks/v0.7.0-beta-release-runbook.md`.
- The **authoritative, live** `docs/runbooks/release-runbook.md` has a Happy Path
  of steps 1–9 (Start → Preflight → Prepare → PR → Tag → Monitor → Verify →
  Comms → Close Out) with **no adversarial-sweep step and no council step**.
- Consequently **no council artefact exists past v0.7.1-beta** (2026-05-22):
  v0.7.2 through v0.8.1 all shipped without one.
- The two existing artefacts (v0.7.0, v0.7.1) were authored by **copying** the
  prior tag's file — the exact error-prone practice #1872 flags (stale
  references, wrong carry-forward lists, drifting SHA/tag/diff strings).

So #1872's premise ("we copy the prior council artefact, give us a template")
describes a workflow the current release process no longer runs, and #1712 (still
open) owns re-establishing the review steps in the general process. Building a
template in isolation would produce an orphan for a dormant process — hence this
design settles the whole shape first.

## Goals

1. Make the pre-tag review doctrine a **first-class, tag-agnostic** part of the
   authoritative release process (not stranded in a version-specific runbook).
2. Bind the doctrine to the **current** review tooling, not the retired
   clawpatch command block verbatim (see [D3](#d3-adversarial-sweep-mechanism)).
3. Give each tag a **canonical template** to fill (input brief + output
   artefact), so no artefact is ever copied from the prior tag again.
4. Keep the **human gate**: the release owner signs the synthesised verdict
   before `tag.sh` runs (the load-bearing property of the `release` tier).
5. **Remove the re-cut restart tax** — the actual reason the process lapsed (see
   below). The council found genuinely useful issues, but every fix moved
   `main`'s SHA, invalidated the candidate, and forced a **full sweep + council
   from scratch**. The doctrine is only worth reinstating if that tax is gone.

## Why it lapsed (operator, 2026-07-02)

> "The pre-tag council found some very useful issues in the past, but the
> problem is that regardless of whether it finds anything it changes the main
> SHA and we start again… so it became painful. But I believe it is missing."

The value is not in question — the **re-cut-restart loop** is. The v0.7.0
doctrine treated "fix → re-cut candidate → return to §1" as "normal and
expected", which in practice meant: any council finding (even a trivial one)
landed a fix on `main`, moved the SHA, and restarted the whole ~12-minute sweep
plus the full council over the entire release window. That is the pain this
design must eliminate — not by dropping the council, but by decoupling it from
`main`'s motion and reviewing only the delta on a re-pass.

## Non-Goals

- The non-review doctrine promotion in #1712 (§13 APS reconcile, §14
  release-record fill-in, §15 CHANGELOG bump). Related; tracked by #1712; can be
  done in the same canonical doc later but is not designed here.
- Changing the council **reviewer set** or the judge/debate mechanics.
- Changing the release-record schema (`plans/specs/2026-05-10-release-record-schema.md`).
- Auto-blocking behaviour beyond what exists (warnings-over-blocks posture holds).

## Current-state evidence

| Fact | Source |
| ---- | ------ |
| §1 clawpatch + §2 council doctrine | `docs/runbooks/v0.7.0-beta-release-runbook.md` §1–§2 |
| General runbook has no review step | `docs/runbooks/release-runbook.md` (Happy Path 1–9) |
| No artefacts past v0.7.1-beta | `plans/reviews/release-council/` (only 2026-05-20…22 files) |
| "Need a canonical template not a copy" | v0.7.1 council artefact, Human-gate Notes (Josh, 2026-05-22) |
| #1712 owns doctrine promotion | issue #1712 (open), Option A recommended |

## Proposed design — "the whole lot"

### 1. Doctrine home

Promote the pre-tag review doctrine into a **single canonical document**,
`docs/runbooks/release-process.md` (#1712 Option A), and have the authoritative
`release-runbook.md` reference it as a required pre-tag gate (a new step between
Preflight and Tag). Version-specific runbooks, if any, link up to it and add only
the gates unique to that tag.

Rationale: lowest-risk; the doctrine becomes one referenced document instead of
being re-copied per tag. Promote to a driveable **skill** (#1712 Option B) only
if a future tag's runbook collapses to "see release-process.md plus N additions".

### 2. The two pre-tag review gates (promoted, tag-agnostic)

- **Gate A — adversarial sweep (`clawpatch`).** The operator's `claw-sweep` over
  the release window, output committed as durable JSON under `plans/audits/`.
  Critical findings auto-block; highs are enumerated for the council to triage.
  This is the live tool, carried forward as-is from the v0.7.0 §1 doctrine (see
  [D3](#d3-adversarial-sweep-mechanism)).
- **Gate B — council review.** A multi-persona `release`-tier council over the
  union of (a) Gate A's findings and (b) the full release diff
  `<prior-tag>..<candidate-sha>`. Judge produces a ship / fix / defer verdict per
  finding plus an overall release-diff verdict; the **human gate** signs off
  before the tag is cut. Output committed under
  `plans/reviews/release-council/<date>-<tag>-pre-tag.md` — **filled from the
  template** (§3), not copied.
- **Re-passes are delta-only, not restarts** — see the next section. A fix does
  **not** trigger a full sweep + full council over the whole window.

### 2.5 Removing the re-cut restart tax (the core of this design)

The tax was: fix on `main` → SHA moves → candidate stale → full sweep + full
council again. Four changes — all using mechanisms the repo **already has** —
remove it:

1. **Freeze the candidate on a stabilisation branch.** Use the runbook's existing
   `--strategy stabilisation` path: cut `release/vX.Y.Z` at the candidate SHA and
   run the council against the **frozen branch**. Must-fix fixes land on the
   branch (PR into `release/*`), **not** on `main`. `main` keeps moving
   independently — the candidate no longer fights trunk. The candidate SHA
   changes *only* when a hardening commit is deliberately applied to the branch.
   *(This is why the "direct vs stabilisation" strategy already exists in
   `release-runbook.md` step 4 — this design makes stabilisation the path used
   whenever the council has any must-fix.)*

2. **Delta-only re-review.** When a hardening commit lands on the branch, review
   only `<prior-candidate>..<new-candidate>` (the fix) **plus** re-verify the one
   finding it closes. Prior verdicts **carry forward** — the artefact already has
   a "Carry-forward verdicts" section; this design makes it *intra-tag* carry
   forward, formalised via an `anvil capsule` (ADR-074) snapshot of the prior
   pass so verdicts reattach instead of being re-derived. No full re-sweep, no
   full-window council on a re-pass.

3. **Defer-don't-block by default.** Only **critical / must-fix** findings force a
   hardening commit + delta re-review. Major/minor findings are **filed as
   issues and do not block the tag** (the warnings-over-blocks posture, ADR-002).
   This is the biggest tax reducer: most findings become follow-up issues and
   trigger **zero** re-cuts. The v0.7.1-beta pass already did this (its D1–D4
   were "filed, not blocking") — the design makes it the default, not an
   ad-hoc call.

4. **Streaming Council upstream shrinks the batch.** The `council` skill's
   **Streaming Council** runs during implementation; **Batch Council** at
   milestones. With streaming review already applied across the window, the
   pre-tag pass is a **confirmation** over largely-reviewed code, not a
   discovery-from-scratch sweep — far fewer surprise must-fix findings at tag
   time, so far fewer re-cuts.

Net effect: a re-cut happens *only* for a genuine blocker, touches *only* the
frozen branch (not trunk), and re-reviews *only* the fix. The "start again over
the whole window every time" loop is gone.

### 3. Canonical templates (#1872)

Add **two** templates under `plans/reviews/release-council/`, since a pass
produces two artefacts:

- `TEMPLATE-input.md` — the council **input brief** (claim, claim-gates, freight,
  hard-gate evidence, operator observations, council questions, references).
- `TEMPLATE.md` — the council **output artefact** (reviewer-outputs table,
  CHANGELOG excerpt, judge synthesis, must-fix actions, deferred actions,
  carry-forward verdicts, tag-cut sequence, **human-gate sign-off** block,
  reviewer one-liners appendix).

Both use `{{placeholders}}` for every value that drifts when copying:
`{{tag}}`, `{{date}}`, `{{candidate_sha}}`, `{{candidate_sha_short}}`,
`{{prior_tag}}`, `{{diff_scope}}` (= `{{prior_tag}}..{{candidate_sha}}`),
`{{prior_council_session}}`, `{{prior_council_file}}`. Updating a template is how
changes propagate to every future tag. **Each tag copies the template, never the
prior artefact.** (This is #1872 Option 1 — the cheap, high-value option.)

A `scripts/release/scaffold-council.mjs` that auto-populates the placeholders
from `git describe`/HEAD/next-version (#1872 Option 2) is **deferred** — added
only if hand-filling the template proves to be the remaining friction.

### 4. Operator flow (composed)

```
release-runbook.md → "Pre-tag review" step →
  1. run Gate A sweep            → plans/audits/<date>-sweep-<tag>.json
  2. cp TEMPLATE-input.md  <date>-<tag>-pre-tag-input.md ; fill {{…}}
  3. cp TEMPLATE.md        <date>-<tag>-pre-tag.md
  4. run Gate B council (release tier) → fills the output artefact
  5. HUMAN GATE sign-off recorded in the artefact
  6. reference both artefacts from the release record → proceed to Tag
```

## Decisions to sign off

### D1 — Doctrine home
**Options:** (A) canonical `docs/runbooks/release-process.md` referenced by the
runbook · (B) a `.claude/skills/release-process/` skill.
**Recommendation: A** now (align with #1712), B later if warranted.

### D2 — Template vs scaffold
**Options:** (1) hand-edited `TEMPLATE.md` + `TEMPLATE-input.md` · (2)
`scaffold-council.mjs` auto-populator.
**Recommendation: 1 now** (delivers Josh's ask cheaply); defer 2.

### D3 — Adversarial-sweep mechanism
**Gate A is `clawpatch`** — the operator still runs it (`claw-sweep` / `claw-map`
per the v0.7.0 doctrine); it is the live adversarial-sweep tool, not retired.
(Only the *per-release finding tracker* `plans/archive/modules/clawpatch-pre-tag-v0.7.0-beta.aps.md`
was archived, via CIB-039, once that release's CLAWP-001…065 findings were all
dispositioned — normal lifecycle for a finding tracker, unrelated to the tool.)
**Recommendation:** keep Gate A = clawpatch, exactly as the v0.7.0 §1 doctrine
described it (full `claw-sweep`, JSON report committed under `plans/audits/`,
critical → block, high → council triage). **Open question (optional):** whether to
*also* run the newer `code-review` RELEASE tier / `/code-review ultra` as a
complementary lens alongside clawpatch, or keep Gate A clawpatch-only. Default:
clawpatch-only, matching current practice — add the second lens only if the
operator wants it.

### D4 — Which tags require the gate
Once §2.5 removes the re-cut tax, "which tags" stops being a cost question —
a confirmation pass over a Streaming-reviewed window with defer-don't-block is
cheap enough to run everywhere. **Open question (softened):** run the pre-tag
council for **every** tag (full for `vX.Y.0`, focused for patches), or only
`vX.Y.0`?
**Recommendation:** **every** tag — full-tier for minor/significant (`vX.Y.0`),
a documented **focused** variant (smaller reviewer set, claim-scoped input
brief, as v0.7.1-beta did) for patch tags. The template carries a "tier / focus
rationale" field so the same artefact serves both. With the tax gone, the reason
to skip patch tags (cost) no longer applies.

## Work breakdown (post-sign-off, the "separate piece of work")

1. **RELORCH-A** — author `docs/runbooks/release-process.md` §"Pre-tag review"
   (Gates A+B, pass criteria, human gate), bound to the D3 mechanism. Reference
   it as a required step from `release-runbook.md`.
2. **RELORCH-B (the tax fix — highest value)** — document the §2.5 re-pass model
   as the **default**: stabilisation-branch candidate freeze, delta-only
   re-review with `anvil capsule` carry-forward of prior verdicts, and
   defer-don't-block triage. This is what makes the reinstated council survivable;
   without it, RELORCH-A just re-imports the old pain.
3. **RELORCH-C** — add `TEMPLATE.md` + `TEMPLATE-input.md` (#1872) with the
   placeholder set + a "tier / focus rationale" field (full vs focused, D4); a
   "how to use" header; a fixture-free example row so `docs:check` passes
   (the templates live under `plans/`, which is excluded from oxfmt).
4. **RELORCH-D** — retire the version-specific §1/§2 in
   `v0.7.0-beta-release-runbook.md` down to a link-up to the canonical doc
   (keep it as historical evidence; don't delete).
5. **RELORCH-E** (optional, deferred) — `scaffold-council.mjs` (#1872 Option 2).
6. Close #1872 (delivered by RELORCH-C) and the review-gate half of #1712
   (delivered by RELORCH-A/B/D); note the remaining #1712 doctrine (§13/§14/§15)
   as its own follow-up.

## Risks & open questions

- **D3/D4 are genuine forks** the owner must settle — the recommendations above
  are proposals, not settled facts.
- ~~Orphan risk if the process stays dormant~~ **Resolved (operator,
  2026-07-02): the council is wanted** — it found genuinely useful issues. The
  question was never its value but the re-cut restart tax, which §2.5 removes.
  Reinstating it (not closing #1872/#1712 as won't-do) is the agreed direction.
- **The tax fix must actually hold:** if a re-pass ever silently degrades back
  to a full-window re-sweep (e.g. an operator re-runs the whole council out of
  caution), the pain returns. The delta-only re-review + carry-forward capsule
  must be the *documented default*, and the runbook must state it explicitly.
- **Correction (2026-07-02):** an earlier draft wrongly implied clawpatch might
  be retired — it conflated the archived *v0.7.0-beta finding tracker* (CIB-039)
  with the tool. clawpatch is live and in use; Gate A stays clawpatch (D3).
- **plans/ formatting:** these artefacts live under `plans/` (excluded from
  oxfmt); the templates must still pass `docs:check`/APS lint if placed where
  those gates run.

## References

- Issues: [#1872](https://github.com/eddacraft/anvil-001/issues/1872) (template),
  [#1712](https://github.com/eddacraft/anvil-001/issues/1712) (doctrine promotion)
- Doctrine source: `docs/runbooks/v0.7.0-beta-release-runbook.md` §1–§2
- Artefact exemplars: `plans/reviews/release-council/2026-05-22-v0.7.1-beta-pre-tag{,-input}.md`
- Authoritative runbook: `docs/runbooks/release-runbook.md`
- Release-record schema (unchanged): `plans/specs/2026-05-10-release-record-schema.md`
