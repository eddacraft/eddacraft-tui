# Pre-Tag Review Process

| Type    | Authority     | Owner   | Status | Freshness                                                                                         |
| ------- | ------------- | ------- | ------ | ------------------------------------------------------------------------------------------------- |
| Runbook | Authoritative | RELORCH | Live   | First filed 2026-07-02 (design `plans/specs/2026-07-02-release-pretag-review-doctrine-design.md`) |

| Upstream                                                                                                                                                                                                             | Downstream                                                                                                                                                                                   |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`release-runbook.md`](release-runbook.md), design `plans/specs/2026-07-02-release-pretag-review-doctrine-design.md`, `clawpatch`, `council` skill, [ADR-074](../../plans/decisions/074-review-capsule-v0-format.md) | [`plans/reviews/release-council/TEMPLATE.md`](../../plans/reviews/release-council/TEMPLATE.md), [`TEMPLATE-input.md`](../../plans/reviews/release-council/TEMPLATE-input.md), release record |

Canonical, **tag-agnostic** doctrine for the pre-tag review gate: an adversarial
sweep, a multi-persona council, and a human sign-off, run against a **frozen
candidate** before a tag is cut. The authoritative
[`release-runbook.md`](release-runbook.md) references this as a required step;
version-specific runbooks link up to it and add only the gates unique to that
tag.

Substitute the tag being cut for `<tag>` (e.g. `v0.9.0-beta`), the prior
released tag for `<prior-tag>`, and the frozen candidate SHA for
`<candidate-sha>` throughout.

## Why this exists (and why it must not tax the tag)

