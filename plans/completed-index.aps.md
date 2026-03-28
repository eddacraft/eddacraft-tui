<!-- APS: Completed Work Archive — read-only record of all shipped work -->
<!-- This document is non-executable. It archives completed tasks, milestones, and modules. -->

# Anvil — Completed Work

## Overview

Anvil makes AI-generated code safe to merge by catching architecture boundary
violations and AI escape-hatch anti-patterns at file-save time. Developers get
actionable warnings before code leaves the file, with human-owned exceptions for
intentional deviations.

**Why this matters:** AI coding tools are accelerating development, but they
don't understand your architecture. They produce code that compiles and passes
tests, yet drifts from intended patterns. By the time drift is noticed in
review, it's already merged or too expensive to fix. Anvil catches it at the
moment of creation — when fixing is cheap.

**Product thesis:** Anvil improves trust in AI-generated code so more of it
reaches production faster, while architecture drift slows or reverses over time.

**Primary beneficiary:** Individual developers — they get to use AI safely at
the pace leadership expects.

## Release Plan

### 0.1.0 — Beta (Complete)

**Philosophy:** A powerful engine is worthless if no one uses it. The initial
release must deliver both the core value AND a friction-free first experience.

#### Core Engine

| Feature             | Description                                    | Status   |
| ------------------- | ---------------------------------------------- | -------- |
| Analysis Engine     | `anvil check <files>` with caching + parallel  | Complete |
| Architecture Safety | Baseline inference, new-edge detection          | Complete |
| Anti-patterns       | 7 high-confidence patterns                     | Complete |
| Suppressions        | Time-boxed with mandatory explanations          | Complete |
| Git Integration     | `--changed`, `--staged`, `--since <ref>`       | Complete |
| Watch Mode          | `anvil watch --source` for real-time feedback   | Complete |
| CI/CD               | GitHub Action with PR comments + status checks  | Complete |

#### Onboarding Experience

| Feature           | Description                                     | Status   |
| ----------------- | ----------------------------------------------- | -------- |
| TUI Foundation    | Ink setup + base components (TUI-001)           | Complete |
| Init Wizard       | Visual `anvil init` with guided flow (TUI-002)  | Complete |
| Status Dashboard  | Quick health check: `anvil status` (TUI-003)    | Complete |
| Doctor Command    | Diagnose setup issues: `anvil doctor` (TUI-004) | Complete |
| First-run Welcome | Show value immediately on first run (TUI-005)   | Complete |

#### Documentation & Polish

| Feature           | Description                     | Status   |
| ----------------- | ------------------------------- | -------- |
| Quick Start Guide | 5-minute path to first value    | Complete |
| User Guide        | Complete command reference       | Complete |
| Demo/Tutorial     | Show Anvil catching real issues  | Complete |
| Error Messages    | Actionable, not cryptic          | Complete |

#### Drift Visibility & Developer Trust

| Feature                | Description                                    | Status   |
| ---------------------- | ---------------------------------------------- | -------- |
| Explain Command        | `anvil explain <id>` — deep-dive into warnings | Complete |
| Drift Snapshots        | `anvil drift snapshot` — capture current state  | Complete |
| Drift Compare          | `anvil drift compare` — show changes over time  | Complete |
| Drift Reports          | `anvil drift report` — visualise trends         | Complete |
| OPA Architecture       | DC-OPA bridge, YAML-first architecture          | Complete |
| Architecture Templates | Layered, Hexagonal, Clean, DDD presets          | Complete |
| Remote Policy Bundles  | Centralised policy distribution                 | Complete |
| Monorepo Migration     | Restructure to apps/packages layered layout     | Complete |

#### AI Tool Integration

| Feature         | Description                                | Status   |
| --------------- | ------------------------------------------ | -------- |
| llms.txt Export | Export constraints for AI tool consumption | Complete |
| Command Safety  | Validate AI tool commands (CMDSAF)         | Complete |
| MCP Server      | Real-time validation during AI generation  | Complete |

#### HTML/CSS, Tutorial & First Run

| Feature                   | Description                                         | Status   |
| ------------------------- | --------------------------------------------------- | -------- |
| Configurable Extensions   | Make analysable file extensions configurable         | Complete |
| HTML Anti-patterns        | Inline styles, scripts, event handlers, deprecated  | Complete |
| CSS Anti-patterns         | `!important` abuse, CSS `@import` performance       | Complete |
| Tutorial Overhaul         | Scan-watch-fix flow, feature tutorials, docs        | Complete |
| Intelligent First Run     | Post-init analysis, smart defaults, quick wins      | Complete |

### 0.1.x — Completed Work

| Feature                    | Description                                              | Status   | Progress |
| -------------------------- | -------------------------------------------------------- | -------- | -------- |
| Forge Hook & Agent         | Pre-commit hook + reviewer agent with codex delegation   | Complete | —        |
| Forge Negotiation          | Structured finding/response protocol, round cap          | Complete | —        |
| Deferred Finding Filing    | Auto-file deferred findings as GH issues or APS items    | Complete | —        |
| Temper Workflow             | GitHub Actions self-healing loop with 2-cycle cap        | Complete | —        |
| Configuration & Docs       | Env vars, settings.json, CLAUDE.md, toggle matrix        | Complete | —        |
| CLI Hardening              | Error handling, edge cases, robustness                   | Complete | —        |
| Coaching Nudges            | Context-aware suggestions for pattern improvement        | Complete | —        |
| Nx Task Migration          | Migrate root scripts to Nx-orchestrated per-project      | Complete | 6/6      |
| CLI esbuild Bundling       | Self-contained npm package via esbuild                   | Complete | 3/3      |
| MCP Server Hardening       | Production-readiness for MCP server                      | Complete | —        |
| Security CI Pipeline       | Automated security scanning on every PR                  | Complete | —        |
| Tutorial Path Continuation | Continue with another tutorial from completion screen     | Complete | —        |
| Post-Beta Launch Uplift    | Address 57 findings from v0.1.2-beta post-release review | Complete | 57/57    |
| Code Review Backlog        | 29 architectural recommendations from code review        | Complete | 29/29    |
| Security Review Backlog    | Cross-package security findings from adversarial review  | Complete | 8/8      |

