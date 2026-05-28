# Issue Triage And APS Authority

| Type | Authority     | Owner | Status | Freshness                                                                 |
| ---- | ------------- | ----- | ------ | ------------------------------------------------------------------------- |
| Spec | Authoritative | CIB   | Live   | Created 2026-05-28 from planning discussion and Planning Council feedback |

| Upstream                                                | Downstream                                                          |
| ------------------------------------------------------- | ------------------------------------------------------------------- |
| `AGENTS.md`, `plans/aps-rules.md`, `plans/project-context.md`, `plans/modules/continuous-improvement-backlog.aps.md` | GitHub issue triage, PR descriptions, APS/CIB promotion decisions |

## Purpose

Define how Anvil uses public GitHub issues, private monorepo issues, CIB, and
APS together without turning GitHub issues into a shadow planning system or
making every small bug pay full APS ceremony.

The core rule is:

> Private monorepo issues or PRs can authorise small fixes; APS authorises
> planned work.

Public GitHub issues are evidence and discussion. They do not authorise
implementation by themselves unless the work is also covered by a private
small-fix rationale, CIB item, APS work item, or emergency-hotfix declaration.

## Surfaces

| Surface | Role | Authority |
| ------- | ---- | --------- |
| Public `eddacraft/anvil` GitHub issues | Publicly identified bugs, discussion, support signal, public reproduction evidence | Intake and public evidence only |
| Private `eddacraft/anvil-001` GitHub issues | Internal triage for true bugs, high-priority defects, small fixes, private agent/dev evidence | Intake plus small-fix authority when the exemption applies |
| `plans/modules/continuous-improvement-backlog.aps.md` | Standing intake for concrete cross-cutting improvements | APS-backed execution authority for small/medium internal improvements |
| Dedicated APS modules | Product, platform, release, architecture, or workflow work with sequencing and validation needs | Execution authority |
| ADRs | Durable architectural or process decisions | Decision authority |
| `RELEASE-PLAN.md` | Current release cut and release-blocker context | Release-slate authority |

## Issue Triage Outcomes

Every triaged GitHub issue should end in one of these outcomes:

| Outcome | Meaning |
| ------- | ------- |
| `small-fix` | Narrow enough to fix directly from the issue or PR without APS |
| `needs-aps` | Needs APS/CIB authority before implementation |
| `cib-candidate` | Concrete internal improvement that probably belongs in CIB |
| `promoted-to-aps` | Already represented by a CIB item or APS work item |
| `gh-only` | Discussion, support, duplicate, declined, or parking-lot item |

Public issues may remain public evidence after promotion. Private issues may
hold implementation detail, reproduction artefacts, and agent coordination, but
they do not replace APS when the work is non-trivial.

## Small-Fix Exemption

A private monorepo issue or PR may bypass APS when all criteria are true:

- The change fits in one PR.
- The affected area is one surface or a very narrow file set.
- Behavioural risk is low and easy to explain.
- No public API, schema, feature flag, release policy, trust-boundary, or
  architectural contract changes.
- No sequencing, dependency, rollback, or follow-up state is needed.
- Validation is obvious and can be stated directly in the PR.

Examples that usually qualify:

- Typo or broken link fixes.
- Obvious CLI copy or help-text correction.
- One failing fixture or snapshot update with a clear cause.
- Isolated regression with a clear reproduction and targeted test.
- Narrow docs correction that does not change policy.

Examples that do not qualify:

- CI workflow, release, branch, or agent lifecycle changes.
- Security-sensitive or trust-boundary changes.
- Cross-crate, cross-package, or cross-surface implementation.
- User-visible behaviour changes with product implications.
- Anything delegated to agents as meaningful scheduled work.
- Anything where the small-fix rationale feels like a stretch.

## Priority

Priority describes delivery urgency, not abstract importance.

