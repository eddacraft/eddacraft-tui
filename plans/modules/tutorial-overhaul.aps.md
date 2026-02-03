<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Tutorial & Onboarding Overhaul

| Scope | Owner  | Priority | Status |
| ----- | ------ | -------- | ------ |
| TUT   | @aneki | high     | Ready  |

## Purpose

Restructure Anvil's tutorial system so a new user's first experience is:
install → scan their project → see what's wrong → turn on watcher → see it
catch something live. Then provide focused feature tutorials for policies,
architecture boundaries, drift tracking, and CI integration.

The current `anvil tutorial` teaches Plans → Validate → Gates (the APS
workflow), which is an advanced concept. The highest-impact first experience is
seeing Anvil analyse your own code and catching something in real time.

## Problem Statement

1. **Interactive tutorial teaches the wrong thing first.** The TUI tutorial
   walks through APS plans/validate/gates — abstract concepts with sample
   files. New users want to see what's wrong with their code right now.

2. **No "scan my project" moment.** The biggest hook for adoption is "here's
   what's already wrong" (like running eslint for the first time). No single
   guided flow delivers this.

3. **Quickstart has the right flow but isn't interactive.** The docs-site
   quickstart (init → check → watch) matches the ideal path, but it's
   read-only markdown. Users copy-paste between docs and terminal.

4. **No feature tutorials.** Policies (OPA/Rego), architecture boundaries,
   drift tracking, and CI integration have reference docs but no hands-on
   guided tutorials.

5. **Disconnected surfaces.** Five different documents teach overlapping things
   in different ways. A new user doesn't know which to use.

## Success Criteria

- [ ] `anvil tutorial` scans the user's actual project and shows real findings
- [ ] Time to first "wow" moment < 90 seconds (scan shows real issues)
- [ ] `anvil tutorial policies` guides user through creating a working policy
- [ ] `anvil tutorial architecture` guides user through defining boundaries
- [ ] `anvil tutorial --list` shows all available tutorials
- [ ] docs-site has a Tutorials section with standalone feature guides
- [ ] quickstart.md follows scan-watch-fix flow

## In Scope

- Rewriting the `anvil tutorial` TUI flow (core 4 steps)
- Creating interactive feature tutorials (policies, architecture, drift, CI)
- Creating docs-site written tutorials (policies, architecture, drift, CI, suppressions)
- Updating quickstart.md and first-project.md
- Tutorial discovery (`--list` flag)
- End-to-end test for core tutorial flow

## Out of Scope

- New CLI commands (all commands already exist: check, watch, policy, architecture, drift)
- Changes to the analysis engine (anvil-runtime, anvil-core)
- VS Code extension tutorial (separate module)
- APS/Kindling tutorials (separate products)
- Website marketing changes

## Interfaces

**Depends on:**

- `@eddacraft/anvil-runtime` — GateRunner, watch orchestrator, policy loader
- `@eddacraft/anvil-core` — snapshot, architecture compilation
- `apps/anvil-cli` — existing TUI components, theme, renderer
- `apps/docs-site` — Docusaurus sidebar, MDX support

**Exposes:**

- Rewritten `anvil tutorial` command (core flow)
- `anvil tutorial <feature>` subcommands (policies, architecture, drift, ci)
- `anvil tutorial --list` discovery flag
- `apps/docs-site/docs/anvil/tutorials/` documentation section

## Boundary Rules

- Tutorial code must not modify the analysis engine
- Feature tutorials must use existing CLI commands, not internal APIs
- Docs tutorials must be independently readable (no dependency on CLI tutorial)
- All TUI components must gracefully fall back when TTY unavailable

## Risks & Mitigations

| Risk                                  | Mitigation                                     |
| ------------------------------------- | ---------------------------------------------- |
| Scan finds nothing in clean projects  | FixStep falls back to simulated example         |
| Watch step blocks if user doesn't act | 30s timeout with hint, skip option              |
| OPA not installed for policy tutorial | Tutorial checks for OPA, offers install guidance|
| Feature tutorials too long            | Each capped at 5 minutes, skip-ahead supported  |

## Tasks

### TUT-001: Rewrite tutorial step types for scan-watch-fix flow

- **Intent:** Tutorial state and step definitions reflect the new Scan → Watch → Fix → Next Steps progression
- **Expected Outcome:** New step IDs, ordering, and state shape replace old plan-centric definitions
- **Scope:** `apps/anvil-cli/src/tui/commands/tutorial/types.ts`
- **Non-scope:** TUI rendering, step implementations
- **Validation:** `nx test anvil-cli -- --testPathPattern tutorial`
- **Confidence:** high

---

### TUT-002: Create ScanStep TUI component

- **Intent:** First tutorial step scans user's actual project and shows real findings
- **Expected Outcome:** ScanStep runs GateRunner.analyzeFiles, displays warning count, file count, top 3 examples
- **Scope:** `apps/anvil-cli/src/tui/commands/tutorial/steps/ScanStep.tsx`
- **Non-scope:** Analysis engine changes
- **Dependencies:** TUT-001
- **Validation:** `nx test anvil-cli -- --testPathPattern scan-step`
- **Confidence:** high

---

### TUT-003: Create WatchStep TUI component

- **Intent:** Second tutorial step demonstrates real-time watch mode on user's project
- **Expected Outcome:** WatchStep starts file watcher, detects user edit, shows live validation result
- **Scope:** `apps/anvil-cli/src/tui/commands/tutorial/steps/WatchStep.tsx`
- **Non-scope:** Watch orchestrator changes
- **Dependencies:** TUT-001
- **Validation:** `nx test anvil-cli -- --testPathPattern watch-step`
- **Confidence:** medium