**Design doc (Forge & Temper):** [docs/plans/2026-02-24-forge-temper-review-pipeline.md](../docs/plans/2026-02-24-forge-temper-review-pipeline.md)

### 0.4.0 — Edda Stack (Memory System)

| Feature                | Description                                    | Status   |
| ---------------------- | ---------------------------------------------- | -------- |
| Kindling Integration   | Observation layer — session and gate hooks      | Complete |
| Ember                  | Interpretive layer — candidate memory proposals | Complete |
| Edda                   | Canonical memory — git-backed, provenance-tracked | Complete |
| Edda Stack Integration | Shared schemas, event bus, layer ports          | Complete |
| Edda-Ember Review      | Non-critical improvements from consolidated review | Complete |

### Future — Ratatui TUI (RATS, Done)

7/7 tasks complete. Full task table in [completed-index.aps.md](./completed-index.aps.md).
**Module:** [RATS — Ratatui TUI](./archive/modules/ratatui-tui.aps.md)

### Future — Ink-to-Ratatui Port (PORT, Done)

15/15 tasks complete. Full task table in [completed-index.aps.md](./completed-index.aps.md).
**Module:** [PORT — Ink-to-Ratatui Port](./archive/modules/ink-to-ratatui-port.aps.md)

## Milestones

### M1: Core Analysis Engine

- **Status:** Complete
- **Includes:** save-time-trust, architecture-safety
- **Delivered:** `anvil check <file>` returns warnings with explanations

### M2: Anti-pattern Detection

- **Status:** Complete
- **Includes:** antipattern-library
- **Delivered:** ESLint-disable, `any`, `@ts-ignore` detected in new code

### M3: Developer Ergonomics

- **Status:** Complete
- **Includes:** suppressions, drift-reporting
- **Delivered:** Developers can suppress with accountability; drift snapshots and reports

### M4: Integration Points

- **Status:** Complete
- **Includes:** ci-integration, ide-integration
- **Delivered:** PRs show warning summaries via GitHub Action; VS Code extension v0.1.0

## Modules

### Completed (0.1.0)

Task-level detail for all completed work is archived in
[completed.aps.md](./completed.aps.md).

| Module | Scope | Release |
| ------ | ----- | ------- |
| [save-time-trust](./archive/modules/save-time-trust.aps.md) | CORE | 0.1.0 |
| [architecture-safety](./archive/modules/architecture-safety.aps.md) | ARCH | 0.1.0 |
| [antipattern-library](./archive/modules/antipattern-library.aps.md) | ANTI | 0.1.0 |
| [suppressions](./archive/modules/suppressions.aps.md) | SUPP | 0.1.0 |
| [ci-integration](./archive/modules/ci-integration.aps.md) | CI | 0.1.0 |
| [tui](./archive/modules/tui.aps.md) | TUI | 0.1.0 |
| [documentation-polish](./archive/modules/documentation-polish.aps.md) | DOCS | 0.1.0 |
| [explain-command](./archive/modules/explain-command.aps.md) | EXPLAIN | 0.1.0 |
| [drift-reporting](./archive/modules/drift-reporting.aps.md) | DRIFT | 0.1.0 |
| [opa-architecture-integration](./archive/modules/opa-architecture-integration.aps.md) | OPA | 0.1.0 |
| [ide-integration](./archive/modules/ide-integration.aps.md) | IDE | 0.1.0 |
| [llms-txt-export](./archive/modules/llms-txt-export.aps.md) | LLMS | 0.1.0 |
| [command-safety-validation](./archive/modules/command-safety-validation.aps.md) | CMDSAF | 0.1.0 |
| [mcp-server](./archive/modules/mcp-server.aps.md) | MCP | 0.1.0 |
| [aps-markdown-adapter](./archive/modules/aps-markdown-adapter.aps.md) | APSMD | 0.1.0 |
| [adapter-upstream-updates](./archive/modules/adapter-upstream-updates.aps.md) | ADAPTUP | 0.1.0 |
| [onboarding-feedback-resolution](./archive/modules/onboarding-feedback-resolution.aps.md) | ONFBK | 0.1.0 |
| [html-css-support](./archive/modules/html-css-support.aps.md) | HTMLCSS | 0.1.0 |
| [intelligent-first-run](./archive/modules/intelligent-first-run.aps.md) | IFR | 0.1.0 |
| [tutorial-overhaul](./archive/modules/tutorial-overhaul.aps.md) | TUT | 0.1.0 |
| [tutorial-path-continuation](./archive/modules/tutorial-path-continuation.aps.md) | Tutorial | 0.1.x |
| [website-migration](./archive/modules/website-migration.aps.md) | WEB | 0.1.0 |
| [monorepo-migration](./archive/modules/monorepo-migration.aps.md) | MONO | 0.1.0 |
| [test-quality](./archive/modules/test-quality.aps.md) | TEST | — |
| [beta-launch-checklist](./archive/modules/beta-launch-checklist.aps.md) | — | 0.1.2-beta |
| [beta-testing-improvements](./archive/modules/beta-testing-improvements.aps.md) | — | 0.1.2-beta |
| [post-beta-launch-uplift](./archive/modules/post-beta-launch-uplift.aps.md) | PBLU | 0.1.x |
| [migrate-unosend-to-resend](./archive/modules/migrate-unosend-to-resend.md) | — | 0.1.x |

### Completed (0.1.x)

