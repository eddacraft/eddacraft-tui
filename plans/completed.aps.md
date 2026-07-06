<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- This document is an archive of completed task-level detail. -->

# Completed Work Archive

| Scope | Status   |
| ----- | -------- |
| ALL   | Complete |

## Purpose

Historical record of all completed task-level work. Grouped by release and
module area. Individual module specs in `plans/archive/modules/` contain the
authoritative detail; this file preserves the task tables for reference and
traceability.

---

## 0.1.0 — Beta

### Core Engine

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

### Onboarding TUI

| Task    | Module | Description                   | Status   | Priority |
| ------- | ------ | ----------------------------- | -------- | -------- |
| TUI-001 | tui    | Ink foundation and components | Complete | high     |
| TUI-002 | tui    | `anvil init` wizard           | Complete | high     |
| TUI-003 | tui    | `anvil status` dashboard      | Complete | high     |
| TUI-004 | tui    | `anvil doctor` diagnostics    | Complete | high     |
| TUI-005 | tui    | First-run welcome experience  | Complete | high     |
| TUI-008 | tui    | Testing infrastructure        | Complete | medium   |

### Documentation

| Task     | Module | Description            | Status   | Priority |
| -------- | ------ | ---------------------- | -------- | -------- |
| DOCS-001 | docs   | Quick Start Guide      | Complete | high     |
| DOCS-002 | docs   | User Guide command ref | Complete | high     |
| DOCS-003 | docs   | Demo material creation | Complete | high     |
| DOCS-004 | docs   | Error message audit    | Complete | medium   |
| DOCS-005 | docs   | Troubleshooting guide  | Complete | medium   |
| DOCS-006 | docs   | README refresh         | Complete | high     |

### Explain Command

| Task        | Module  | Description               | Status   | Priority |
| ----------- | ------- | ------------------------- | -------- | -------- |
| EXPLAIN-001 | explain | Warning ID system         | Complete | high     |
| EXPLAIN-002 | explain | Explanation templates     | Complete | high     |
| EXPLAIN-003 | explain | Architecture explanations | Complete | high     |
| EXPLAIN-004 | explain | Anti-pattern explanations | Complete | high     |
| EXPLAIN-005 | explain | ExplainService            | Complete | high     |
| EXPLAIN-006 | explain | CLI explain command       | Complete | high     |

### Drift Reporting

| Task      | Module | Description               | Status   | Priority |
| --------- | ------ | ------------------------- | -------- | -------- |
| DRIFT-001 | drift  | Snapshot schema & storage | Complete | high     |
| DRIFT-002 | drift  | Snapshot capture          | Complete | high     |
| DRIFT-003 | drift  | Snapshot comparison       | Complete | high     |
| DRIFT-004 | drift  | Report generator          | Complete | medium   |
| DRIFT-005 | drift  | CLI drift commands        | Complete | high     |

### Onboarding Feedback Resolution

| Task      | Module | Description                                 | Status   | Priority |
| --------- | ------ | ------------------------------------------- | -------- | -------- |
| ONFBK-001 | onfbk  | Fix --no-tui flag handling                  | Complete | high     |
| ONFBK-002 | onfbk  | Fix TUI wizard early exit                   | Complete | high     |
| ONFBK-003 | onfbk  | Improve layer detection for project variety | Complete | high     |
| ONFBK-004 | onfbk  | Improve entry points presentation           | Complete | medium   |
| ONFBK-005 | onfbk  | Add architecture explanation                | Complete | medium   |

### OPA & Architecture Integration

