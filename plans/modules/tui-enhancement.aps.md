<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# TUI Enhancement

| Scope | Owner | Priority | Status     |
| ----- | ----- | -------- | ---------- |
| TUI   | —     | high     | Superseded |

> **Superseded** by [RATS — Ratatui TUI](./ratatui-tui.aps.md) and the
> [Architecture Evolution](../../docs/architecture/anvil-architecture-evolution.md)
> document. The OpenTUI/Bun-based approach described here has been replaced by a
> Ratatui TUI running in the same process as the Rust kernel. See also D-005:
> Ink over OpenTUI (which itself is superseded by the Rust stack decision).

## Purpose

Provide rich terminal UI (TUI) components and commands that remove friction from
the first-time user experience and ongoing developer workflow. Makes Anvil's
value clear within 30 seconds of installation while maintaining CLI-first
philosophy with graceful degradation.

## In Scope

- OpenTUI integration with React renderer
- Reusable TUI component library (layout, display, input, feedback)
- `anvil init` wizard (mode selection, format choice, hooks preview)
- `anvil status` dashboard (hooks status, repo profile, recent results)
- `anvil doctor` diagnostics (system checks, auto-fix suggestions)
- First-run welcome experience
- Static template library (10-15 curated plan templates)
- Interactive tutorial for onboarding
- TTY detection with automatic CLI fallback

## Out of Scope

- AI-powered features (deferred to V2)
- Web-based UI or dashboard
- Persistent TUI daemon
- Custom TUI framework (using OpenTUI)
- Mobile/tablet support

## Interfaces

**Depends on:**

- `@opentui/core`, `@opentui/react` — TUI framework with React renderer
- `react`, `react-dom` — React for component composition
- Existing CLI commands (`anvil validate`, `anvil gate`, `anvil hooks`)
- Existing configuration system (`.anvilrc`)
- Bun runtime for TUI execution

**Exposes:**

- `renderTUI(Component, props)` — TUI rendering utility
- `isTUIAvailable()` — TTY detection helper
- Reusable component library for future TUI commands
- Template API for programmatic access

## Boundary Rules

- TUI components must gracefully degrade to CLI when TTY unavailable
- TUI code must not pollute core CLI logic (separate `cli/src/tui/` directory)
- All TUI commands must support `--no-tui` flag
- Templates must be static markdown (no AI generation)
- Components must be testable without real terminal

## Acceptance Criteria

- [ ] `anvil init` wizard completes setup in < 60 seconds
- [ ] First-run experience shows value proposition clearly
- [ ] Template library includes 10+ curated templates
- [ ] All TUI commands work in non-TTY environments (CI, SSH)
- [ ] Component test coverage > 90%
- [ ] TUI commands respect `NO_TUI=1` environment variable
- [ ] Tutorial can be completed in < 5 minutes
- [ ] Works on Linux, macOS, Windows terminals

## Risks & Mitigations

| Risk                          | Mitigation                                   |
| ----------------------------- | -------------------------------------------- |
| OpenTUI adoption risk         | Small, focused framework; TypeScript-native  |
| Terminal compatibility issues | Extensive cross-platform testing; CLI fallback |
| Template maintenance burden   | Start small (10-15); community contributions |
| TUI adds complexity           | Strict separation; `--no-tui` always works   |
| First-run value unclear       | User testing; iterate on messaging           |

## Tasks

### TUI-001: OpenTUI foundation and component library

- **Intent:** Set up OpenTUI with React and create reusable component library
- **Expected Outcome:** Working TUI renderer with 15+ tested components (Box,
  Container, Header, Select, Spinner, etc.)