| Module | Scope | Status | Progress |
| ------ | ----- | ------ | -------- |
| [cli-hardening](./archive/modules/cli-hardening.aps.md) | CLIH | Complete | — |
| [coaching-nudges](./archive/modules/coaching-nudges.aps.md) | NUDGE | Complete | — |
| [mcp-server-hardening](./archive/modules/mcp-server-hardening.aps.md) | MCPH | Complete | — |
| [nx-task-migration](./archive/modules/nx-task-migration.aps.md) | NXTASK | Complete | 6/6 |
| [security-ci-pipeline](./archive/modules/security-ci-pipeline.aps.md) | SEC | Complete | — |
| [cli-esbuild-bundling](./archive/modules/cli-esbuild-bundling.aps.md) | BUNDLE | Complete | 3/3 |
| [01-forge-hook-agent](./archive/modules/01-forge-hook-agent.aps.md) | FORGE | Complete | 5/5 |
| [02-forge-negotiation](./archive/modules/02-forge-negotiation.aps.md) | FNEG | Complete | 5/5 |
| [03-deferred-finding-filing](./archive/modules/03-deferred-finding-filing.aps.md) | DEFER | Complete | 5/5 |
| [04-temper-workflow](./archive/modules/04-temper-workflow.aps.md) | TEMPER | Complete | 6/6 |
| [05-forge-temper-config](./archive/modules/05-forge-temper-config.aps.md) | FTCFG | Complete | 6/6 |
| [code-review-backlog](./archive/modules/code-review-backlog.aps.md) | CRB | Complete | 29/29 |

### Completed (0.4.0 — Edda Stack)

| Module | Scope | Status | Progress | Dependencies |
| ------ | ----- | ------ | -------- | ------------ |
| [kindling-integration](./archive/modules/kindling-integration.aps.md) | KINDLING | Complete | 19/19 | save-time-trust, drift-reporting |
| [ember](./archive/modules/ember.aps.md) | EMBER | Complete | 14/14 | kindling-integration |
| [edda](./archive/modules/edda.aps.md) | EDDA | Complete | 19/19 | ember |
| [edda-stack-integration](./archive/modules/edda-stack-integration.aps.md) | STACK | Complete | 19/19 | kindling-integration, ember, edda |
| [edda-ember-review](./archive/modules/edda-ember-review.aps.md) | EERB | Complete | 16/16 | ember, edda |

### Task Status — 0.1.0 (Core Engine)

| Task     | Module          | Description                      | Status   |
| -------- | --------------- | -------------------------------- | -------- |
| CORE-001 | save-time-trust | Warning schema definition        | Complete |
| CORE-002 | save-time-trust | Check runner refactor            | Complete |
| CORE-003 | save-time-trust | CLI check command                | Complete |
| CORE-004 | save-time-trust | Git-aware changed file detection | Complete |
| CORE-005 | save-time-trust | Source file watch mode           | Complete |
| ARCH-001 | architecture    | Baseline inference               | Complete |
| ARCH-002 | architecture    | Edge detection                   | Complete |
| ARCH-003 | architecture    | Architecture check integration   | Complete |
| ARCH-004 | architecture    | CLI architecture service         | Complete |
| ANTI-001 | antipattern     | Pattern catalogue definition     | Complete |
| ANTI-002 | antipattern     | Scanner implementation           | Complete |
| ANTI-003 | antipattern     | Antipattern check integration    | Complete |
| ANTI-004 | antipattern     | Allowlist and opt-in support     | Complete |
| SUPP-001 | suppressions    | Suppression parser               | Complete |
| SUPP-002 | suppressions    | Suppression store                | Complete |
| SUPP-003 | suppressions    | Gate runner integration          | Complete |
| CI-001   | ci-integration  | GitHub Action composite          | Complete |
| CI-002   | ci-integration  | Changed files detection          | Complete |
| CI-003   | ci-integration  | PR comments and status checks    | Complete |
| CI-004   | ci-integration  | Documentation and configuration  | Complete |

### Task Status — 0.1.0 (Onboarding TUI)

| Task    | Module | Description                   | Status   | Priority |
| ------- | ------ | ----------------------------- | -------- | -------- |
| TUI-001 | tui    | Ink foundation and components | Complete | high     |
| TUI-002 | tui    | `anvil init` wizard           | Complete | high     |
| TUI-003 | tui    | `anvil status` dashboard      | Complete | high     |
| TUI-004 | tui    | `anvil doctor` diagnostics    | Complete | high     |
| TUI-005 | tui    | First-run welcome experience  | Complete | high     |
| TUI-008 | tui    | Testing infrastructure        | Complete | medium   |

### Task Status — 0.1.0 (Documentation)

| Task     | Module | Description            | Status   | Priority |
| -------- | ------ | ---------------------- | -------- | -------- |
| DOCS-001 | docs   | Quick Start Guide      | Complete | high     |
| DOCS-002 | docs   | User Guide command ref | Complete | high     |
| DOCS-003 | docs   | Demo material creation | Complete | high     |
| DOCS-004 | docs   | Error message audit    | Complete | medium   |
| DOCS-005 | docs   | Troubleshooting guide  | Complete | medium   |
| DOCS-006 | docs   | README refresh         | Complete | high     |

### Task Status — 0.1.0 (Explain Command)

| Task       | Module  | Description               | Status   | Priority |
| ---------- | ------- | ------------------------- | -------- | -------- |
| EXPLAIN-001 | explain | Warning ID system         | Complete | high     |
| EXPLAIN-002 | explain | Explanation templates     | Complete | high     |
| EXPLAIN-003 | explain | Architecture explanations | Complete | high     |
| EXPLAIN-004 | explain | Anti-pattern explanations | Complete | high     |
| EXPLAIN-005 | explain | ExplainService            | Complete | high     |
| EXPLAIN-006 | explain | CLI explain command       | Complete | high     |

### Task Status — 0.1.0 (Drift Reporting)

| Task     | Module | Description               | Status   | Priority |
| -------- | ------ | ------------------------- | -------- | -------- |
| DRIFT-001 | drift  | Snapshot schema & storage | Complete | high     |
| DRIFT-002 | drift  | Snapshot capture          | Complete | high     |
| DRIFT-003 | drift  | Snapshot comparison       | Complete | high     |
| DRIFT-004 | drift  | Report generator          | Complete | medium   |
| DRIFT-005 | drift  | CLI drift commands        | Complete | high     |

### Task Status — 0.1.0 (Onboarding Feedback Resolution)

