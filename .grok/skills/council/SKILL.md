---
name: council
description: >-
  Risk-selected multi-perspective review with two primary modes: design challenge
  before implementation and assurance review after verification. Spec-bound,
  read-only, deduplicated, persisted, and convergence-bounded.
---

# Council

Council is a quality sensor and decision aid, not the default implementation
engine. Preserve its existing persisted-session/reviewer machinery, but spend it
only where multiple perspectives materially improve a decision or expose residual
risk.

## Modes

### `council design`

Use before implementation for frontier reasoning, materially ambiguous module or
architecture work, migrations, trust boundaries, irreversible choices, repeated
failed approaches, or policy-triggered design review.

1. Resolve the problem, governing APS/spec/ADR sources, constraints, non-goals,
   risk/reasoning, and known alternatives.
2. Select 2–3 relevant lenses by default. Ask them independently to challenge
   acceptance completeness, architecture/decomposition, invariants, failure
   modes, validation strategy, and landing/rollback strategy where applicable.
3. Synthesize into one decision artefact: chosen approach, alternatives rejected,
   invariants, risks, open questions, required validation, and landing strategy.
4. Return `PASS` when the design is fit to implement, `REPLAN` when it is not, or
   `BLOCK` when an external decision/dependency is required. Do not write product
   code from Council.

The existing `planningSession` schema remains the persisted shape when a design
Council needs interrogation/negotiation state; do not invent a second planning
session format.

### `council assurance`

Use after independent verification when risk, dispute, defect signal, release
boundary, or repository policy warrants broader review of an immutable candidate.

Normal standard-risk work does **not** require Council: deterministic evidence
plus fresh `verify-loop` is the default assurance path.

### Compatibility operations

Existing operational surfaces remain supported:

```text
council                    # current target, risk-selected/quick review
council design             # planning/architecture challenge
council assurance          # post-verification assurance
council status             # inspect persisted session
council publish            # publication summary/PR artefact
council escalate           # add relevant perspectives to the active session
/council-full              # explicit expensive full supervised pack
```

`streaming`, `batch`, legacy reviewer packs, `full:codex`, status/publish, and
session escalation remain compatibility capabilities. They are no longer the
canonical assumption for every delivery run.

## Reviewer selection

Select sharp lenses by risk/domain rather than always running a fixed pack.
Canonical lenses include:

- `kernel-maintainer` — simplify, remove unnecessary abstraction/dependency,
  preserve correctness/performance;
- `adversarial-reviewer` — break assumptions, malformed input, boundaries,
  races, failure handling;
- `security-analyst` — threat/trust boundary and security review;
- `operations-reviewer` — runtime, restart, degradation, resource and production
  behaviour;
- `pragmatic-lead` — scope, user outcome, shipping and over/under-engineering
  trade-offs.

Typical selection:

- ordinary backend item: verifier only; maintainer/adversarial for calibration
  when useful;
- auth/trust boundary: security + adversarial (+ maintainer if complexity matters);
- daemon/runtime: operations + maintainer + adversarial;
- architecture/module design: maintainer + pragmatic + relevant domain lens.

Perspective diversity, model diversity, provider diversity, and context
independence are separate properties. Record what was actually achieved; do not
count multiple personas on one provider as cross-provider evidence.

## Governing contract

When a specification exists, it is the contract. The diff/plan is evidence, not
a licence to invent more scope. Resolve/pin the contract and immutable target
before assurance dispatch.

Classify every canonical finding:

- `in_contract` — may block;
- `later_item` — valid follow-up, never blocks this target;
- `out_of_scope` — do not promote into a blocker;
- `no_contract` — standards-only observation when no governing contract exists.

## Finding action

Every finding gets one action:

- `must_fix` — required for current contract; blocks landing;
- `should_fix` — high-value bounded improvement in scope, non-blocking by itself;
- `consider` — taste/optional optimisation;
- `later_item` — valuable later work.

Contract impact outranks aesthetic severity. A minor-labelled acceptance violation
may be `must_fix`; a major concern owned by a later item is not a current blocker.

## Persistence and adapters

Prefer a discovered Council runtime adapter for session/finding/evidence/publication
operations. The existing bundled scripts remain the POSIX fallback:

- `scripts/council-session.sh`
- `scripts/council-finding.sh`
- `scripts/council-evidence.sh`
- `scripts/council-publish.sh`
- `scripts/council-codex-reviewer.sh` for the explicit legacy Codex batch path

Session state remains outside the skill bundle. Pin base/head, target digest and
spec digest. If identity changes materially, the prior Council decision is stale.

Open a session with the mode you are actually running, and record the judge
outcome so the loop can route on it:

```sh
council-session.sh init --mode assurance --target staged --pack risk-selected
council-session.sh decide <session-id> --decision REWORK \
  --rationale "defect-dense attempt" --head "$(git rev-parse HEAD)"
```

`decide` persists `judgeResult` on the session and derives the legacy
`gate` value from the decision. A decision that is never recorded does not
survive resume or publication: `verdict` alone counts findings and cannot
express `REWORK` or `REPLAN`.
Native Windows still requires a conforming adapter for persisted Council state;
do not silently claim the POSIX fallback is native Windows support.

## Reviewer execution

Register each reviewer in session state, run it read-only against the pinned
contract/target, supervise output, deduplicate by stable fingerprint, and attach
corroborating sources instead of inflating finding counts. Reviewer transport
failure, timeout, malformed output, or authentication failure is a failed review
route, **not** a product finding and never a synthetic pass.

The explicit `/council-full` compatibility flow may still supervise/debate a full
pack. Ordinary design/assurance should select only the lenses justified by risk.

## Structural failure outcomes

A large coherent set of valid findings is not automatically "14 patches". The
judge decides whether the attempt itself failed:

- `PASS` — no current-contract blockers;
- `REPAIR` — small bounded current-contract defects; incremental repair economical;
- `REWORK` — implementation is structurally poor/defect-dense; replace/escalate
  the implementation approach instead of patching symptoms;
- `REPLAN` — intent, architecture, decomposition, or acceptance is wrong/incomplete;
- `BLOCK` — unresolved critical risk, authority, environment, or dependency;
- `DEBATE_REQUIRED` — material reviewer contradiction needs adjudication.

The judge also emits legacy `gate: PASS|WARN|BLOCK` for existing publication and
merge consumers. The agentic loop uses the richer `decision` for routing. Persist
both through `council-session.sh decide`; `council-session.sh status` and
`council-publish.sh` surface them alongside the finding-count verdict.

## Convergence rule

One Council invocation reviews one immutable target/candidate.

1. Judge and stop.
2. Permit one bounded implementation repair pass by default for `REPAIR`.
3. Rerun deterministic evidence and independent `verify-loop` on the repair.
4. **Do not automatically run another full Council.** A second full pass requires
   material redesign/scope change, a critical redesign finding, a new material
   verifier concern, explicit repository policy, or explicit user request.

Council reviewers never repair, push, merge, or change APS state themselves.
