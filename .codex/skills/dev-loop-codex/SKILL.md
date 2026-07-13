---
name: dev-loop-codex
description: Execute the canonical dev-loop using Codex orchestration, subagents, advisory tasks, isolated workspaces, and independent verification. Use when running or testing dev-loop specifically in Codex, or when another harness delegates a development target to Codex.
---

# Codex Development Loop Binding

Load and obey `dev-loop` first. This binding maps its roles to Codex capabilities and must not weaken the canonical invariants.

- Keep the root agent as orchestrator.
- Use executor subagents only for bounded, independently owned work.
- Use advisor subagents for design, research, critique, and investigation without writes.
- Use fresh subagents for verification; pass specification, APS scope, base/head or diff, and required gates—never executor reasoning.
- Use parallel subagents only for dependency-independent work. Give each writer its own Worktrunk worktree or non-overlapping write ownership.
- Continue useful local orchestration while subagents work, then reconcile results against Git and fresh evidence rather than trusting summaries.
- If subagents or durable background execution are unavailable, degrade to sequential fresh contexts and record the limitation in the checkpoint.
- Keep completion in the active turn while safe in-scope work remains; do not stop at a plan or promise.

For high-risk verification, prefer a fresh verifier using a different available model or delegate to another harness. The root orchestrator alone owns claims, APS reconciliation, repair routing, PR state, and merge authority.