---

### TUT-004: Create FixStep TUI component

- **Intent:** Third tutorial step guides user through fixing one real issue
- **Expected Outcome:** FixStep shows a specific warning, explains fix, detects when resolved
- **Scope:** `apps/anvil-cli/src/tui/commands/tutorial/steps/FixStep.tsx`
- **Non-scope:** New anti-pattern rules
- **Dependencies:** TUT-001, TUT-002
- **Validation:** `nx test anvil-cli -- --testPathPattern fix-step`
- **Confidence:** medium

---

### TUT-005: Create NextStepsStep and wire up Tutorial.tsx

- **Intent:** Final step shows feature tutorials and resources; Tutorial.tsx renders new flow
- **Expected Outcome:** Complete working tutorial with 4 new steps replacing 5 old steps
- **Scope:** `apps/anvil-cli/src/tui/commands/tutorial/`
- **Non-scope:** Feature tutorial implementations
- **Dependencies:** TUT-002, TUT-003, TUT-004
- **Validation:** `nx test anvil-cli -- --testPathPattern tutorial`
- **Confidence:** high

---

### TUT-006: Interactive policy creation tutorial

- **Intent:** Users can learn to write custom OPA/Rego policies through guided interactive tutorial
- **Expected Outcome:** `anvil tutorial policies` walks through creating, testing, and triggering a policy
- **Scope:** `apps/anvil-cli/src/tui/commands/tutorial/features/`
- **Non-scope:** OPA engine changes, new policy templates
- **Dependencies:** TUT-005
- **Validation:** `anvil tutorial policies` completes without error
- **Confidence:** medium

---

### TUT-007: Interactive architecture boundaries tutorial

- **Intent:** Users can learn to define and enforce architecture boundaries through guided tutorial
- **Expected Outcome:** `anvil tutorial architecture` walks through template selection, compilation, validation
- **Scope:** `apps/anvil-cli/src/tui/commands/tutorial/features/`
- **Non-scope:** Architecture analysis changes
- **Dependencies:** TUT-005
- **Validation:** `anvil tutorial architecture` completes without error
- **Confidence:** medium

---

### TUT-008: Interactive drift tracking tutorial

- **Intent:** Users can learn to capture and compare architecture snapshots
- **Expected Outcome:** `anvil tutorial drift` walks through snapshot, change, comparison flow
- **Scope:** `apps/anvil-cli/src/tui/commands/tutorial/features/`
- **Non-scope:** Drift engine changes
- **Dependencies:** TUT-005
- **Validation:** `anvil tutorial drift` completes without error
- **Confidence:** medium

---

### TUT-009: Interactive CI integration tutorial

- **Intent:** Users can set up Anvil in their CI pipeline through guided tutorial
- **Expected Outcome:** `anvil tutorial ci` detects CI system, generates config, explains exit codes
- **Scope:** `apps/anvil-cli/src/tui/commands/tutorial/features/`
- **Non-scope:** New CI integrations
- **Dependencies:** TUT-005
- **Validation:** `anvil tutorial ci` completes without error
- **Confidence:** high

---

### TUT-010: Docs-site tutorials section

- **Intent:** Standalone written tutorials available on the documentation site
- **Expected Outcome:** Tutorials section with guides for policies, architecture, drift, CI, suppressions
- **Scope:** `apps/docs-site/docs/anvil/tutorials/`
- **Non-scope:** Marketing website, concept docs rewrites
- **Dependencies:** TUT-006, TUT-007, TUT-008, TUT-009
- **Validation:** `nx build docs-site` succeeds, tutorials render in sidebar
- **Confidence:** high

---

### TUT-011: Rewrite quickstart.md and update navigation

- **Intent:** Quickstart reflects scan-watch-fix flow; docs navigation includes tutorials
- **Expected Outcome:** Updated quickstart, sidebar with Tutorials section, cross-links
- **Scope:** `apps/docs-site/docs/anvil/`
- **Non-scope:** Other product docs (APS, Kindling)
- **Dependencies:** TUT-010
- **Validation:** `nx build docs-site` succeeds, quickstart follows new flow
- **Confidence:** high

---

### TUT-012: Tutorial --list flag and e2e test

- **Intent:** Users can discover available tutorials; core flow has e2e coverage
- **Expected Outcome:** `anvil tutorial --list` shows all tutorials; e2e test passes
- **Scope:** `apps/anvil-cli/src/commands/tutorial.ts`, `apps/e2e/`
- **Non-scope:** Feature tutorial e2e tests
- **Dependencies:** TUT-005, TUT-006, TUT-007, TUT-008, TUT-009
- **Validation:** `anvil tutorial --list` outputs tutorial list; `nx e2e e2e -- --testPathPattern tutorial`
- **Confidence:** high

## Decisions

- **D-001:** Core tutorial uses user's actual project code, not sample files —
  the scan-your-own-code moment is the primary adoption hook
- **D-002:** Feature tutorials are TUI-interactive, not just docs — learning by
  doing beats reading
- **D-003:** APS plan tutorial moves from core flow to a future `anvil tutorial plans`
  feature tutorial — plans are a power-user concept, not first-contact material

## Notes

- The existing tutorial step components (IntroStep, PlanStep, ValidateStep,
  GateStep, CompletionStep) are replaced, not modified. Clean break.
- Policy tutorial depends on OPA binary being available — tutorial should check
  and guide installation if missing.
- All tutorials should support `--reset` to restart from scratch.
