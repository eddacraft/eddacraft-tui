<!--
CANONICAL pre-tag release-council OUTPUT template. Do NOT copy the previous
tag's artefact — copy THIS file:

  cp plans/reviews/release-council/TEMPLATE.md \
     plans/reviews/release-council/$(date +%Y-%m-%d)-<tag>-pre-tag.md

then fill every {{placeholder}}. Process: docs/runbooks/release-process.md.
Placeholders: {{tag}} {{date}} {{prior_tag}} {{candidate_sha}}
{{candidate_sha_short}} {{diff_scope}}=<prior_tag>..<candidate_sha>
{{prior_council_file}} {{prior_council_session}}. Update THIS template to
propagate a change to all future tags.
-->

# {{tag}} pre-tag release council — {{date}}

- **Candidate SHA:** `{{candidate_sha}}` (`{{candidate_sha_short}}`), frozen on `release/{{tag}}`
- **Release diff:** `{{diff_scope}}`
- **Prior tag / council:** `{{prior_tag}}` — `{{prior_council_file}}`
- **Tier:** {{tier}} (full | focused — see the input brief's rationale)
- **Sweep report:** `plans/audits/{{date}}-clawpatch-{{tag}}.json`

## Reviewer outputs (artefacts)

| Reviewer | Verdict | Findings (C/M/min/N) |
| --- | --- | --- |
| council-reviewer (architect) | {{verdict}} | 0 / 0 / 0 / 0 |
| security-analyst | {{verdict}} | 0 / 0 / 0 / 0 |
| adversarial-reviewer | {{verdict}} | 0 / 0 / 0 / 0 |
| operations-reviewer | {{verdict}} | 0 / 0 / 0 / 0 |
| pragmatic-lead | {{verdict}} | 0 / 0 / 0 / 0 |

## CHANGELOG excerpt (verify before sign-off)

```
## [Unreleased]
## [{{tag}}] — {{date}} — {{theme}}
## [{{prior_tag}}] — {{prior_date}} — {{prior_theme}}
```

## Judge synthesis

{{Overall ship / fix / defer verdict for the release diff, and the reasoning.
Note per-PR Council coverage already applied via Streaming Council, and which
carry-forward gates from the prior tag still hold.}}

## Required actions before `tag.sh` (must-fix)

_Only critical / must-fix findings appear here. Each forces a hardening commit on
`release/{{tag}}` and a delta-only re-pass (release-process.md §Re-passes). If
none, state "None — clean ship."_

### A1 — {{title}}

{{what / why / the fix}}

## Deferred actions (filed, not blocking)

_Major / minor / nit findings. Filed as GH issues; they do NOT block the tag
(defer-don't-block). Each row names the issue._

### D1 — {{title}} ({{#issue}})

{{summary + why it's non-blocking}}

## Carry-forward verdicts from the prior tag pass

_Verdicts carried from `{{prior_council_file}}` (and from prior passes of THIS
tag) via the `anvil capsule` snapshot — not re-derived. List what carried and
why it still holds._

## Tag-cut sequence (post-sign-off)

1. Human gate signed below.
2. Merge `release/{{tag}}` (if a stabilisation branch was used).
3. `promote.sh` records the state → `tag.sh` cuts `{{tag}}` at `{{candidate_sha}}`.
4. Fill the release record `plans/releases/{{tag}}.md`, referencing this file and the input brief.

## Human gate sign-off

The `release` tier requires an explicit **human** sign-off before `tag.sh` runs.

```
Signed-off-by: {{name}}
Date:          {{date}}
Verdict:       ship | fix | defer
Notes:         {{notes}}
```

## Appendix: reviewer one-liners

- **council-reviewer:** {{one-liner}}
- **security-analyst:** {{one-liner}}
- **adversarial-reviewer:** {{one-liner}}
- **operations-reviewer:** {{one-liner}}
- **pragmatic-lead:** {{one-liner}}