| Task     | Module | Description                                 | Status   | Priority |
| -------- | ------ | ------------------------------------------- | -------- | -------- |
| ONFBK-001 | onfbk  | Fix --no-tui flag handling                  | Complete | high     |
| ONFBK-002 | onfbk  | Fix TUI wizard early exit                   | Complete | high     |
| ONFBK-003 | onfbk  | Improve layer detection for project variety | Complete | high     |
| ONFBK-004 | onfbk  | Improve entry points presentation           | Complete | medium   |
| ONFBK-005 | onfbk  | Add architecture explanation                | Complete | medium   |

### Task Status — 0.1.0 (OPA & Architecture Integration)

| Task    | Module | Description                         | Status      | Priority |
| ------- | ------ | ----------------------------------- | ----------- | -------- |
| OPA-001 | opa    | Architecture YAML schema (Zod)      | Complete    | high     |
| OPA-002 | opa    | YAML parser with template expansion | Complete    | high     |
| OPA-003 | opa    | DC config generator from YAML       | Complete    | high     |
| OPA-004 | opa    | `anvil architecture init` command   | Complete    | high     |
| OPA-005 | opa    | Architecture context extraction     | Complete    | high     |
| OPA-006 | opa    | OPA input schema enhancement        | Complete    | high     |
| OPA-007 | opa    | Gate runner integration             | Complete    | high     |
| OPA-008 | opa    | Rego generator from architecture    | Complete    | high     |
| OPA-009 | opa    | Generated policy marker             | Complete    | medium   |
| OPA-010 | opa    | Auto-regeneration on YAML change    | Complete    | medium   |
| OPA-011 | opa    | Layered architecture template       | Complete    | medium   |
| OPA-012 | opa    | Hexagonal architecture template     | Complete    | medium   |
| OPA-013 | opa    | Clean Architecture template         | Complete    | medium   |
| OPA-014 | opa    | DDD template with bounded contexts  | Complete    | medium   |
| OPA-015 | opa    | Template loader and validator       | Complete    | medium   |
| OPA-016 | opa    | TypeScript analyser foundation      | Deferred    | low      |
| OPA-017 | opa    | Path alias resolver                 | Deferred    | low      |
| OPA-018 | opa    | Analyser feature flag               | Deferred    | low      |
| OPA-019 | opa    | Bundle download and caching         | Complete    | medium   |
| OPA-020 | opa    | Signature verification              | Complete    | medium   |
| OPA-021 | opa    | Basic auth and CLI commands         | Complete    | medium   |

> **Note:** OPA-016 through OPA-018 were deferred when the OPA module was marked
> Complete at OPA-015. OPA-019 through OPA-021 (remote policy bundles) were
> subsequently implemented. The remaining tasks may be revisited in the OPA
> Enhancements module (OPAE) or a future release.

### Task Status — 0.1.0 (Monorepo Migration)

| Task     | Module | Description                          | Status   | Priority |
| -------- | ------ | ------------------------------------ | -------- | -------- |
| MONO-001 | mono   | Nx generators for package scaffolding | Complete | high     |
| MONO-002 | mono   | Import path codemod                  | Complete | high     |
| MONO-003 | mono   | Shared tooling packages              | Complete | medium   |
| MONO-004 | mono   | Extract contracts from core          | Complete | high     |
| MONO-005 | mono   | Extract ports from core              | Complete | high     |
| MONO-006 | mono   | Extract pure domain to core          | Complete | high     |
| MONO-007 | mono   | Extract runtime package              | Complete | high     |
| MONO-008 | mono   | Extract policy package               | Complete | high     |
| MONO-009 | mono   | Extract config package               | Complete | medium   |
| MONO-010 | mono   | Extract storage package              | Complete | medium   |
| MONO-011 | mono   | Extract crypto package               | Complete | medium   |
| MONO-012 | mono   | Split adapters per-integration       | Complete | medium   |
| MONO-013 | mono   | Move CLI to apps/                    | Complete | high     |
| MONO-014 | mono   | Reorganise E2E tests                 | Complete | medium   |
| MONO-015 | mono   | Move scripts to tools/               | Complete | low      |
| MONO-016 | mono   | Full test suite validation           | Complete | high     |
| MONO-017 | mono   | Dependency graph validation          | Complete | high     |
| MONO-018 | mono   | Documentation update                 | Complete | medium   |

### Task Status — 0.1.0 (APS Markdown Adapter)

| Task     | Module | Description                          | Status   | Priority |
| -------- | ------ | ------------------------------------ | -------- | -------- |
| APSMD-001 | apsmd  | APSMarkdownAdapter with detection    | Complete | high     |
| APSMD-002 | apsmd  | Confidence scoring system            | Complete | high     |
| APSMD-003 | apsmd  | Parse method implementation          | Complete | high     |
| APSMD-004 | apsmd  | Task-to-Change conversion            | Complete | high     |
| APSMD-005 | apsmd  | Registry integration                 | Complete | high     |
| APSMD-006 | apsmd  | CLI PlanLoader integration           | Complete | high     |

### Task Status — 0.1.0 (Advanced Experience)

#### IDE Integration (VS Code Extension)

| Task    | Module | Description                                     | Status   | Priority |
| ------- | ------ | ----------------------------------------------- | -------- | -------- |
| IDE-001 | ide    | Embed @eddacraft/anvil-core for fast-path operations      | Complete | high     |
| IDE-002 | ide    | Anti-pattern detection on save with diagnostics | Complete | high     |
| IDE-003 | ide    | Improve source location mapping from CLI output | Complete | medium   |
| IDE-004 | ide    | Architecture gate display in tree view          | Complete | high     |
| IDE-005 | ide    | OPA policy failure display with remediation     | Complete | high     |
| IDE-006 | ide    | Click-to-navigate for all violation types       | Complete | medium   |
| IDE-007 | ide    | APS and Rego syntax highlighting                | Complete | medium   |
| IDE-008 | ide    | Analysis caching and Marketplace preparation    | Complete | medium   |

#### TUI Operational (CLI)

