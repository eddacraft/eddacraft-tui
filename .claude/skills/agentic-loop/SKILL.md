---
name: agentic-loop
description: >-
  Canonical agentic delivery lifecycle for APS work items and modules. The active
  native harness lead owns implementation by default; delegated workers are
  optional. Independent verification is mandatory, Council is risk- and
  uncertainty-triggered, and objective state transitions depend on fresh evidence.
---

# Canonical agentic loop

This is the portable lifecycle contract. Harness adapters decide how Codex,
Claude Code, OpenCode, or Grok execute it natively. Anvil may strengthen claims,
policy, evidence, provenance, and approvals when present; the loop must still
operate without Anvil and must not recreate Anvil internally.

## Core principle

Canonical behaviour, harness-specific choreography.

- APS defines authorised scope, acceptance, dependencies, risk, and reasoning.
- The active harness **lead** owns the run and implements directly by default.
- A **worker** is optional and exists only when delegation has an explicit benefit.
- Objective gates come from Git, commands, CI, PR state, and fresh evidence.
- A fresh, read-only **verifier** is mandatory before a completion claim.
- **Council** is a design/assurance escalation mechanism, not a mandatory repair loop.

## Roles

### Lead

The primary agent/session the developer is already using. It owns target scope,
implementation, delegation decisions, repair routing, PR lifecycle, and terminal
authority. It may write implementation code directly.

### Worker

Optional delegated implementation role. Use only for one or more of:
parallel independent work, specialist capability, cheaper mechanical work,
context isolation, or deliberate cross-model implementation. Each writing worker
gets an owned workspace/write surface and bounded outcome.

### Advisor

Optional read-only challenge for architecture, ambiguity, unfamiliar technology,
security-sensitive decisions, difficult debugging, module decomposition, or
repeated no-progress.

### Verifier

Mandatory fresh-context, read-only acceptance check. It receives the governing
contract, APS scope, immutable candidate, acceptance criteria, and gates — never
the implementation reasoning transcript. It never repairs implementation.

### Council

Risk-selected multi-perspective review. `council design` challenges planning and
architecture before code. `council assurance` adjudicates material residual risk
after verification. Council does not repair implementation itself.

## Lifecycle

### 1. Resolve

Establish target, governing sources, acceptance criteria, non-goals,
dependencies, risk, reasoning, authority, integration branch, and publication
policy. APS is planning truth; Git is implementation truth; PR/checks are review
truth.

If scope is stale, ambiguous, blocked, or unauthorised, stop `needs-plan-update`
or `blocked`. Do not manufacture readiness.

### 2. Shape / challenge when triggered

Skip ceremony for already-clear routine work. Invoke an advisor or `council design`
when policy, risk, frontier reasoning, architectural ambiguity, migration/trust
boundaries, module decomposition, irreversibility, or repeated failed approaches
justify it.

The output must tighten the contract: chosen approach, alternatives, invariants,
risks, open questions, required validation, and landing strategy where relevant.

### 3. Isolate

Never implement on the protected/default branch. Acquire the available claim and
isolate when required by policy, module scope, autonomous work, or parallel
writers. Treat degraded claims as advisory and record the limitation.

### 4. Implement

The lead implements directly by default. Delegate workers only for an explicit
reason and record that reason.

A delegated worker receives: bounded objective, allowed write surface/workspace,
interfaces/invariants, constraints/non-goals, acceptance checks, and requested
reasoning/effort when the harness supports it.

Do not force every coherent ReadyItem into one- or two-step subagent hand-offs.
Bound authority, time, write ownership, and evidence; let the selected model solve
a coherent problem.

### 5. Evidence

Run the deterministic validation appropriate to the changed surface through
`evidence-gate`. An agent success statement is never evidence.

### 6. Verify

Run fresh blind `verify-loop` against the immutable candidate. Bind the decision
to target, base/head, governing sources, and relevant validation evidence.

Decisions: `pass`, `pass-with-advisories`, `repair-required`, `blocked`,
`under-specified`.

Verification must be bounded however it is transported. The adapter proves fresh
context, read-only implementation authority, immutable candidate identity, and
durable result capture, and it must be able to stop a verifier that stalls:

- a finite wall-clock deadline for the whole verification, set before dispatch;
- the ability to cancel or terminate the verifier and any process tree it
  started;
- at most one controlled retry, which does **not** reset the deadline;
- a stalled or cancelled verifier is `blocked`, never a pass.

If the adapter cannot meet these, verification is `blocked`. It may fall back to
supervised headless transport when policy permits, but it may not drop the
bounds.

### 7. Classify failure before repairing

Do not automatically convert every finding into a patch loop. Classify the
failure:

