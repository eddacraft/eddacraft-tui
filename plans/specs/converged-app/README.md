# APS and anvil Converged Application Documentation Pack

This pack captures the current product and architecture direction for converging APS and anvil into one app-layer platform with native desktop, web, tray, CLI, and TUI surfaces.

## Documents

1. **[Converged app-layer requirements](01-converged-app-layer-requirements.md)**  
   The authoritative requirements document covering product goals, daemon, planning, workspaces, runs, sessions, governance, evidence, native, web, tray, CLI/TUI, security, reliability, and the standalone APS distribution.

2. **[Domain, command, event, and projection model](02-domain-command-and-projection-model.md)**  
   A framework-neutral domain model defining projects, APS work, workspaces, runs, sessions, evidence, approvals, commands, events, projections, capabilities, and repository authority.

3. **[Surface and experience architecture](03-surface-and-experience-architecture.md)**  
   The intended user experience across native desktop, web, tray, CLI, TUI, and agent APIs, including the operational sidebar, board, workspaces, tabs, panes, terminal, diff, and cross-surface continuity.

4. **[Convergence and migration plan](04-convergence-and-migration-plan.md)**  
   A phased path for importing APS into the anvil monorepo, removing duplicate implementations, establishing the daemon app layer, building the first native slice, and publishing the public APS source mirror.

5. **[Decisions, risks, and open questions](05-decisions-risks-and-open-questions.md)**  
   The working decisions, ADR candidates, principal risks, mitigations, and questions that should remain open until spike evidence exists.

## Core direction

```text
APS defines and authorises intended work
anvil governs, observes, verifies, and records execution
The daemon owns durable local behaviour and runtime state
Native, web, tray, CLI, TUI, and agent surfaces share one app layer
Standalone APS is extracted and published from the canonical monorepo
```

## Recommended immediate sequence

```text
canonical APS import
→ shared planning application layer
→ commands, capabilities, and projections
→ daemon integration
→ native framework spike decision
→ app shell and tray
→ APS board/workspace vertical slice
→ run/session depth
→ evidence and governance
→ web surface
→ public APS mirror cut-over
```