| Task    | Module | Description                       | Status  | Priority |
| ------- | ------ | --------------------------------- | ------- | -------- |
| TUI-009 | tui    | `anvil watch` real-time dashboard | Complete | medium   |
| TUI-013 | tui    | `<MermaidDiagram />` component + `layersToMermaid()` helper | Complete | high |
| TUI-014 | tui    | Replace existing ASCII diagrams with mermaid rendering | Complete | high |
| TUI-015 | tui    | `anvil architecture visualise` command (ascii/svg/mermaid formats) | Complete | high |

### Task Status — 0.1.0 (HTML/CSS Support)

| Task        | Module  | Description                                 | Status   | Priority |
| ----------- | ------- | ------------------------------------------- | -------- | -------- |
| HTMLCSS-001 | htmlcss | Make analysable extensions configurable      | Complete | high     |
| HTMLCSS-002 | htmlcss | HTML anti-pattern detectors (AP-008–011)     | Complete | high     |
| HTMLCSS-003 | htmlcss | CSS anti-pattern detectors (AP-012–013)      | Complete | high     |
| HTMLCSS-004 | htmlcss | HTML/CSS edge detection                      | Complete | high     |
| HTMLCSS-005 | htmlcss | HTML suppression comment syntax              | Complete | high     |
| HTMLCSS-006 | htmlcss | VS Code extension HTML/CSS trigger           | Complete | medium   |
| HTMLCSS-007 | htmlcss | Documentation and tests                      | Complete | medium   |

### Task Status — 0.1.0 (Tutorial Overhaul)

| Task    | Module | Description                                          | Status   | Priority |
| ------- | ------ | ---------------------------------------------------- | -------- | -------- |
| TUT-001 | tut    | Rewrite tutorial step types for scan-watch-fix flow  | Complete | high     |
| TUT-002 | tut    | Create ScanStep TUI component                        | Complete | high     |
| TUT-003 | tut    | Create WatchStep TUI component                       | Complete | high     |
| TUT-004 | tut    | Create FixStep TUI component                         | Complete | high     |
| TUT-005 | tut    | Create NextStepsStep and wire up Tutorial.tsx         | Complete | high     |
| TUT-006 | tut    | Interactive policy creation tutorial                  | Complete | medium   |
| TUT-007 | tut    | Interactive architecture boundaries tutorial          | Complete | medium   |
| TUT-008 | tut    | Interactive drift tracking tutorial                   | Complete | medium   |
| TUT-009 | tut    | Interactive CI integration tutorial                   | Complete | high     |
| TUT-010 | tut    | Docs-site tutorials section                           | Complete | high     |
| TUT-011 | tut    | Rewrite quickstart.md and update navigation           | Complete | high     |
| TUT-012 | tut    | Tutorial --list flag and e2e test                     | Complete | high     |

### Task Status — 0.1.0 (Intelligent First Run)

| Task    | Module | Description                                   | Status   | Priority |
| ------- | ------ | --------------------------------------------- | -------- | -------- |
| IFR-001 | ifr    | Add project context detection service         | Complete | high     |
| IFR-002 | ifr    | Create smart defaults generator               | Complete | high     |
| IFR-003 | ifr    | Add post-init automatic analysis              | Complete | high     |
| IFR-004 | ifr    | Create quick wins identifier                  | Complete | high     |
| IFR-005 | ifr    | Create interactive results dashboard TUI      | Complete | high     |
| IFR-006 | ifr    | Add historical analysis feature               | Complete | medium   |
| IFR-007 | ifr    | Integrate all components in init flow         | Complete | high     |
| IFR-008 | ifr    | Update documentation                          | Complete | medium   |

### Task Status — 0.1.0 (Adapter Upstream Updates)

| Task        | Module  | Description                                 | Status   | Priority |
| ----------- | ------- | ------------------------------------------- | -------- | -------- |
| ADAPTUP-001 | adaptup | Update BMAD folder structure detection       | Complete | high     |
| ADAPTUP-002 | adaptup | Update BMAD config path handling             | Complete | high     |
| ADAPTUP-003 | adaptup | Update BMAD variable syntax                  | Complete | medium   |
| ADAPTUP-004 | adaptup | Add BMAD hasSidecar field support             | Complete | medium   |
| ADAPTUP-005 | adaptup | Update SpecKit command namespace detection   | Complete | high     |
| ADAPTUP-006 | adaptup | Add SpecKit AGENTS.md support                | Complete | medium   |
| ADAPTUP-007 | adaptup | Update adapter test fixtures                 | Complete | high     |
| ADAPTUP-008 | adaptup | Update adapter documentation                 | Complete | medium   |

### Task Status — 0.1.0 (AI Tool Integration)

| Task       | Module         | Description                       | Status  | Priority |
| ---------- | -------------- | --------------------------------- | ------- | -------- |
| LLMS-001   | llms-txt       | Constraint collector              | Complete | high     |
| LLMS-002   | llms-txt       | llms.txt formatter                | Complete | high     |
| LLMS-003   | llms-txt       | MCP resource formatter            | Complete | medium   |
| LLMS-004   | llms-txt       | Prompt fragment formatter         | Complete | medium   |
| LLMS-005   | llms-txt       | CLI export command                | Complete | high     |
| CMDSAF-001 | command-safety | Rule system and types             | Complete | high     |
| CMDSAF-002 | command-safety | Command parser with unwrapping    | Complete | high     |
| CMDSAF-003 | command-safety | Rule matcher with specificity     | Complete | high     |
| CMDSAF-004 | command-safety | Default git operation rules       | Complete | medium   |
| CMDSAF-005 | command-safety | Default filesystem rules          | Complete | medium   |
| CMDSAF-006 | command-safety | CommandSafetyCheck implementation | Complete | high     |
| CMDSAF-007 | command-safety | Configuration system              | Complete | medium   |
| CMDSAF-008 | command-safety | Message formatting                | Complete | low      |
| CMDSAF-009 | command-safety | CLI integration and documentation | Complete | high     |
| MCP-001    | mcp-server     | Package scaffold and basic server | Complete | high     |
| MCP-002    | mcp-server     | anvil_check tool implementation   | Complete | high     |
| MCP-003    | mcp-server     | anvil_gate and anvil_status tools | Complete | high     |
| MCP-004    | mcp-server     | anvil_fix and anvil_suppress tools| Complete | high     |
| MCP-005    | mcp-server     | anvil_query_boundary tool         | Complete | high     |
| MCP-006    | mcp-server     | Resources with subscriptions      | Complete | medium   |
| MCP-007    | mcp-server     | Prompt templates                  | Complete | medium   |
| MCP-008    | mcp-server     | Streamable HTTP transport         | Complete | medium   |
| MCP-009    | mcp-server     | Config generators and CLI         | Complete | high     |
| MCP-010    | mcp-server     | Error handling and JSON-RPC       | Complete | high     |