| Task    | Module | Description                         | Status   | Priority |
| ------- | ------ | ----------------------------------- | -------- | -------- |
| OPA-001 | opa    | Architecture YAML schema (Zod)      | Complete | high     |
| OPA-002 | opa    | YAML parser with template expansion | Complete | high     |
| OPA-003 | opa    | DC config generator from YAML       | Complete | high     |
| OPA-004 | opa    | Architecture init templates (crate API); CLI `init` not registered — see ARCHCFG-007 / CLICT-001 | Complete | high     |
| OPA-005 | opa    | Architecture context extraction     | Complete | high     |
| OPA-006 | opa    | OPA input schema enhancement        | Complete | high     |
| OPA-007 | opa    | Gate runner integration             | Complete | high     |
| OPA-008 | opa    | Rego generator from architecture    | Complete | high     |
| OPA-009 | opa    | Generated policy marker             | Complete | medium   |
| OPA-010 | opa    | Auto-regeneration on YAML change    | Complete | medium   |
| OPA-011 | opa    | Layered architecture template       | Complete | medium   |
| OPA-012 | opa    | Hexagonal architecture template     | Complete | medium   |
| OPA-013 | opa    | Clean Architecture template         | Complete | medium   |
| OPA-014 | opa    | DDD template with bounded contexts  | Complete | medium   |
| OPA-015 | opa    | Template loader and validator       | Complete | medium   |
| OPA-016 | opa    | TypeScript analyser foundation      | Deferred | low      |
| OPA-017 | opa    | Path alias resolver                 | Deferred | low      |
| OPA-018 | opa    | Analyser feature flag               | Deferred | low      |
| OPA-019 | opa    | Bundle download and caching         | Complete | medium   |
| OPA-020 | opa    | Signature verification              | Complete | medium   |
| OPA-021 | opa    | Basic auth and CLI commands         | Complete | medium   |

> **Note:** OPA-016 through OPA-018 were deferred when the OPA module was marked
> Complete at OPA-015. OPA-019 through OPA-021 (remote policy bundles) were
> subsequently implemented. The remaining tasks may be revisited in the OPA
> Enhancements module (OPAE) or a future release.

### Monorepo Migration

| Task     | Module | Description                           | Status   | Priority |
| -------- | ------ | ------------------------------------- | -------- | -------- |
| MONO-001 | mono   | Nx generators for package scaffolding | Complete | high     |
| MONO-002 | mono   | Import path codemod                   | Complete | high     |
| MONO-003 | mono   | Shared tooling packages               | Complete | medium   |
| MONO-004 | mono   | Extract contracts from core           | Complete | high     |
| MONO-005 | mono   | Extract ports from core               | Complete | high     |
| MONO-006 | mono   | Extract pure domain to core           | Complete | high     |
| MONO-007 | mono   | Extract runtime package               | Complete | high     |
| MONO-008 | mono   | Extract policy package                | Complete | high     |
| MONO-009 | mono   | Extract config package                | Complete | medium   |
| MONO-010 | mono   | Extract storage package               | Complete | medium   |
| MONO-011 | mono   | Extract crypto package                | Complete | medium   |
| MONO-012 | mono   | Split adapters per-integration        | Complete | medium   |
| MONO-013 | mono   | Move CLI to apps/                     | Complete | high     |
| MONO-014 | mono   | Reorganise E2E tests                  | Complete | medium   |
| MONO-015 | mono   | Move scripts to tools/                | Complete | low      |
| MONO-016 | mono   | Full test suite validation            | Complete | high     |
| MONO-017 | mono   | Dependency graph validation           | Complete | high     |
| MONO-018 | mono   | Documentation update                  | Complete | medium   |

### APS Markdown Adapter

| Task      | Module | Description                   | Status   | Priority |
| --------- | ------ | ----------------------------- | -------- | -------- |
| APSMD-001 | apsmd  | APSMarkdownAdapter with detection | Complete | high |
| APSMD-002 | apsmd  | Confidence scoring system     | Complete | high     |
| APSMD-003 | apsmd  | Parse method implementation   | Complete | high     |
| APSMD-004 | apsmd  | Task-to-Change conversion     | Complete | high     |
| APSMD-005 | apsmd  | Registry integration          | Complete | high     |
| APSMD-006 | apsmd  | CLI PlanLoader integration    | Complete | high     |

### IDE Integration (VS Code Extension)

