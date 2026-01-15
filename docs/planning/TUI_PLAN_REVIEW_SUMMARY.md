# TUI Plan Review Summary

**Date:** 2025-12-26  
**Branch:** `review-tui-plan`  
**Status:** Ready for Review

## Overview

This document provides a comprehensive plan for implementing a Terminal User
Interface (TUI) for Anvil, enhancing the developer experience with interactive,
real-time feedback during validation and gate execution.

## What Was Created

### 1. Module Plan: `plans/modules/tui.aps.md` (438 lines)

**Key sections:**

- **Overview** — Current state vs target state
- **Problem Statement** — Developer pain points and success criteria
- **Solution Architecture** — Component design with mermaid diagram
- **Implementation Phases** — 5 sprints of work clearly defined
- **Technical Decisions** — Ink vs Blessed, opt-in strategy, package separation
- **Dependencies** — New packages required (Ink, React)
- **Testing Strategy** — Unit, integration, and manual testing approach
- **Risks & Mitigations** — Bundle size, performance, compatibility concerns
- **Success Metrics** — Measurable targets for adoption and performance

**Highlights:**

- Uses [Ink](https://github.com/vadimdemedes/ink) (React for CLI) for component
  architecture
- Graceful degradation to basic CLI output when TTY unavailable
- Opt-in via `--tui` flag with auto-detection
- TUI module co-located under `apps/anvil-cli` for tighter integration

### 2. Execution Steps: `plans/execution/TUI-001.steps.md` (578 lines)

**Foundation & Infrastructure** — Sprint 1 (2-3 days)

Detailed implementation guide covering:

- Dependency installation (Ink, React, testing libraries)
- TypeScript configuration for JSX/TSX support
- TTY detection utility implementation
- Basic UI components (StatusBadge, ProgressBar, KeyboardShortcuts)
- Complete unit test suite
- Package exports and documentation

**Key deliverables:**

- Functional TUI module under `apps/anvil-cli`
- TTY detection logic with auto-fallback
- Reusable component library
- 100% test coverage for utilities

### 3. Execution Steps: `plans/execution/TUI-002.steps.md` (524 lines)

**Watch Dashboard Implementation** — Sprint 2 (3-4 days)

Comprehensive guide for building the interactive watch mode dashboard:

- State management architecture
- Real-time dashboard component with keyboard shortcuts
- History tracking (last 10 runs)
- Summary statistics (pass rate, avg duration)
- CLI integration with `anvil watch --tui`
- Integration tests

**Key deliverables:**

- Interactive watch mode dashboard
- Real-time status updates
- Keyboard shortcuts (q=quit, r=run now, c=clear history)
- Graceful fallback to basic output

### 4. Index Updates: `plans/index.aps.md`

**Changes:**

- Added TUI module to modules table (Draft status, Medium priority)
- Added TUI-001 and TUI-002 tasks to task status table (Planned status)

## Architecture Decisions

### D-001: Ink over Blessed

**Chosen:** [Ink](https://github.com/vadimdemedes/ink) (React-based TUI library)

**Rationale:**

- React-based API familiar to frontend developers
- Better TypeScript support and active maintenance
- Composable components via JSX
- Easier testing with React Testing Library
- Modern architecture vs Blessed's older patterns

**Trade-off:** Need to build some widgets ourselves vs Blessed's built-ins
(acceptable — keeps bundle small and gives us control)

### D-002: Opt-in via Auto-detection

**Chosen:** Auto-enable TUI when TTY available, `--no-tui` to disable

**Rationale:**

- Best experience by default for interactive developers
- Zero friction for those who want it
- Graceful fallback for CI/non-interactive contexts
- No breaking changes to existing scripts or workflows

**Trade-off:** Could surprise users expecting simple output (mitigated by clear
docs and obvious keyboard shortcuts)

### D-003: Co-locate with CLI

**Chosen:** Implement TUI inside `apps/anvil-cli`, not as a separate package

**Rationale:**

- Keeps TUI behaviour close to CLI command handling
- Avoids additional workspace package overhead
- Simplifies dependency management for Ink

**Trade-off:** Larger CLI package footprint and less isolation for future web UI
reuse

## Implementation Roadmap

### Phase 1: Foundation (Sprint 1, TUI-001)

- Set up Ink infrastructure
- Implement TTY detection
- Build basic reusable components
- **Outcome:** TUI module under `apps/anvil-cli` ready for use

### Phase 2: Watch Dashboard (Sprint 2, TUI-002)

- Real-time watch mode dashboard
- History tracking and statistics
- Keyboard shortcuts
- **Outcome:** `anvil watch --tui` fully functional

### Phase 3: Gate Explorer (Sprint 3)

- Interactive gate result exploration
- Keyboard navigation through check results
- Failure filtering and detail drill-down
- **Outcome:** `anvil gate --tui` with interactive explorer

### Phase 4: Progress Visualisation (Sprint 4)

- Real-time progress bars for parallel checks
- ETA calculation and cache indicators
- Verbose mode with check details
- **Outcome:** Visual progress during long-running gates

### Phase 5: Polish & Refinement (Sprint 5)

- Log panel with search and filtering
- Colour scheme configuration
- Copy-to-clipboard support
- Performance optimisation (<100ms updates)
- **Outcome:** Production-ready TUI with all features

## Dependencies Required

### New Dependencies for anvil-cli TUI

```json
{
  "dependencies": {
    "ink": "^5.0.1",
    "ink-spinner": "^5.0.0",
    "ink-text-input": "^6.0.0",
    "react": "^19.0.0"
  },
  "devDependencies": {
    "@types/react": "^19.0.0",
    "ink-testing-library": "^4.0.0"
  }
}
```

**Bundle size impact:** ~500KB (ink + react) — acceptable for optional feature

### CLI Integration

TUI components live under `apps/anvil-cli/src/tui`, so no workspace dependency
is required.

## Success Metrics

| Metric                          | Target              |
| ------------------------------- | ------------------- |
| Watch mode UI update latency    | < 100ms             |
| Developer satisfaction (survey) | > 8/10              |
| Adoption rate (interactive use) | > 60% of developers |
| Bug reports (terminal issues)   | < 5 per sprint      |
| Fallback reliability            | 100% (never breaks) |

## Risks & Mitigations

| Risk                             | Impact | Likelihood | Mitigation                                  |
| -------------------------------- | ------ | ---------- | ------------------------------------------- |
| Ink adds significant bundle size | medium | high       | Make TUI optional, lazy-load when needed    |
| Performance issues in watch mode | high   | medium     | Throttle updates, optimise renders (<100ms) |
| Terminal compatibility issues    | medium | medium     | Extensive testing, fallback to basic output |
| Breaks existing CI scripts       | high   | low        | Auto-detection, `--no-tui` flag             |
| Learning curve for contributors  | low    | high       | Good docs, React is widely known            |

## Alignment with Project Goals

### Roadmap Alignment

From `docs/planning/ROADMAP.md`:

- **Horizon 4: IDE Integration** — TUI enhances developer experience alongside
  VS Code extension
- **v0.5.0 milestone** — TUI targets this release (IDE & Experience Enhancement)

### Save-time Trust Alignment

From `plans/index.aps.md`:

- **Core thesis:** Make AI-generated code safe by providing actionable feedback
  at file-save time
- **TUI enhancement:** Real-time dashboard shows warnings immediately, improving
  feedback loop
- **Watch mode integration:** Perfect match for continuous validation during
  development

## Open Questions for Review

1. **Bundle Size** — Is ~500KB acceptable for an optional feature? Alternative:
   lazy-load TUI components
2. **Mouse Support** — Should we support mouse clicks? (Plan says no —
   keyboard-first)
3. **Custom Colour Schemes** — Should `.anvilrc` support colour theme
   configuration?
4. **Accessibility** — How do we handle screen reader users? (Fallback to basic
   output?)
5. **Priority vs Other Work** — Medium priority — should this be higher given
   developer experience impact?
6. **Web Dashboard** — Should TUI components be designed for later reuse in web
   UI?

## Next Steps

### Immediate Actions

1. **Review this plan** — Gather feedback from team/stakeholders
2. **Validate architecture** — Ensure Ink choice aligns with long-term vision
3. **Resource allocation** — Assign Sprint 1 & 2 to developer(s)
4. **Documentation** — Create `docs/TUI_GUIDE.md` skeleton

### Implementation Order

1. **TUI-001** (Sprint 1) — Foundation and infrastructure
2. **TUI-002** (Sprint 2) — Watch dashboard (highest value)
3. **TUI-003** (Sprint 3) — Gate explorer
4. **TUI-004** (Sprint 4) — Progress visualisation
5. **TUI-005** (Sprint 5) — Polish and refinement

## Files Changed

```
plans/modules/tui.aps.md              +438 lines (new file)
plans/execution/TUI-001.steps.md      +578 lines (new file)
plans/execution/TUI-002.steps.md      +524 lines (new file)
plans/index.aps.md                    +3 lines (module & tasks added)
```

## References

- [Ink Documentation](https://github.com/vadimdemedes/ink)
- [React Testing Library for Ink](https://github.com/vadimdemedes/ink-testing-library)
- Similar TUI projects: [k9s](https://k9scli.io/),
  [lazygit](https://github.com/jesseduffield/lazygit)
- Current CLI implementation: `cli/src/commands/watch.ts`,
  `cli/src/commands/gate.ts`