### Task Status — 0.4.0 (Edda Stack — Memory System)

The Edda Stack provides a three-layer architecture for memory: Kindling (observation),
Ember (interpretation), and Edda (canonical memory).

#### Kindling Integration (Observation Layer)

| Task         | Module   | Description                         | Status   | Priority |
| ------------ | -------- | ----------------------------------- | -------- | -------- |
| KINDLING-001 | kindling | Kindling service wrapper            | Complete | high     |
| KINDLING-002 | kindling | Configuration schema and loading    | Complete | high     |
| KINDLING-003 | kindling | Session observation hooks           | Complete | high     |
| KINDLING-004 | kindling | Gate evaluation observations        | Complete | high     |
| KINDLING-005 | kindling | Action execution observations       | Complete | medium   |
| KINDLING-006 | kindling | Plan lifecycle observations         | Complete | medium   |
| KINDLING-007 | kindling | Human input and constraint obs      | Complete | medium   |
| KINDLING-008 | kindling | Error observations                  | Complete | high     |
| KINDLING-009 | kindling | Query service with scope enforcement| Complete | high     |
| KINDLING-010 | kindling | Query limits and throttling         | Complete | high     |
| KINDLING-011 | kindling | Malicious AI test suite             | Complete | high     |
| KINDLING-012 | kindling | Session query command (run show)    | Complete | high     |
| KINDLING-013 | kindling | Plan, gate, action query commands   | Complete | high     |
| KINDLING-014 | kindling | Status integration                  | Complete | medium   |
| KINDLING-015 | kindling | Sensitive data validation           | Complete | high     |
| KINDLING-016 | kindling | Retention and pruning               | Complete | medium   |
| KINDLING-017 | kindling | Performance benchmarking            | Complete | medium   |
| KINDLING-018 | kindling | Documentation and examples          | Complete | medium   |
| KINDLING-019 | kindling | OpenAPI spec generation             | Complete | medium   |

#### Ember (Interpretive Layer — Candidate Memory)

| Task      | Module | Description                       | Status   | Priority |
| --------- | ------ | --------------------------------- | -------- | -------- |
| EMBER-001 | ember  | Candidate Memory Proposal schema  | Complete | high     |
| EMBER-002 | ember  | Proposal type definitions         | Complete | high     |
| EMBER-003 | ember  | Ember configuration schema        | Complete | high     |
| EMBER-004 | ember  | ProposalStore implementation      | Complete | high     |
| EMBER-005 | ember  | DecayService implementation       | Complete | high     |
| EMBER-006 | ember  | AggregatorService foundation      | Complete | medium   |
| EMBER-007 | ember  | Evaluation rules engine           | Complete | medium   |
| EMBER-008 | ember  | Built-in evaluation rules         | Complete | medium   |
| EMBER-009 | ember  | CandidateService (high-level API) | Complete | high     |
| EMBER-010 | ember  | Kindling observation hooks        | Complete | medium   |
| EMBER-011 | ember  | CLI ember commands                | Complete | high     |
| EMBER-012 | ember  | Query API implementation          | Complete | high     |
| EMBER-013 | ember  | Status integration                | Complete | medium   |
| EMBER-014 | ember  | Documentation and examples        | Complete | medium   |

#### Edda (Canonical Memory Layer)

| Task      | Module | Description                       | Status   | Priority |
| --------- | ------ | --------------------------------- | -------- | -------- |
| EDDA-001  | edda   | Memory Object schema              | Complete | high     |
| EDDA-002  | edda   | Memory type definitions           | Complete | high     |
| EDDA-003  | edda   | Provenance schema                 | Complete | high     |
| EDDA-004  | edda   | Evolution graph schema            | Complete | high     |
| EDDA-005  | edda   | Edda configuration schema         | Complete | high     |
| EDDA-006  | edda   | Git-backed MemoryStore            | Complete | high     |
| EDDA-007  | edda   | YAML serialisation                | Complete | high     |
| EDDA-008  | edda   | Version tracking                  | Complete | medium   |
| EDDA-009  | edda   | PromotionService                  | Complete | high     |
| EDDA-010  | edda   | ProvenanceService                 | Complete | medium   |
| EDDA-011  | edda   | EvolutionService                  | Complete | high     |
| EDDA-012  | edda   | MemoryService (high-level API)    | Complete | high     |
| EDDA-013  | edda   | CLI list and show commands        | Complete | high     |
| EDDA-014  | edda   | CLI promote command               | Complete | high     |
| EDDA-015  | edda   | CLI retire and trace commands     | Complete | high     |
| EDDA-016  | edda   | Human-in-the-loop enforcement     | Complete | high     |
| EDDA-017  | edda   | Status integration                | Complete | medium   |
| EDDA-018  | edda   | Schema migration tooling          | Complete | medium   |
| EDDA-019  | edda   | Documentation                     | Complete | medium   |

#### Edda Stack Integration

