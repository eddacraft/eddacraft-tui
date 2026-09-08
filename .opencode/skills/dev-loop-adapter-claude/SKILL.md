---
name: dev-loop-adapter-claude
description: Native Claude Code choreography for the canonical agentic loop.
---

# Claude Code adapter

Load `agentic-loop`. Keep the main Claude session as lead and implementation owner by default.

- Implement coherent work directly unless subagent/team delegation provides a concrete benefit.
- Use subagents for bounded workers, advisors, specialists, and fresh verification; use teams only for genuinely independent coordinated streams.
- Give each writing teammate an isolated worktree/write surface.
- Use model/effort selection deliberately from APS `reasoning`/policy; do not force the lead into architect-only behaviour.
- The verifier receives the governing contract and immutable candidate, not the implementation transcript, and must be read-only with respect to implementation.
- Require finite supervision/cancellation capability for mandatory verification. Prefer native subagent/task controls; supervised headless `claude -p` is a fallback, not the canonical default.
- Advisor features that inherit the full conversation are implementation/design assistance, not blind verification.
- Lead alone owns APS transitions, repair routing, PR lifecycle, and merge authority.

For differential work, external Claude-hosted bridges may be used only when they prove the required read/write boundary and provider identity. Transcript transfer never counts as independent verification.
