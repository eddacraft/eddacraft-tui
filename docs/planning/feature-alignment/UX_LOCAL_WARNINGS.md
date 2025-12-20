# UX_LOCAL_WARNINGS — Core Interaction Design

## Design intent
Seatbelt, not lecturer: interrupt at the moment of risk, stay calm, make the next step obvious.

## Primary trigger
File saved (IDE/CLI).

## Architecture boundary warning (template)
- Title: Architectural boundary crossed
- First line: `Payments` is calling into `Identity`
- Where: file:line (linkable)
- Pattern/rule: named pattern + named rule tied to boundary
- Impact: prefer user journey/feature; fallback to route/job/service
- Drift: “already violated elsewhere” + “this change introduces a NEW dependency”
- Suggestion: deterministic safer alternatives
- Actions: view map, show examples, suppress with note

## AI anti-pattern warning (template)
- Title: AI anti-pattern detected
- First line: Broad `eslint-disable` added
- Why: bypasses guardrails; commonly AI escape hatch
- Suggest: deterministic alternative; narrow suppression with reason; refactor
- Actions: view safer pattern, suppress with note

## Suppression UX
- Require human-written note
- Write inline marker (JSDoc/comment)
- Record structured provenance (author, time, rule, scope, note)
- Keep suppressions discoverable

## Confidence posture
- Default: careful phrasing (“appears to…”)
- Show explicit confidence labels only when low