| Task      | Module | Description                       | Status   | Priority |
| --------- | ------ | --------------------------------- | -------- | -------- |
| STACK-001 | stack  | Common identifier schemas         | Complete | high     |
| STACK-002 | stack  | Timestamp and temporal schemas    | Complete | high     |
| STACK-003 | stack  | Confidence scale definitions      | Complete | high     |
| STACK-004 | stack  | Provenance link schema            | Complete | high     |
| STACK-005 | stack  | Proposal → Memory type mapping    | Complete | high     |
| STACK-006 | stack  | Observation → Proposal mapping    | Complete | medium   |
| STACK-007 | stack  | Layer port definitions            | Complete | high     |
| STACK-008 | stack  | Event bus for layer communication | Complete | medium   |
| STACK-009 | stack  | Layer mock factories              | Complete | high     |
| STACK-010 | stack  | Integration test fixtures         | Complete | high     |
| STACK-011 | stack  | Provenance chain validator        | Complete | high     |
| STACK-012 | stack  | Stack configuration schema        | Complete | high     |
| STACK-013 | stack  | CLI stack status command          | Complete | high     |
| STACK-014 | stack  | CLI stack validate command        | Complete | high     |
| STACK-015 | stack  | Stack architecture documentation  | Complete    | medium   |
| STACK-016 | stack  | Migration guide                   | Complete    | medium   |
| STACK-017 | stack  | Path drift cleanup in APS plans   | Complete | medium   |
| STACK-018 | stack  | Retroactive evidence capture      | Complete    | medium   |
| STACK-019 | stack  | Missing deliverable audit         | Complete    | medium   |

#### Edda-Ember Review Backlog

Non-critical improvements from the 2026-03-05 consolidated code review of the
Edda + Ember feature branches. All 10 critical issues resolved; these track
remaining major and minor improvements.

| Task     | Module | Description                                        | Status   | Priority |
| -------- | ------ | -------------------------------------------------- | -------- | -------- |
| EERB-001 | eerb   | Race condition in processSession candidate limit   | Complete | Low      |
| EERB-002 | eerb   | EscalationRule assumes array order equals temporal  | Complete | Medium   |
| EERB-003 | eerb   | Prune threshold duplicated with different values   | Complete | Medium   |
| EERB-004 | eerb   | Fallback synthesises fake UUIDs for provenance     | Complete | Medium   |
| EERB-005 | eerb   | Duplicated queryProposals call in ember list       | Complete | Low      |
| EERB-006 | eerb   | Dismissed count missing from anvil status Ember    | Complete | Low      |
| EERB-007 | eerb   | colourStatus/colourConfidence duplicated in ember  | Complete | Low      |
| EERB-008 | eerb   | Hardcoded method: 'cli_command' in attribution     | Complete | Low      |
| EERB-009 | eerb   | Double search filtering is redundant               | Complete | Medium   |
| EERB-010 | eerb   | Hardcoded limit: 100 silently truncates methods    | Complete | Low      |
| EERB-011 | eerb   | groupByKind uses O(n²) array spread in loop        | Complete | Low      |
| EERB-012 | eerb   | getExpiringsSoon double-s typo                     | Complete | Low      |
| EERB-013 | eerb   | SurpriseRule references unknown observation kinds   | Complete | Low      |
| EERB-014 | eerb   | validateEvolutionGraph uses .parse() not .safeParse | Complete | Low      |
| EERB-015 | eerb   | serialisation.ts has manual MemoryIndexEntry type  | Complete | Low      |
| EERB-016 | eerb   | migrateV0ToV1 status preservation path untested    | Complete | Low      |

### Task Status — 0.1.0 (Pulumi Infrastructure as Code)

| Task    | Module | Description                              | Status   | Priority |
| ------- | ------ | ---------------------------------------- | -------- | -------- |
| IAC-001 | iac    | Scaffold Pulumi project in monorepo      | Complete | high     |
| IAC-002 | iac    | Configure Pulumi state backend           | Complete | high     |
| IAC-003 | iac    | Manage website Vercel project config     | Complete | high     |
| IAC-004 | iac    | Manage docs-site Vercel project config   | Complete | high     |
| IAC-005 | iac    | Create VercelApp ComponentResource       | Complete | medium   |
| IAC-007 | iac    | Manage Azure DNS zones and records       | Complete | high     |
| IAC-008 | iac    | Add Pulumi CI/CD pipeline integration    | Complete | high     |
| IAC-009 | iac    | Write unit tests for infrastructure code | Complete | medium   |
| IAC-010 | iac    | Import existing Vercel resources         | Complete | high     |
| IAC-011 | iac    | Document IaC setup and contributor guide | Complete | medium   |
| IAC-012 | iac    | Document rollback procedures             | Complete | medium   |

### Task Status — 0.1.x (Code Review Backlog)

Architectural recommendations from the 2026-02-16 code review.

| Task    | Module | Description                                         | Status   | Priority |
| ------- | ------ | --------------------------------------------------- | -------- | -------- |
| CRB-001 | crb    | Standardise stderr/stdout policy across CLI         | Complete | Medium   |
| CRB-002 | crb    | Consolidate hook scripts to single source           | Complete | Medium   |
| CRB-003 | crb    | Add Zod validation to runtime YAML parsers          | Complete | Medium   |
| CRB-004 | crb    | OPA binary manager safer PATH + shared logger       | Complete | Low      |
| CRB-005 | crb    | Dependency audit — surface errors deterministically | Complete | Medium   |
| CRB-006 | crb    | Monorepo-wide vitest config strategy                | Complete | Low      |
| CRB-007 | crb    | Move process.exit from library code to CLI layer    | Complete | High     |
| CRB-008 | crb    | Consistent workspace root containment for output    | Complete | High     |
| CRB-009 | crb    | OPA checksum table contains placeholder hashes      | Complete | High     |
| CRB-010 | crb    | APS task locking is not atomic (race condition)     | Complete | Medium   |
| CRB-011 | crb    | APS loader maxDepth parameter ignored               | Complete | Low      |
| CRB-012 | crb    | Config loader placeholder vs Complete status drift  | Complete | Low      |
| CRB-013 | crb    | MCP server tests not in vitest include globs        | Complete | Medium   |
| CRB-014 | crb    | Add tests for git command composition safety        | Complete | Medium   |
| CRB-015 | crb    | Add symlink escape tests to file-storage            | Complete | Medium   |
| CRB-016 | crb    | Add Windows separator tests to MCP path guards      | Complete | Low      |
| CRB-017 | crb    | Add tests for platform/core config loaders          | Complete | Low      |
| CRB-018 | crb    | Standardise works-from-repo-root workflow           | Complete | Medium   |
| CRB-019 | crb    | Consistent logging/output conventions               | Complete | Medium   |
| CRB-020 | crb    | Option parsing/validation inconsistency             | Complete | Low      |
| CRB-021 | crb    | Duplicated implementations and naming drift         | Complete | Low      |
| CRB-023 | crb    | Silent fallbacks without visibility                 | Complete | Medium   |
| CRB-024 | crb    | Subprocess calls without timeouts in CI             | Complete | Medium   |
| CRB-025 | crb    | Docs and scripts drifting from reality              | Complete | Low      |
| CRB-026 | crb    | Fix spinner leak on TUI fallback path in audit      | Complete | Medium   |
| CRB-027 | crb    | Add workspace path containment to policy validate   | Complete | High     |
| CRB-028 | crb    | Annotate mcp-config symlink guard as fixed          | Complete | Low      |
| CRB-029 | crb    | Expand test coverage for untested CLI commands      | Complete | Medium   |

