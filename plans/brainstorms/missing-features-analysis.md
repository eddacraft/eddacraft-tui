# Missing Features Analysis

**Date:** 2026-02-03
**Scope:** Gap analysis of Anvil v1.2 against competitive landscape and user expectations

## Executive Summary

Anvil v1.0-v1.2 delivers a solid CLI-first experience for catching architecture
drift and AI anti-patterns in TypeScript/JavaScript projects. However, several
major feature gaps exist that limit adoption beyond individual TS/JS developers.
This document identifies 12 missing capabilities ranked by strategic impact.

---

## Critical Gaps (High Impact, Blocks Adoption)

### 1. Multi-Language Support

**Current state:** Only TypeScript/JavaScript files are analysable
(`ANALYSABLE_EXTENSIONS = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs']` in
`apps/anvil-cli/src/commands/check.ts:15`).

**Why it matters:** Architecture drift is language-agnostic. Teams using Go,
Python, Rust, Java, or C# face the same problems Anvil solves but cannot use it.
This is the single largest barrier to broader adoption.

**What's needed:**
- Language-agnostic import/dependency graph extraction (tree-sitter or LSP-based)
- Per-language boundary detection (package boundaries differ by ecosystem)
- Anti-pattern detectors parameterised by language (e.g., Python `# type: ignore`,
  Go `//nolint`, Java `@SuppressWarnings`)
- Configuration to declare which languages a project uses

**Effort:** Large. Requires a parser abstraction layer and per-language adapters.

---

### 2. Auto-Fix / Remediation

**Current state:** Anvil detects violations but offers no automated fixes. The
only codemod (`tools/codemods/`) rewrites import paths for monorepo migration,
not for violations.

**Why it matters:** Detection without remediation creates alert fatigue.
Developers see warnings but must manually research and fix each one. Competing
tools (ESLint `--fix`, Semgrep autofix, Biome) set the expectation that fixable
issues get fixed automatically.

**What's needed:**
- `anvil fix` command that applies safe, deterministic fixes
- Auto-fix for suppressions (insert suppression comment with time-box)
- Auto-fix for anti-patterns where the fix is unambiguous (e.g., replace
  `@ts-ignore` with `@ts-expect-error`)
- Suggested fix in `anvil explain` output (code diff preview)
- VS Code quick-fix actions (Code Actions API)

**Effort:** Medium. Many anti-patterns have obvious fixes. Architecture
violations require human judgment and should suggest rather than auto-fix.

---

### 3. MCP Server for AI Tool Integration

**Current state:** Planned in `plans/modules/mcp-server.aps.md` but not started.
`llms.txt` export is also planned but not implemented.

**Why it matters:** Anvil's core thesis is making AI-generated code safe. Without
an MCP server, AI tools (Claude, Cursor, Copilot) generate code blind to
architecture constraints. Real-time validation during generation — not just at
save time — would be a significant differentiator and is aligned with Anvil's
mission.

**What's needed:**
- MCP server exposing architecture rules, boundaries, and active suppressions
- Real-time validation endpoint for AI-generated code fragments
- `llms.txt` export of project constraints for AI context windows
- Integration guides for Claude Code, Cursor, and Copilot

**Effort:** Medium. The core analysis engine exists; wrapping it in MCP protocol
is the primary work.

---

### 4. Web Dashboard and API Service

**Current state:** `apps/anvil-api/` and `apps/anvil-ui/` are placeholder
directories. Five dashboard modules are planned (`plans/modules/dashboard-*.aps.md`)
with 40+ tasks, but no working implementation exists.

**Why it matters:** Team leads, architects, and engineering managers need
visibility into architecture health across repositories. A CLI tool is invisible
to leadership. Dashboards enable:
- Drift trend tracking over time
- Cross-repo architecture health comparison
- Policy compliance visibility for audits
- Team-level adoption metrics

**What's needed:**
- REST/GraphQL API service for submitting and querying analysis results
- Persistent storage for historical analysis data
- Dashboard with overview, gates, warnings, drift, and policy views
- Authentication for multi-user access

**Effort:** Large. This is a full application build, though the domain logic
already exists in the core engine.

---

## Major Gaps (Significant Impact, Limits Scale)