- **Scope:** `cli/src/tui/components/`, `cli/src/tui/utils/`
- **Non-scope:** Actual command implementations
- **Files:**
  - `cli/src/tui/components/layout/Box.tsx`
  - `cli/src/tui/components/layout/Container.tsx`
  - `cli/src/tui/components/layout/Grid.tsx`
  - `cli/src/tui/components/display/Header.tsx`
  - `cli/src/tui/components/display/InfoPanel.tsx`
  - `cli/src/tui/components/display/List.tsx`
  - `cli/src/tui/components/display/Table.tsx`
  - `cli/src/tui/components/input/Select.tsx`
  - `cli/src/tui/components/input/Checkbox.tsx`
  - `cli/src/tui/components/input/TextInput.tsx`
  - `cli/src/tui/components/feedback/Spinner.tsx`
  - `cli/src/tui/components/feedback/StatusMessage.tsx`
  - `cli/src/tui/components/feedback/ProgressBar.tsx`
  - `cli/src/tui/utils/renderer.ts`
  - `cli/src/tui/utils/colors.ts`
  - `cli/src/tui/utils/icons.ts`
- **Dependencies:** —
- **Validation:** `nx test cli --testNamePattern="TUI components"`
- **Confidence:** medium

### TUI-002: `anvil init` wizard

- **Intent:** Create interactive setup wizard with mode selection, format choice,
  and git hooks preview
- **Expected Outcome:** Step-by-step TUI wizard that generates `.anvilrc`,
  installs hooks with preview, and completes setup in < 60 seconds
- **Scope:** `cli/src/tui/commands/init/`
- **Non-scope:** Existing `anvil init` CLI behavior (keep as fallback)
- **Files:**
  - `cli/src/tui/commands/init/InitWizard.tsx`
  - `cli/src/tui/commands/init/steps/ModeStep.tsx`
  - `cli/src/tui/commands/init/steps/FormatStep.tsx`
  - `cli/src/tui/commands/init/steps/DirectoryStep.tsx`
  - `cli/src/tui/commands/init/steps/HooksStep.tsx`
  - `cli/src/tui/commands/init/steps/SummaryStep.tsx`
  - `cli/src/commands/init.ts` (update to use TUI when available)
- **Dependencies:** TUI-001
- **Validation:** `anvil init` in terminal, `NO_TUI=1 anvil init` for fallback
- **Confidence:** high

### TUI-003: `anvil status` dashboard

- **Intent:** Create live dashboard showing hooks status, repo profile, and
  recent validation results
- **Expected Outcome:** Interactive TUI dashboard with keyboard navigation (j/k,
  q to quit), showing system state at a glance
- **Scope:** `cli/src/tui/commands/status/`
- **Non-scope:** Historical analytics, persistent daemon
- **Files:**
  - `cli/src/tui/commands/status/StatusDashboard.tsx`
  - `cli/src/tui/commands/status/panels/HooksPanel.tsx`
  - `cli/src/tui/commands/status/panels/ProfilePanel.tsx`
  - `cli/src/tui/commands/status/panels/ResultsPanel.tsx`
  - `cli/src/commands/status.ts` (new command)
- **Dependencies:** TUI-001
- **Validation:** `anvil status`, verify keyboard navigation, quit behavior
- **Confidence:** medium

### TUI-004: `anvil doctor` diagnostics

- **Intent:** Create diagnostic wizard that checks system requirements, config,
  hooks, and offers auto-fix
- **Expected Outcome:** Comprehensive health check with actionable fixes, one-click
  resolution for common issues
- **Scope:** `cli/src/tui/commands/doctor/`
- **Non-scope:** Advanced system troubleshooting, remote diagnostics
- **Files:**
  - `cli/src/tui/commands/doctor/Diagnostics.tsx`
  - `cli/src/tui/commands/doctor/checks/SystemCheck.tsx`
  - `cli/src/tui/commands/doctor/checks/ConfigCheck.tsx`
  - `cli/src/tui/commands/doctor/checks/HooksCheck.tsx`
  - `cli/src/tui/commands/doctor/checks/PermissionsCheck.tsx`
  - `cli/src/commands/doctor.ts` (new command)
- **Dependencies:** TUI-001
- **Validation:** `anvil doctor`, test auto-fix suggestions, verify exit codes
- **Confidence:** medium

### TUI-005: First-run experience and welcome screen

- **Intent:** Detect first run and show value proposition with quick start guide
- **Expected Outcome:** Friendly welcome screen on first `anvil` command that
  explains value in 30 seconds
- **Scope:** `cli/src/tui/commands/welcome/`
- **Non-scope:** Persistent onboarding state, analytics
- **Files:**
  - `cli/src/tui/commands/welcome/Welcome.tsx`
  - `cli/src/services/first-run-detector.ts`
  - `cli/src/index.ts` (update to show welcome on first run)
