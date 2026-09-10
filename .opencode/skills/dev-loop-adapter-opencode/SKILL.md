---
name: dev-loop-adapter-opencode
description: Native OpenCode choreography for the canonical agentic loop.
---

# OpenCode adapter

Load `agentic-loop`. Keep the primary OpenCode agent as lead and implementation owner by default.

- Implement coherent work directly unless delegation adds parallelism, specialist capability, context isolation, or deliberate model separation.
- Use specialised agents with explicit model and tool permissions for workers, advisors, and verification.
- Deny write/destructive tools to advisors and verifiers; constrain workers to their owned workspace/write surface.
- Use native provider-aware routing when provider identity and permissions are observable; record the effective provider/model.
- Fresh verification must start in a separate context and receive only the governing contract, immutable candidate, acceptance criteria, and required gates.
- Prefer native tasks/sessions. Use supervised headless `opencode run` only as a policy-approved fallback when native supervision cannot satisfy a required boundary.
- After a child or bounded worker returns, continue the invocation target in this session. If the session cannot continue, write the run checkpoint and emit an explicit resume handle; never end the turn empty.
- Primary lead alone owns APS transitions, repair routing, PR lifecycle, and merge authority.

Different agent names on the same provider do not satisfy cross-provider independence.