The pre-tag council has repeatedly found genuinely useful issues. Its earlier
incarnation lapsed for one reason: **every fix moved `main`'s SHA, invalidated
the candidate, and forced a full sweep + full council from scratch.** That
restart tax — not the review — is what made it painful. This doctrine keeps the
review and removes the tax (see
[Re-passes](#re-passes-delta-only-never-a-restart)).

## The candidate is frozen (never chase `main`)

The review runs against a **frozen candidate**, not live `main`:

- Cut a short-lived stabilisation branch `release/<tag>` at the candidate SHA
  (the runbook's `promote.sh --strategy stabilisation` path).
- The sweep and council review **that branch**. `main` keeps moving
  independently — the candidate never chases trunk.
- **Must-fix** fixes land on `release/<tag>` (PR into the branch), _not_ on
  `main` first. The candidate SHA changes only when a hardening commit is
  deliberately applied to the branch.

This is what makes a re-pass cheap: the thing under review holds still.

## Gate A — adversarial sweep (`clawpatch`)

A whole-repo adversarial sweep of the release window on the frozen candidate.
`clawpatch` walks the feature map, runs the AI escape-hatch detectors,
anti-pattern engines, and policy classifiers against each feature, and records
findings — broader than the per-PR watch surface because it sees the entire
backlog in one pass.

```bash
clawpatch map                                   # alias: claw-map — refresh the feature map
clawpatch review --limit <N> --jobs 8           # alias: claw-sweep — full-backlog sweep
clawpatch status                                # alias: claw-status
clawpatch report --json > plans/audits/$(date +%Y-%m-%d)-clawpatch-<tag>.json
```

Commit the JSON under `plans/audits/` so the council reads findings together
with the release diff and the release record references a durable artefact.

**Pass criteria.** `claw-sweep` exits clean (every mapped feature has a fresh
review record); `claw-status` shows no `error` rows; **no `severity: critical`**
finding (any critical is an automatic block — fix or honestly defer with a named
GH issue before re-running); `severity: high` findings are enumerated for the
council to triage in Gate B (they do not auto-block, but each needs an explicit
ship / fix / defer verdict).

## Gate B — release council

A multi-persona [`council`](../../.claude/commands/council.md) review over the
release diff `<prior-tag>..<candidate-sha>`, reading Gate A's findings alongside
it. The judge produces a ship / fix / defer verdict per finding plus an overall
release-diff verdict.

**"Release tier" is an operator convention, not a `/council` flag:** run the
**`full`** pack (all reviewer roles, including `security-analyst`) in **batch**
mode, and treat the [human gate](#human-gate) +
[defer-don't-block](#triage-defer-dont-block-by-default) as the release-specific
additions. `/council`'s own tiers are `quick | mini | full`; there is no
`release` tier and no `--findings`/`--tier`/`--output` flags. This is the
**full** case — patch tags may use a smaller pack, see
[Full vs focused](#full-vs-focused).

Author the two artefacts **from the templates** (never by copying the prior
tag's file), run the council over the diff, then synthesise its findings into
the output artefact by hand (the council does not write the artefact):

```bash
cp plans/reviews/release-council/TEMPLATE-input.md \
   plans/reviews/release-council/$(date +%Y-%m-%d)-<tag>-pre-tag-input.md
cp plans/reviews/release-council/TEMPLATE.md \
   plans/reviews/release-council/$(date +%Y-%m-%d)-<tag>-pre-tag.md
# fill the {{placeholders}} in both, then run a full-pack council over the diff:
/council full <prior-tag>..<candidate-sha>
# read Gate A's plans/audits/<date>-clawpatch-<tag>.json as findings input, then
# record verdicts + the human gate in the -pre-tag.md artefact.
```

**Pass criteria.** Every Gate A finding has an explicit council verdict; the
overall verdict is **ship** for the release diff; no reviewer has an unresolved
CRITICAL outstanding (the judge routes contradictions to council-debate; the
debate verdict is final); the [human gate](#human-gate) is signed in the output
artefact.

## Triage: defer-don't-block by default

Only **critical / must-fix** findings force a hardening commit and a re-pass.
Everything else — major, minor, nit — is **filed as a GH issue and does not
block the tag** (the warnings-over-blocks posture,
[ADR-002](../../plans/decisions/002-warnings-over-blocks.md)). This is the
single biggest reason the process stays cheap: most findings become follow-up
issues and trigger **zero** re-cuts. The output artefact records them under
"Deferred actions (filed, not blocking)".

## Re-passes: delta-only, never a restart

When a hardening commit lands on `release/<tag>`, do **not** re-run the full
sweep + full council. Re-review only the delta:

1. **Scoped sweep** of the changed features only:

   ```bash
   BASE_REF=<prior-candidate-sha>
   git diff --name-only "$BASE_REF" \
     | clawpatch map --features-owning - --json | jq -r '.[]' \
     | xargs -r -I{} clawpatch review --feature {}   # alias: claw-branch
   ```

2. **Re-verify only the finding(s)** the commit closes; **carry forward** every
   other verdict from the prior pass. Snapshot the prior pass as an
   [`anvil capsule`](../../plans/decisions/074-review-capsule-v0-format.md) so
   verdicts reattach instead of being re-derived; the output artefact's
   "Carry-forward verdicts" section records what carried.

A re-pass therefore touches only the frozen branch and reviews only the fix. The
"start again over the whole window" loop is gone.

> **Do not silently widen a re-pass back to a full sweep.** If an operator
> re-runs the whole council out of caution, the tax returns. Delta-only +
> carry-forward is the default; deviate only with a recorded reason.

## Upstream: Streaming Council shrinks the batch

The [`council`](../../.claude/commands/council.md) skill's **Streaming Council**
runs during implementation; **Batch Council** at milestones. With streaming
review already applied across the window, this pre-tag Gate B is a
**confirmation** over largely-reviewed code, not a discovery-from-scratch sweep
— far fewer surprise must-fix findings at tag time, so far fewer re-passes.

## Human gate

The `release` tier requires a **human** sign-off: the release owner (not just
the agent panel) signs the synthesised verdict in the output artefact's "Human
gate sign-off" block **before** `tag.sh` runs. No sign-off, no tag.

## Full vs focused

- **Full** (minor/significant tags, `vX.Y.0`): the whole reviewer panel over the
  full window diff.
- **Focused** (patch tags, `vX.Y.Z`, Z>0): a claim-scoped input brief and a
  smaller reviewer set, reviewing only the patch's claim + its gates (as
  `v0.7.1-beta` did). Record the choice in the input brief's "tier / focus
  rationale" field.

With the re-cut tax removed, a focused confirmation pass is cheap enough to run
for **every** tag.

## Where the artefacts live

- Sweep report: `plans/audits/<date>-clawpatch-<tag>.json`
- Council input brief:
  `plans/reviews/release-council/<date>-<tag>-pre-tag-input.md`
- Council output: `plans/reviews/release-council/<date>-<tag>-pre-tag.md`
- Both council files are referenced from the release record
  (`plans/releases/<tag>.md`).