| Task    | Module | Description                                            | Status   | Priority |
| ------- | ------ | ------------------------------------------------------ | -------- | -------- |
| IDE-001 | ide    | Embed @eddacraft/anvil-core for fast-path operations   | Complete | high     |
| IDE-002 | ide    | Anti-pattern detection on save with diagnostics        | Complete | high     |
| IDE-003 | ide    | Improve source location mapping from CLI output        | Complete | medium   |
| IDE-004 | ide    | Architecture gate display in tree view                 | Complete | high     |
| IDE-005 | ide    | OPA policy failure display with remediation            | Complete | high     |
| IDE-006 | ide    | Click-to-navigate for all violation types              | Complete | medium   |
| IDE-007 | ide    | APS and Rego syntax highlighting                       | Complete | medium   |
| IDE-008 | ide    | Analysis caching and Marketplace preparation           | Complete | medium   |

### TUI Operational

| Task    | Module | Description                                                                     | Status   | Priority |
| ------- | ------ | ------------------------------------------------------------------------------- | -------- | -------- |
| TUI-006 | tui    | Static template library                                                         | Deferred | medium   |
| TUI-007 | tui    | Interactive tutorial                                                            | Deferred | low      |
| TUI-009 | tui    | `anvil watch` real-time dashboard                                               | Complete | medium   |
| TUI-010 | tui    | `anvil gate` interactive explorer                                               | Deferred | medium   |
| TUI-011 | tui    | Parallel progress visualisation                                                 | Deferred | low      |
| TUI-012 | tui    | Log panel with filtering                                                        | Deferred | low      |
| TUI-013 | tui    | `<MermaidDiagram />` component + `layersToMermaid()` helper                     | Complete | high     |
| TUI-014 | tui    | Replace existing ASCII diagrams with mermaid rendering                          | Complete | high     |
| TUI-015 | tui    | Mermaid helpers shipped; CLI `visualise` not registered — use `anvil dashboard architecture` (ARCHCFG-010 / CLICT-001) | Complete | high     |

### HTML/CSS Support

| Task        | Module  | Description                            | Status   | Priority |
| ----------- | ------- | -------------------------------------- | -------- | -------- |
| HTMLCSS-001 | htmlcss | Make analysable extensions configurable | Complete | high     |
| HTMLCSS-002 | htmlcss | HTML anti-pattern detectors (AP-008-011) | Complete | high   |
| HTMLCSS-003 | htmlcss | CSS anti-pattern detectors (AP-012-013)  | Complete | high   |
| HTMLCSS-004 | htmlcss | HTML/CSS edge detection                | Complete | high     |
| HTMLCSS-005 | htmlcss | HTML suppression comment syntax        | Complete | high     |
| HTMLCSS-006 | htmlcss | VS Code extension HTML/CSS trigger     | Complete | medium   |
| HTMLCSS-007 | htmlcss | Documentation and tests                | Complete | medium   |

### Tutorial Overhaul

| Task    | Module | Description                                         | Status   | Priority |
| ------- | ------ | --------------------------------------------------- | -------- | -------- |
| TUT-001 | tut    | Rewrite tutorial step types for scan-watch-fix flow | Complete | high     |
| TUT-002 | tut    | Create ScanStep TUI component                       | Complete | high     |
| TUT-003 | tut    | Create WatchStep TUI component                      | Complete | high     |
| TUT-004 | tut    | Create FixStep TUI component                        | Complete | high     |
| TUT-005 | tut    | Create NextStepsStep and wire up Tutorial.tsx        | Complete | high     |
| TUT-006 | tut    | Interactive policy creation tutorial                 | Complete | medium   |
| TUT-007 | tut    | Interactive architecture boundaries tutorial         | Complete | medium   |
| TUT-008 | tut    | Interactive drift tracking tutorial                  | Complete | medium   |
| TUT-009 | tut    | Interactive CI integration tutorial                  | Complete | high     |
| TUT-010 | tut    | Docs-site tutorials section                          | Complete | high     |
| TUT-011 | tut    | Rewrite quickstart.md and update navigation          | Complete | high     |
| TUT-012 | tut    | Tutorial --list flag and e2e test                    | Complete | high     |