### 5. Notification and Webhook System

**Current state:** No external notification support. Evidence is written to local
files only (`apps/anvil-cli/src/services/evidence-writer.ts`). No Slack, Teams,
email, or webhook integrations exist.

**Why it matters:** In team settings, violations need to reach the right people
through the channels they already use. CI comments on PRs are the only
team-visible output today.

**What's needed:**
- Webhook support for arbitrary HTTP endpoints
- Slack/Teams integration (bot or incoming webhook)
- Configurable notification rules (e.g., notify on new boundary violations,
  suppress on anti-pattern warnings)
- Summary digests (daily/weekly architecture health report)

**Effort:** Small-Medium. Straightforward HTTP integration with configurable
triggers.

---

### 6. Remote / Shared Cache

**Current state:** Cache providers are local only — `FileCacheProvider` (disk)
and `MemoryCacheProvider` (in-process) in
`packages/anvil/runtime/src/cache/providers/`. No distributed or shared cache.

**Why it matters:** In CI environments, every pipeline run starts cold. Teams
with large monorepos pay the full analysis cost on every PR. A shared cache
(Redis, S3, or similar) would dramatically reduce CI times and enable
cross-developer cache sharing.

**What's needed:**
- Remote cache provider interface (already has a good provider abstraction)
- S3/GCS-compatible object storage adapter
- Redis adapter for fast shared caching
- Cache key strategy that accounts for file content hash + config version
- `anvil cache --remote` configuration

**Effort:** Medium. The cache provider abstraction exists; adding remote backends
is the main work.

---

### 7. Custom Rule Authoring (User-Defined Rules)

**Current state:** Anti-pattern detectors are built-in (7 patterns). The ESLint
plugin (`packages/eslint-plugin-anvil/`) provides 3 custom ESLint rules. OPA
policies allow custom Rego rules. But there is no general-purpose system for
users to define their own Anvil-native detection rules without writing an adapter
or Rego.

