# Pre-Tag Release Review Doctrine + Canonical Council Template — Design

**Module:** RELORCH (Release Orchestration)
**Status:** Draft — **for owner sign-off** (folds #1872 with the review-gate half of #1712)
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
   clawpatch command block verbatim (see [D3](#d3--adversarial-sweep-mechanism)).
3. Give each tag a **canonical template** to fill (input brief + output
   artefact), so no artefact is ever copied from the prior tag again.
4. Keep the **human gate**: the release owner signs the synthesised verdict
   before `tag.sh` runs (the load-bearing property of the `release` tier).

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
`release-runbook.md` reference it as a required pre-Tag gate (a new step between
Preflight and Tag). Version-specific runbooks, if any, link up to it and add only
the gates unique to that tag.

Rationale: lowest-risk; the doctrine becomes one referenced document instead of
being re-copied per tag. Promote to a driveable **skill** (#1712 Option B) only
if a future tag's runbook collapses to "see release-process.md plus N additions".

### 2. The two pre-tag review gates (promoted, tag-agnostic)

- **Gate A — adversarial sweep.** A whole-repo adversarial review of the release
  window, output committed as durable JSON under `plans/audits/`. Critical
  findings auto-block; highs are enumerated for the council to triage. Bound to
  the **current** sweep mechanism, not the v0.7.0 clawpatch commands verbatim
  (see [D3](#d3--adversarial-sweep-mechanism)).
- **Gate B — council review.** A multi-persona `release`-tier council over the
  union of (a) Gate A's findings and (b) the full release diff
  `<prior-tag>..<candidate-sha>`. Judge produces a ship / fix / defer verdict per
  finding plus an overall release-diff verdict; the **human gate** signs off
  before the tag is cut. Output committed under
  `plans/reviews/release-council/<date>-<tag>-pre-tag.md` — **filled from the
  template** (§3), not copied.
- **Multi-pass loop** (unchanged doctrine): a fix returns to Gate A on a re-cut
  candidate SHA; multiple passes are normal and expected.

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
The v0.7.0 doctrine ran `clawpatch` (`claw-sweep`) via the release engineer's
shell aliases. The clawpatch **finding tracker** was archived (CIB-039); it is
unclear the `clawpatch` **tool** is still the intended pre-tag sweep vs the newer
`code-review` skill **RELEASE tier** / `/code-review ultra` (multi-agent cloud
review). **Open question for sign-off:** which mechanism is Gate A?
**Recommendation:** bind Gate A to the **`code-review` RELEASE tier** (current,
maintained, produces a reviewable artefact) and treat clawpatch as an optional
additional sweep only if still run. This avoids resurrecting a possibly-retired
command block.

### D4 — Which tags require the gate
v0.7.2–v0.8.1 shipped without a council — possibly deliberate for patch tags.
**Open question:** is the pre-tag council required for **every** tag, or only
**minor/significant** tags (`vX.Y.0`), with patch tags (`vX.Y.Z`, Z>0) doing a
lighter focused pass (as v0.7.1-beta itself did — a "focused, not full-tier"
input brief)?
**Recommendation:** required for minor/significant tags; a documented **focused**
variant (smaller reviewer set, claim-scoped input brief) for patch tags — the
template already supports this via a "tier / focus rationale" field.

## Work breakdown (post-sign-off, the "separate piece of work")

1. **RELORCH-A** — author `docs/runbooks/release-process.md` §"Pre-tag review"
   (Gates A+B, pass criteria, multi-pass loop, human gate), bound to the D3
   mechanism. Reference it as a required step from `release-runbook.md`.
2. **RELORCH-B** — add `TEMPLATE.md` + `TEMPLATE-input.md` (#1872) with the
   placeholder set; a one-paragraph "how to use" header; a fixture-free example
   row so `docs:check`/oxfmt pass.
3. **RELORCH-C** — retire the version-specific §1/§2 in
   `v0.7.0-beta-release-runbook.md` down to a link-up to the canonical doc
   (keep it as historical evidence; don't delete).
4. **RELORCH-D** (optional, deferred) — `scaffold-council.mjs` (#1872 Option 2).
5. Close #1872 (delivered by RELORCH-B) and the review-gate half of #1712
   (delivered by RELORCH-A/C); note the remaining #1712 doctrine (§13/§14/§15) as
   its own follow-up.

## Risks & open questions

- **D3/D4 are genuine forks** the owner must settle — the recommendations above
  are proposals, not settled facts.
- **Orphan risk if the process stays dormant:** if the owner does *not* want a
  pre-tag council reinstated at all, the correct outcome is to **close #1872 and
  #1712 as won't-do** and drop this design — the template only earns its keep if
  the gate is real.
- **Tooling drift:** binding Gate A to `code-review` RELEASE tier assumes that
  skill is the maintained adversarial-review surface; confirm before RELORCH-A.
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