| Priority | Meaning | APS Expectation |
| -------- | ------- | --------------- |
| `P0` | Active incident: unsafe release, security exposure, broken `main`, publish/release corruption, data loss risk | Do not block containment on APS; reconcile into APS/CIB/release evidence after containment |
| `P1` | Current-release blocker or high-trust bug: user-visible regression, required CI failure, install/start broken, serious false positive/negative | APS/CIB usually required unless it is a narrow hotfix |
| `P2` | Planned important work: meaningful bug, workflow friction, product/DX improvement, recurring failure | APS/CIB expected before implementation |
| `P3` | Small opportunistic fix: typo, doc link, isolated test cleanup, obvious narrow bug | GitHub issue or PR small-fix rationale is enough |
| `P4` | Parking lot: interesting idea, weak signal, no current consumer, needs more evidence | No execution commitment |

Default assumptions:

- New public issues start unprioritised until triage confirms impact.
- Private true bugs usually start at `P2`.
- Tiny fixes usually start at `P3`.
- Release blockers are `P1`.
- Active harm is `P0` and should be rare.

## Labels

Use labels as routing hints, not as planning truth.

Recommended minimal labels:

```text
priority:P0
priority:P1
priority:P2
priority:P3
priority:P4

kind:bug
kind:ci
kind:security
kind:docs
kind:dx
kind:workflow
kind:product
kind:maintenance
kind:research

readiness:needs-triage
readiness:needs-design
readiness:ready
readiness:blocked

tracked:small-fix
tracked:needs-aps
tracked:cib-candidate
tracked:promoted-to-aps
tracked:gh-only
```

The public repo can use a smaller visible subset if private scheduling detail
would create noise or leak internal priorities.

## Promotion Rules

Promote an issue into CIB or a dedicated APS work item when any of these are
true:

- It cannot satisfy the small-fix exemption.
- It becomes more than one PR.
- It needs scheduling, dependencies, or handoff between agents/humans.
- It affects release readiness, CI, workflow, security, or governance.
- It changes public behaviour beyond a narrow bug fix.
- It produces follow-up work that should not live only in issue comments.

Promotion target:

| Condition | Target |
| --------- | ------ |
| Cross-cutting improvement, process friction, recurring agent/dev workflow issue | CIB |
| Product feature, user-facing capability, or multi-step subsystem work | Dedicated APS module/work item |
| Durable architecture/process decision | ADR, with APS/CIB work item if implementation is needed |
| Current release blocker | APS/CIB plus `RELEASE-PLAN.md` if it affects the cut |

## PR Declaration

Every PR should declare what authorised the work.

Accepted forms:

```text
Authority: APS MLP2-051g
Authority: CIB-031
Authority: Private GH #1234 small-fix exemption — isolated help-text correction, validated by snapshot test
Authority: Emergency hotfix — containment first; APS/CIB reconciliation follow-up required
```

For small-fix PRs, the declaration must state why APS was unnecessary and how the
change was validated.

## Emergency Handling

`P0` work may start before APS/CIB exists when waiting would increase active
harm. The follow-up must reconcile the work into the correct authority surface:

- Release evidence if a release/publish state changed.
- CIB or APS if follow-up work remains.
- ADR if the incident changes durable architecture or operating policy.
- Public issue update when safe and useful for external visibility.

## Non-Goals

- This spec does not add automation.
- This spec does not require every GitHub issue to become APS.
- This spec does not make GitHub labels authoritative planning state.
- This spec does not replace `plans/aps-rules.md`, `plans/project-context.md`, or
  the CIB intake rules.

## Review Trigger

Revisit this policy after it has been used for several issue/PR cycles, or when
one of these occurs:

- Small-fix exemptions repeatedly hide non-trivial work.
- CIB becomes a dumping ground for vague ideas.
- Private GitHub issues begin to function as a parallel APS backlog.
- Agents repeatedly stall because an issue's authority is ambiguous.
- Automation would remove repeated manual triage friction.