- `REPAIR` — a small bounded set of in-contract defects; incremental repair is economical.
- `REWORK` — implementation attempt is structurally poor or defect-dense; replace/escalate the implementation approach rather than patching symptoms.
- `REPLAN` — governing intent, architecture, decomposition, or acceptance is wrong/incomplete; return to planning/design.
- `BLOCK` — environment, dependency, authority, or unresolved critical risk prevents progress.

Repository policy may define numeric rework thresholds, but the semantic rule is
stronger: once defect density shows the attempt itself is bad, stop spending on
finding-by-finding repair.

After a repair/rework, rerun affected deterministic evidence and independent
verification. A repair budget never weakens the meaning of `verified`.

### 8. Publish

Open/update the PR only when the candidate has adequate evidence for publication.
Bind the verification reference to the exact candidate head. Draft/parked PRs may
be used for blocked partial work; `review-ready` means the configured done bar is
actually met.

### 9. PR / CI / review

Process configured review sources; do not hard-code Copilot as a canonical done
bar. Red CI, conflicts, blocking review findings, or changed scope route back to
repair/rework/replan as appropriate.

### 10. Re-verify changed surface

When the candidate head changes after verification, stale the old decision.
Use targeted delta verification only when the runtime/lead can justify that the
change is a narrow repair with unchanged scope and risk surface; otherwise run a
fresh full verification.

### 11. Land

Immediately before merge require, at minimum:

- current PR head is the verified head;
- governing contract has not materially changed;
- required deterministic CI is green;
- no conflicts or conflict markers;
- no unresolved blocking review/Council findings;
- merge authority is present.

Then invoke `land-branch`. That skill's **pre-merge probe** is the merge gate:
unresolved threads, red CI, and conflicts go to `address-reviews`; `BLOCKED`
with no unresolved threads is missing authority, not a repair pass. Do not run
the full `address-reviews` skill as a no-op. Never infer merge authority.

### 12. Reconcile

Prove integration ancestry before marking APS `Merged`, reconcile APS/files/evidence,
release claims, and clean up branch/worktree state according to policy.

## Composed skills

The lifecycle is a contract, not an implementation. Each stage delegates to a
leaf skill; load the leaf when you reach its stage rather than up front.

| Stage | Skill |
| --- | --- |
| Resolve APS truth | `aps-planning` |
| Shape / challenge | `grill-design`, `council` |
| Readiness contract | `plan-ready` |
| Isolate | `isolate-workspace` |
| Implement (test-first) | `loop-build-tdd` |
| Diagnose a failure | `loop-debug` |
| Evidence | `evidence-gate` |
| Verify | `verify-loop` |
| PR feedback and base sync | `address-reviews` |
| Land | `land-branch` |
| Workflow audit | `loop-retro` |

Drain-mode leaves (`aps-probe`, `aps-safety-rails`, `aps-escalation-queue`,
`aps-resume`, `aps-landing`) are owned by the `dev-loop` front door, which also
holds the concrete policy, claim, evidence, and run-state contracts for this
catalogue.

A stage may be satisfied directly by the lead when the leaf skill would add
ceremony without adding assurance. Evidence, verification, and landing are not
such stages: they always run through their leaf skill.

## Durable run state

Every run carries resumable state. Update one checkpoint at each phase
transition, evidence gate, verifier decision, and landing step; do not create a
second checkpoint location. The concrete schema and default path live with
`dev-loop`.

Module, drain, and autonomous runs require a finite aggregate deadline and a
shared repair budget. Stop when a progress fingerprint repeats: a loop that is
not moving must escalate, not spend.

## Council policy

Normal standard-risk work: deterministic evidence → fresh verifier. No Council
unless risk, uncertainty, defect signal, or dispute requires it.

High-risk work: verifier + one or two risk-selected Council lenses.

Critical/irreversible/disputed work: design Council when useful, verifier, close
assurance Council, and human merge authority unless repository policy explicitly
states otherwise.

A Council pass is against one immutable candidate. After its findings, permit one
bounded repair pass by default; deterministic evidence and the verifier check the
repair. Do not automatically run another full Council. A new full Council requires
material redesign/scope change, a critical redesign finding, a new material
verifier concern, or explicit policy/user request.

## Module calibration

For a non-trivial module, prefer an early representative calibration slice before
expensive backlog draining. Verify that slice (and optionally Council-calibrate it
when risk warrants). If defect density is high, stop the drain and replan/rework
before generating more weak slices. If quality is healthy, continue with lighter
focused evidence and finish with integrated module-close verification.

## Terminal outcomes

Return one of:

`review-ready`, `integrated`, `blocked`, `needs-plan-update`, `claim-conflict`,
`awaiting-merge-authority`, `repair-budget-exhausted`.

Never call work complete merely because implementation finished.
