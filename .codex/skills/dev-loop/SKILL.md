---
name: dev-loop
description: Orchestrate development from an APS work item, module, or natural-language goal through planning, isolated implementation, independent verification, review, PR, and integration. Use for requests such as "complete DASH-001", "complete DASH", "build this goal", "resume this work", or autonomous plan execution. Resolve the installed harness adapter automatically; use explicit dev-loop-codex, dev-loop-claude, dev-loop-grok, or dev-loop-opencode bindings for testing or cross-harness execution.
---

# Development Loop

Deliver the explicitly named target. Scope comes from the target; operating mode controls checkpoints and authority.

## Invocation

Prefer explicit intent:

```text
/dev-loop complete DASH-001
/dev-loop complete DASH
/dev-loop goal "Add tenant export"
/dev-loop resume DASH-001
```

Treat `/dev-loop DASH-001` as shorthand for `complete`. Read the default mode from repository policy. If policy is absent, ask for `interactive` or `autonomous`; never infer merge authority.

- `complete <ITEM>` owns one APS work item.
- `complete <MODULE>` owns the module and may delegate child items.
- `goal <text>` invokes `planning-workflow`, produces or updates an APS plan, then continues. Interactive mode pauses for plan approval; autonomous mode continues within policy.
- `resume <TARGET>` reconstructs state from APS, Git, PR evidence, and the run checkpoint, then verifies it before acting.

## Non-negotiable invariants

1. Never implement on the protected or default branch.
2. Use a fresh PR branch. Prefer a Worktrunk-managed worktree; require isolation for modules, autonomous runs, and parallel writers.
3. Acquire the target claim before writes. A module claim reserves its namespace and may issue published child leases.
4. APS is planning truth, Git is implementation truth, PR/checks are review truth, and the run checkpoint is resumable orchestration state.
5. Give verifiers the governing specification, relevant APS scope, base/head or bounded diff, acceptance criteria, and gate requirements—never the executor's reasoning transcript.
6. Require a fresh adversarial verification context. Keep the verifier read-only. Use a different model or harness for high-risk work and disputed findings when available.
7. Evidence gates every transition. Never advance state from confidence or another agent's success claim.
8. Repository policy and branch protection outrank the mode flag. A session-scoped human override must name its scope, authority, and expiry and be recorded in evidence.

Read [references/policy-contract.md](references/policy-contract.md) when resolving mode, risk, merge authority, repair budget, isolation, or overrides. Read the JSON schemas when creating or validating checkpoints, claims, or evidence.

## Roles

- **Orchestrator:** own scope, dependency order, claims, state, repair routing, and terminal decision.
- **Executor:** implement a bounded target in an owned workspace.
- **Advisor:** investigate or critique without writing implementation.
- **Specialist:** act as executor or advisor for a named domain.
- **Verifier:** independently test the completion claim and issue evidence-backed findings; remain read-only.

Use adaptive orchestration. Keep a capable executor while context remains healthy; introduce fresh contexts, advisors, specialists, or parallel executors where independence or expertise adds value. Never allow agents to share an unpartitioned write surface.

## Loop

### 1. Resolve

Resolve intent, target, repository policy, effective mode, APS scope, dependencies, risk tier, integration branch, and required gates. Effective risk is the maximum of APS metadata, repository policy, and orchestrator assessment. The orchestrator may raise risk but may lower it only through a recorded human override.

If the target is natural language, invoke `planning-workflow`. If the target is an APS identifier, invoke `aps-planning` truth validation. Do not implement stale, ambiguous, blocked, or unauthorised work.

### 2. Claim and isolate

Acquire a hierarchical claim before any write. Refuse a conflicting claim. Record operator, orchestrator session, executor, target, parent claim, branch, workspace, base revision, timestamps, and lease state.

Create or reuse an explicitly owned non-default branch. Use the current worktree only when it is clean, already on the owned feature branch, and no parallel writer exists. Otherwise use Worktrunk to create an isolated worktree and fresh PR branch. Establish a green baseline or record inherited failures.

The first version specifies the claim protocol but does not pretend a distributed lock exists. Until the Git-native coordination module is implemented, use the strongest available shared claim surface and report degraded collision guarantees. See [references/coordination-module.md](references/coordination-module.md).

### 3. Design when required

For architectural, security-sensitive, irreversible, or materially ambiguous work, obtain two independent designs before implementation, then synthesise and record the choice. Do not show either proposal to the other designer. For lower-risk work, use advisors only when expected value exceeds cost.

### 4. Execute

Decompose the target along APS work-item boundaries. Use `test-driven-development`, `systematic-debugging`, and repository-specific skills as applicable. Apply repository policy and the plan's validation requirements. Keep commits focused and attach APS identifiers.

Parallelise only dependency-independent work with explicit ownership. A module orchestrator remains accountable and publishes child leases for delegated work.

After each meaningful boundary, update the run checkpoint. Keep APS `In Progress`; use checkpoint phases such as `implemented`, `verified`, and `review-ready` rather than inventing new APS lifecycle states.

### 5. Verify

Invoke `verify-loop` with the governing contract and bounded change set. Start with specification + plan + diff, then allow relevant repository inspection and fresh command execution. Treat objective gate failures and high-confidence critical/major findings as binding. Route subjective findings to orchestrator judgement; use differential or Council review for material disputes.

### 6. Repair

The orchestrator chooses the original executor, a fresh executor, or a specialist based on defect type, ownership, context health, and repeated failure. The verifier does not repair by default.

Continue while evidence materially improves and the repair budget remains. Escalate expertise before exhaustion. Stop immediately for missing authority, safety boundaries, inaccessible dependencies, or ambiguity that changes intent. Detect repeated findings, repeated patches, and no-progress cycles; then emit a structured blocker checkpoint rather than looping indefinitely.

### 7. Review and land

Default the PR boundary to the invocation target. Split before implementation when size, risk, or independent deployability warrants it, and record the decision. Work items still receive meaningful commits and evidence inside a module PR.

- **Interactive:** finish only when the PR is open, CI is green, automated/agent findings are resolved, and it is ready for human review. Resume explicitly for later feedback.
- **Autonomous:** remain responsible for CI and review feedback, re-verify every repair, and merge only when invocation authority, repository policy, risk gates, branch protection, and any scoped override permit it.

After merge, update the base before dependent work. Preserve the canonical APS lifecycle: `Ready -> In Progress -> Merged -> Released/Shipped -> Complete`. `Complete` remains release/ship gated by project policy.

### 8. Reconcile and release

Reconcile APS, evidence, PR revision, findings, and claim outcome. Release or transfer leases. Preserve abandoned branches and evidence during stale-claim recovery. Report outcomes first: integrated work, verification evidence, APS changes, open risks, overrides used, and the next eligible target.

## Stop outcomes

Return exactly one orchestration outcome:

- `review-ready` — interactive terminal state.
- `integrated` — merged and post-merge requirements passed.
- `blocked` — external dependency, authority, safety, or ambiguity prevents progress.
- `repair-budget-exhausted` — no material progress remains within policy.
- `needs-plan-update` — target truth or scope must be replanned.
- `claim-conflict` — another operator or orchestrator owns the target namespace.
- `awaiting-merge-authority` — the autonomous run is review-ready but policy, required approval, or a missing override withholds merge authority.

Never call work complete when the strongest supported state is merely implemented, verified, or review-ready.
