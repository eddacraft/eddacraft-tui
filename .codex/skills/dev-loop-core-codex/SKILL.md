---
name: dev-loop-core-codex
description: >-
  Execute the wired dev-loop-core pack using Codex orchestration, subagents,
  advisory tasks, isolated workspaces, and independent verification. Use when
  running or testing dev-loop-core specifically in Codex, or when another
  harness delegates a development target to Codex.
---

# Codex development loop (core pack)

Load and obey **`dev-loop-core`** first. This binding maps its roles to Codex
capabilities and must not weaken the pack invariants or stage wiring.

## Stage wiring (unchanged)

Same as `dev-loop-core`:

`plan-ready` → `grill-design` (when needed) → `isolate-workspace` → `build-tdd`
→ `debug` (on failure) → `evidence-gate` → `verify-loop` → `land-branch` →
`address-reviews`. APS truth via `aps-planning` when present.

## Codex mapping

- Keep the **root agent as orchestrator** — sole owner of claims, APS reconcile,
  repair routing, PR state, and merge authority.
- Use **executor subagents** only for bounded, independently owned work
  (`build-tdd` slices).
- Use **advisor subagents** for design, research, critique, and investigation
  without writes (`grill-design`, design proposals).
- Use **fresh subagents for verification** (`verify-loop`): pass specification,
  APS/ReadyItem scope, base/head or diff, and required gates — **never** executor
  reasoning or conversation transcript.
- Parallel subagents only for dependency-independent work. Each writer gets its
  own Worktrunk worktree or non-overlapping write ownership via
  `isolate-workspace`.
- Continue useful local orchestration while subagents work, then reconcile
  against Git and fresh command evidence — do not trust summaries alone.
- If subagents or durable background execution are unavailable, degrade to
  sequential fresh contexts and record the limitation in the run checkpoint.
- Keep completion in the active turn while safe in-scope work remains; do not
  stop at a plan or promise.

## High-risk verification

Prefer a fresh verifier on a different available model, or delegate
`verify-loop` to another harness when policy requires cross-model checks. If the
agent-spawn API exposes no model selector, record `crossModel: unavailable` in
the checkpoint and proceed with a fresh same-model verifier rather than blocking.

## Advisor handoff

Use this handoff shape for advisor subagents:

```markdown
## Advisor Task

- Role: design-advisor | risk-advisor | repo-truth-advisor | critique-advisor
- Question:
- Governing sources: <ADR/module/ReadyItem/docs paths>
- Repo truth to inspect: <files/commands; read-only unless explicitly stated>
- Non-goals:
- Output: recommendation, evidence paths, risks, open questions
```

Do not include executor reasoning or preferred answers. Advisors must cite disk
truth and distinguish confirmed facts from judgement.

## Invocation

```text
/dev-loop-core-codex complete <ITEM>
/dev-loop-core-codex goal "..."
/dev-loop-core-codex resume <ITEM>
```

Equivalent to invoking `dev-loop-core` with this binding active. If the user
invokes `dev-loop-core` inside Codex without this skill, still apply these
mapping rules.