### Task Status — 0.1.x (Codebase Maintenance)

| Task      | Module | Description                                         | Status   | Priority |
| --------- | ------ | --------------------------------------------------- | -------- | -------- |
| MAINT-001 | maint  | CLI option coercion utility (from CRB-020 discovery) | Complete | High     |
| MAINT-002 | maint  | Error formatting consistency                        | Complete | Medium   |
| MAINT-003 | maint  | Workspace root resolution patterns                  | Complete | Low      |
| MAINT-004 | maint  | Git operation wrappers                              | Complete | Medium   |
| MAINT-005 | maint  | JSON output formatting                              | Complete | Low      |
| MAINT-006 | maint  | Nx generator for CLI commands                       | Complete | Low      |
| MAINT-007 | maint  | Nx generator for gate checks                        | Complete | Low      |
| MAINT-008 | maint  | Spinner/progress patterns                           | Complete | Low      |

### Task Status — 0.1.x (Forge & Temper: Autonomous Code Review Pipeline)

Pre-commit review (Forge) and post-push self-healing (Temper) pipeline.
Design doc: [docs/plans/2026-02-24-forge-temper-review-pipeline.md](../docs/plans/2026-02-24-forge-temper-review-pipeline.md)

#### Forge Hook & Agent

| Task      | Module | Description                          | Status   | Priority |
| --------- | ------ | ------------------------------------ | -------- | -------- |
| FORGE-001 | forge  | Create forge.sh PreToolUse hook      | Complete | high     |
| FORGE-002 | forge  | Create forge-reviewer agent spec     | Complete | high     |
| FORGE-004 | forge  | Implement Forge report logging       | Complete | medium   |

#### Forge Negotiation Protocol

| Task     | Module | Description                              | Status   | Priority |
| -------- | ------ | ---------------------------------------- | -------- | -------- |
| FNEG-002 | fneg   | Implement round cap enforcement          | Complete | high     |
| FNEG-003 | fneg   | Implement scoped re-review for rounds 2+ | Complete | medium   |
| FNEG-004 | fneg   | Implement severity-action matrix         | Complete | high     |
| FNEG-005 | fneg   | Implement fix-and-restage flow           | Complete | medium   |

#### Temper Workflow

| Task       | Module | Description                            | Status   | Priority |
| ---------- | ------ | -------------------------------------- | -------- | -------- |
| TEMPER-001 | temper | Create temper.yml workflow scaffold    | Complete | high     |
| TEMPER-002 | temper | Implement cycle 1 full review          | Complete | high     |
| TEMPER-003 | temper | Implement cycle 2 scoped re-review     | Complete | medium   |
| TEMPER-004 | temper | Implement cycle cap enforcement        | Complete | high     |
| TEMPER-005 | temper | Implement manual dispatch trigger      | Complete | medium   |
| TEMPER-006 | temper | Implement PR summary comments          | Complete | medium   |

#### Forge & Temper Configuration & Documentation

| Task      | Module | Description                               | Status   | Priority |
| --------- | ------ | ----------------------------------------- | -------- | -------- |
| FTCFG-002 | ftcfg  | Document Temper GitHub repo variables     | Complete | medium   |
| FTCFG-003 | ftcfg  | Update CLAUDE.md hook behavior table      | Complete | high     |
| FTCFG-004 | ftcfg  | Update CLAUDE.md env var table            | Complete | high     |
| FTCFG-005 | ftcfg  | Document pipeline overview in CLAUDE.md   | Complete | high     |

## Decisions

- **D-001:** Planless-first posture — deliver value without requiring APS plans
  ([ADR](./decisions/001-planless-first.md))
- **D-002:** Warnings over blocks — inform, don't prevent; let CI enforce if
  desired ([ADR](./decisions/002-warnings-over-blocks.md))
- **D-003:** New edges only — baseline existing architecture; warn only on new
  violations ([ADR](./decisions/003-new-edges-only.md))
- **D-004:** Suppression syntax — `@anvil-ignore <ID>: <reason>` with mandatory
  explanation ([ADR](./decisions/004-suppression-syntax.md))
- **D-005:** Ink over OpenTUI — Node.js compatibility over native performance
  ([ADR](./decisions/005-ink-over-opentui.md))
- **D-006:** Hybrid DC + OPA — DC for analysis, OPA for policies, bridge between
  ([ADR](./decisions/006-hybrid-dc-opa.md))
- **D-007:** Pulumi for IaC — open-source Pulumi with TypeScript over Terraform
  for consistency with the monorepo's TypeScript-first toolchain
  ([ADR](./decisions/007-pulumi-iac.md))
- **D-011:** OPA Agent Orchestration — orchestration layer for checkpointed policy
  evaluation, remediation guidance, and auditable exception workflows
  ([ADR](./decisions/011-opa-agent-orchestration.md))
- **D-012:** Eval Harness Adoption — adopt external eval framework behind Anvil
  adapter contracts for CI-native trust regression testing
  ([ADR](./decisions/012-eval-harness-adoption.md))
