<!--
CANONICAL pre-tag release-council INPUT-BRIEF template. Do NOT copy the previous
tag's input brief — copy THIS file:

  cp plans/reviews/release-council/TEMPLATE-input.md \
     plans/reviews/release-council/$(date +%Y-%m-%d)-<tag>-pre-tag-input.md

then fill every {{placeholder}}. This brief is the council's INPUT; the output
lands in the sibling <date>-<tag>-pre-tag.md (TEMPLATE.md). Process:
docs/runbooks/release-process.md.
-->

# {{tag}} pre-tag council — input brief

- **Candidate SHA:** `{{candidate_sha}}` (`{{candidate_sha_short}}`), frozen on `release/{{tag}}`
- **Release diff:** `{{diff_scope}}`
- **Prior tag / council:** `{{prior_tag}}` — `{{prior_council_file}}`
- **Sweep report:** `plans/audits/{{date}}-clawpatch-{{tag}}.json`

## Tier / focus rationale

**{{tier}}** — {{full: whole panel over the full window | focused: claim-scoped,
smaller panel, patch tag}}. {{Why this tier: e.g. "patch tag, single claim, most
of the diff is freight; full-tier is disproportionate."}}

## Claim (verbatim from the release record)

{{The ship claim for this tag, copied verbatim from `plans/releases/{{tag}}.md`
so the council reviews against the stated claim, not an inferred one.}}

## Claim gates (all Merged at the candidate SHA)

| Gate | Evidence (PR / file:line) | State |
| --- | --- | --- |
| {{gate}} | {{evidence}} | Merged |

## Freight (rides the tag — not load-bearing for the claim)

{{Changes in the window that are not part of the claim — routine merges, docs,
chores. Named so the council can weight them lightly.}}

## Hard-gate evidence (code-state-verifiable)

{{The release-runbook hard gates (build/test/lint/cross-target/etc.) with their
verifiable evidence at the candidate SHA — so the council reviews a real
green-state, not an assumed one.}}

## Operator-visible observations to surface

{{Anything the operator wants the council to weigh: known-accepted risks, a prior
carry-forward that needs re-confirmation, a deliberately-deferred class, etc.}}

## Council questions

1. {{The specific questions this pass must answer — e.g. "does the frozen
   candidate honour the claim?", "any critical the sweep missed?"}}

## References

- Process: [`docs/runbooks/release-process.md`](../../../docs/runbooks/release-process.md)
- Prior council: `{{prior_council_file}}` (session `{{prior_council_session}}`)
- Release record: `plans/releases/{{tag}}.md`
- Sweep report: `plans/audits/{{date}}-clawpatch-{{tag}}.json`