**Why it matters:** Every team has project-specific patterns they want to enforce
(e.g., "services must not import from UI components", "database queries only in
repository layer", "no direct fetch calls outside the API client"). A rule
authoring system would make Anvil extensible without forking.

**What's needed:**
- YAML/JSON rule definition format for common patterns (import restrictions,
  naming conventions, file placement rules)
- Rule templates for frequently requested patterns
- `anvil rules add` command to scaffold custom rules
- Documentation and examples for custom rule authoring
- Plugin loading system for complex rules (JS/TS functions)

**Effort:** Medium. The detection pipeline exists; adding a declarative rule
layer on top is the key work. OPA partially covers this but Rego is a barrier
for many users.

---

### 8. Authentication, RBAC, and Multi-User Support

**Current state:** No authentication, authorization, or user management. Anvil is
purely a single-user local tool.

**Why it matters:** For enterprise adoption, organisations need:
- Role-based access to policy configuration (who can modify architecture rules?)
- Audit trail of who suppressed which violations
- SSO integration
- Team-scoped dashboards

**What's needed:**
- User identity integration (Git author as minimum, SSO for enterprise)
- Role definitions (viewer, developer, architect, admin)
- Permission model for policy modification
- Suppression approval workflows (architect must approve certain suppressions)

**Effort:** Large. Requires API service (gap #4) as a prerequisite. Enterprise
feature that can follow the dashboard.

---

## Moderate Gaps (Quality of Life, Competitive Parity)

### 9. Telemetry and Usage Analytics

**Current state:** No telemetry or usage analytics of any kind.

**Why it matters:**
- Cannot measure adoption against success criteria ("50%+ of developers run
  Anvil on every save")
- Cannot identify which features are used or ignored
- Cannot prioritise roadmap based on actual usage data
- Cannot measure impact ("new cross-boundary edges per sprint decreases by 30%")

**What's needed:**
- Opt-in anonymous usage telemetry (commands run, violation counts, fix rates)
- Local metrics collection even without remote reporting
- `anvil metrics` command for project-level health trends
- Integration with dashboard for team-level metrics

**Effort:** Small-Medium. Standard telemetry patterns with privacy-first design.

---

### 10. Incremental Adoption Tooling

**Current state:** `anvil init` provides a good first-run experience, but there's
no guided path for large existing codebases with hundreds of pre-existing
violations.

**Why it matters:** Large projects cannot adopt Anvil all at once. They need:
- A way to baseline all existing violations and only alert on new ones
- Gradual rollout by directory/package/team
- "Ratchet" mode that prevents violation count from increasing without requiring
  all existing violations to be fixed

**What's needed:**
- `anvil baseline --snapshot` to capture current state as accepted
- Ratchet mode: fail only if violation count increases above baseline
- Directory/package scoping (`anvil check --scope packages/api/`)
- Adoption progress reporting ("142/500 violations resolved, 28% clean")
- Migration guide for large codebases

**Effort:** Small-Medium. Baseline infrastructure partially exists; ratchet mode
and scoping are incremental additions.

---

### 11. Sarif / Standard Output Formats

**Current state:** Output is CLI text and JSON. No evidence of SARIF, SPDX,
CodeClimate, or other standard formats.

**Why it matters:** Enterprise CI/CD pipelines and security dashboards (GitHub
Advanced Security, SonarQube, Snyk) consume SARIF format. Without it, Anvil
results cannot be surfaced in existing security/quality dashboards that teams
already use.

**What's needed:**
- SARIF output format (`anvil check --format sarif`)
- CodeClimate format for GitLab integration
- JUnit XML format for CI test result integration
- GitHub Code Scanning integration via SARIF upload

**Effort:** Small. Output formatting is mechanical; SARIF schema is well-documented.

---

### 12. Configuration Inheritance and Sharing

**Current state:** Configuration is per-project (`.anvilrc` or `anvil.config.*`).
Org-level policy hierarchy is planned (`plans/modules/org-policy-hierarchy.aps.md`)
but not implemented.

**Why it matters:** Organisations with many repositories need consistent policies
without copy-pasting configuration. A shared config system (like ESLint's
`extends`) would allow:
- Organisation-wide baseline rules
- Team-level overrides
- Repository-specific additions
- Publishable config packages (`@myorg/anvil-config`)

**What's needed:**
- `extends` field in configuration to inherit from packages or URLs
- Shareable config package format
- Config merging strategy (override vs extend vs lock)
- `anvil config validate` to check merged configuration

**Effort:** Medium. Configuration loading exists; inheritance and merging logic
is the new work.

---

## Summary Matrix

| # | Feature                       | Impact   | Effort | Prerequisite For      |
|---|-------------------------------|----------|--------|-----------------------|
| 1 | Multi-language support         | Critical | Large  | Broader adoption      |
| 2 | Auto-fix / remediation         | Critical | Medium | Reduced alert fatigue |
| 3 | MCP server for AI tools        | Critical | Medium | AI-safety mission     |
| 4 | Web dashboard and API          | Critical | Large  | Team visibility       |
| 5 | Notification / webhook system  | Major    | Small  | Team workflows        |
| 6 | Remote / shared cache          | Major    | Medium | CI performance        |
| 7 | Custom rule authoring          | Major    | Medium | Extensibility         |
| 8 | Auth / RBAC / multi-user       | Major    | Large  | Enterprise adoption   |
| 9 | Telemetry / usage analytics    | Moderate | Small  | Success measurement   |
| 10| Incremental adoption tooling   | Moderate | Small  | Large codebase onboard|
| 11| SARIF / standard output formats| Moderate | Small  | Enterprise CI/CD      |
| 12| Config inheritance / sharing   | Moderate | Medium | Org-wide consistency  |

## Recommended Priority Order

**Immediate (v1.3):** #2 Auto-fix, #11 SARIF output, #10 Incremental adoption
- These are low-to-medium effort and directly improve the existing CLI experience.

**Next (v1.4):** #3 MCP server, #5 Notifications, #7 Custom rules
- These extend Anvil's reach into AI workflows and team settings.

**Platform (v2.0):** #4 Dashboard/API, #6 Remote cache, #9 Telemetry
- These require infrastructure investment but unlock team and org-level value.

**Enterprise (v2.1+):** #1 Multi-language, #8 Auth/RBAC, #12 Config inheritance
- These are large investments that make sense once the platform layer exists.