- **Dependencies:** TUI-001
- **Validation:** Fresh install test, verify welcome shows once
- **Confidence:** high

### TUI-006: Static template library

- **Intent:** Create curated library of 10-15 plan templates (JWT auth, REST API,
  etc.) with browser UI
- **Expected Outcome:** `anvil new` command launches template browser, variable
  substitution works
- **Scope:** `cli/templates/`, `cli/src/tui/commands/new/`
- **Non-scope:** AI generation, custom template creation UI
- **Files:**
  - `cli/templates/authentication-jwt.md`
  - `cli/templates/rest-api-crud.md`
  - `cli/templates/database-migration.md`
  - `cli/templates/frontend-component.md`
  - `cli/templates/websocket-realtime.md`
  - `cli/templates/file-upload.md`
  - `cli/templates/caching-layer.md`
  - `cli/templates/api-integration.md`
  - `cli/templates/testing-suite.md`
  - `cli/templates/ci-cd-pipeline.md`
  - `cli/src/tui/commands/new/TemplateBrowser.tsx`
  - `cli/src/services/template-loader.ts`
  - `cli/src/commands/new.ts` (new command)
- **Dependencies:** TUI-001
- **Validation:** `anvil new`, select template, verify substitution
- **Confidence:** high

### TUI-007: Interactive tutorial

- **Intent:** Step-by-step walkthrough for new users covering validation, gates,
  and hooks
- **Expected Outcome:** 5-minute tutorial that creates sample plan, validates,
  runs gates, shows value
- **Scope:** `cli/src/tui/commands/tutorial/`
- **Non-scope:** Video content, external documentation
- **Files:**
  - `cli/src/tui/commands/tutorial/Tutorial.tsx`
  - `cli/src/tui/commands/tutorial/steps/IntroStep.tsx`
  - `cli/src/tui/commands/tutorial/steps/PlanStep.tsx`
  - `cli/src/tui/commands/tutorial/steps/ValidateStep.tsx`
  - `cli/src/tui/commands/tutorial/steps/GateStep.tsx`
  - `cli/src/tui/commands/tutorial/steps/CompletionStep.tsx`
  - `cli/src/commands/tutorial.ts` (new command)
- **Dependencies:** TUI-001, TUI-006 (uses sample template)
- **Validation:** `anvil tutorial`, complete all steps, verify progress tracking
- **Confidence:** medium

### TUI-008: Testing infrastructure and cross-platform QA

- **Intent:** Set up component testing, E2E tests, and verify cross-platform
  compatibility
- **Expected Outcome:** > 90% test coverage, all TUI commands work on Linux,
  macOS, Windows
- **Scope:** `cli/src/tui/**/*.test.tsx`, E2E tests
- **Non-scope:** Performance benchmarking, accessibility testing (future)
- **Files:**
  - `cli/src/tui/components/**/*.test.tsx`
  - `cli/src/tui/commands/**/*.test.tsx`
  - `cli/vitest.config.tui.ts`
  - `e2e/tui-commands.spec.ts`
- **Dependencies:** TUI-001 through TUI-007
- **Validation:** `nx test cli`, `nx test:e2e cli`, manual cross-platform testing
- **Confidence:** high

## Decisions

- **D-001:** Use OpenTUI over ink/blessed — TypeScript-native, React renderer,
  built-in components (`<diff>`, `<code>`, `<select>`)
- **D-002:** Bun runtime for TUI — faster startup, native TypeScript support
- **D-003:** Static templates only for V1 — achieves 90% of AI benefit without
  complexity
- **D-004:** Graceful degradation always — every TUI command must work as CLI
  fallback

## Notes

- OpenTUI repo: https://github.com/sst/opentui
- Template inspiration: GitHub issue templates, Yeoman generators
- First-run detection: check for `.anvil/first-run` marker file
- Consider GIF demos in docs for TUI commands (asciinema)

## Execution

See [TUI Implementation Plan](../../docs/plans/TUI-IMPLEMENTATION-PLAN.md) for
detailed step-by-step implementation guide with code examples.