### Intelligent First Run

| Task    | Module | Description                              | Status   | Priority |
| ------- | ------ | ---------------------------------------- | -------- | -------- |
| IFR-001 | ifr    | Add project context detection service    | Complete | high     |
| IFR-002 | ifr    | Create smart defaults generator          | Complete | high     |
| IFR-003 | ifr    | Add post-init automatic analysis         | Complete | high     |
| IFR-004 | ifr    | Create quick wins identifier             | Complete | high     |
| IFR-005 | ifr    | Create interactive results dashboard TUI | Complete | high     |
| IFR-006 | ifr    | Add historical analysis feature          | Complete | medium   |
| IFR-007 | ifr    | Integrate all components in init flow    | Complete | high     |
| IFR-008 | ifr    | Update documentation                     | Complete | medium   |

### Adapter Upstream Updates

| Task        | Module  | Description                              | Status   | Priority |
| ----------- | ------- | ---------------------------------------- | -------- | -------- |
| ADAPTUP-001 | adaptup | Update BMAD folder structure detection   | Complete | high     |
| ADAPTUP-002 | adaptup | Update BMAD config path handling         | Complete | high     |
| ADAPTUP-003 | adaptup | Update BMAD variable syntax              | Complete | medium   |
| ADAPTUP-004 | adaptup | Add BMAD hasSidecar field support        | Complete | medium   |
| ADAPTUP-005 | adaptup | Update SpecKit command namespace detection | Complete | high   |
| ADAPTUP-006 | adaptup | Add SpecKit AGENTS.md support            | Complete | medium   |
| ADAPTUP-007 | adaptup | Update adapter test fixtures             | Complete | high     |
| ADAPTUP-008 | adaptup | Update adapter documentation             | Complete | medium   |

### AI Tool Integration

| Task       | Module         | Description                       | Status   | Priority |
| ---------- | -------------- | --------------------------------- | -------- | -------- |
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
| MCP-004    | mcp-server     | anvil_fix and anvil_suppress tools | Complete | high    |
| MCP-005    | mcp-server     | anvil_query_boundary tool         | Complete | high     |
| MCP-006    | mcp-server     | Resources with subscriptions      | Complete | medium   |
| MCP-007    | mcp-server     | Prompt templates                  | Complete | medium   |
| MCP-008    | mcp-server     | Streamable HTTP transport         | Complete | medium   |
| MCP-009    | mcp-server     | Config generators and CLI         | Complete | high     |
| MCP-010    | mcp-server     | Error handling and JSON-RPC       | Complete | high     |

### Pulumi Infrastructure as Code

| Task    | Module | Description                              | Status   | Priority |
| ------- | ------ | ---------------------------------------- | -------- | -------- |
| IAC-001 | iac    | Scaffold Pulumi project in monorepo      | Complete | high     |
| IAC-002 | iac    | Configure Pulumi state backend           | Complete | high     |
| IAC-003 | iac    | Manage website Vercel project config     | Complete | high     |
| IAC-004 | iac    | Manage docs-site Vercel project config   | Complete | high     |
| IAC-005 | iac    | Create VercelApp ComponentResource       | Complete | medium   |
| IAC-006 | iac    | Manage GitHub repository configuration   | Deferred | high     |
| IAC-007 | iac    | Manage Azure DNS zones and records       | Complete | high     |
| IAC-008 | iac    | Add Pulumi CI/CD pipeline integration    | Complete | high     |
| IAC-009 | iac    | Write unit tests for infrastructure code | Complete | medium   |
| IAC-010 | iac    | Import existing Vercel resources         | Complete | high     |
| IAC-011 | iac    | Document IaC setup and contributor guide | Complete | medium   |
| IAC-012 | iac    | Document rollback procedures             | Complete | medium   |
