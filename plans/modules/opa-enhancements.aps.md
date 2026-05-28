# OPA Enhancements: Delightful Policy-as-Code

| ID    | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| OPAE  | —     | high     | Draft  |

**Last reviewed:** 2026-04-26

> NOTE(post-rust): Rust mapping documented in `plans/index.aps.md` (search
> "REVIEW(post-rust)"). Task file paths below were authored against the
> retired TS tree (`core/src/`, `cli/src/`); when each task moves to Ready
> the implementation lands in Rust crates: `crates/anvil-kernel`,
> `crates/anvil-policy`, `crates/anvil-cli`, `crates/anvil-architecture`,
> Ratatui surfaces in `crates/anvil-tui/src/surfaces/`. Several listed
> dependency modules (`opa-architecture-integration`, `architecture-safety`,
> `tui`) are now archived under `plans/archive/modules/` — capability is
> covered by the Rust kernel/policy crates.

## Purpose

Transform Anvil's OPA functionality from useful to truly impressive by:

1. Eliminating the need to write Rego for 90% of use cases
2. Providing rich pre-built policies users can install with one command
3. Making policy failures helpful rather than frustrating
4. Enabling real-time feedback as developers code

## In Scope

### Tier 1: High Impact, High Wow Factor

- Natural language policy generation (describe in English, generate YAML/Rego)
- Interactive policy debugger (TUI-based step-through)
- Policy impact simulator (what historical PRs would have been blocked?)
- Rich policy library (20+ pre-built policies)

### Tier 2: Differentiated & Professional

- Policy exceptions with audit trail
- Real-time policy watch mode
- Policy drift detection
- Remote policy bundles with signatures

### Tier 3: Polish & Developer Experience

- Smart policy recommendations based on codebase analysis
- Visual policy reports (HTML/TUI)
- PR auto-comments (GitHub/GitLab)
- Policy composition and inheritance

### Custom Architecture Policies (Priority)

- YAML-first architecture rules (no Rego required)
- Layer definitions and dependency rules
- Module boundaries with public API enforcement
- File-level and package-level import restrictions
- Interactive setup wizard
- Visual architecture map

## Out of Scope

- Full OPA runtime replacement (we wrap OPA, not replace it)
- Non-TypeScript/JavaScript language support (future)
  <!-- NOTE(post-rust): scope assumption is now invalid — Anvil is Rust-first
       per ADR-026; revisit language scope when OPAE moves to Ready. -->
- Real-time Rego evaluation on every keystroke (performance)
- Enterprise SSO for remote bundles (Phase 2)

## Interfaces

**Depends on:**

- `opa-architecture-integration` — Core OPA infrastructure
- `architecture-safety` — Dependency-cruiser integration
- `tui` — Interactive components

**Exposes:**

- `PolicyLibrary` — Installable policy packages
- `PolicyWizard` — Interactive policy creation
- `PolicyDebugger` — Step-through evaluation
- `ArchitectureYAML` — Enhanced YAML schema
- `PolicyExceptions` — Exception request/approval workflow
- `PolicyMetrics` — Compliance tracking over time

## Acceptance Criteria

- [ ] Users can define architecture rules in YAML without Rego
- [ ] 20+ policies available via `anvil policy install <name>`
- [ ] Policy failures include clear explanation and fix suggestions
- [ ] `anvil policy watch` provides real-time feedback
- [ ] `anvil policy impact` shows historical PR analysis
- [ ] Exception requests are logged with audit trail
- [ ] PR comments automatically posted on GitHub/GitLab

## Risks & Mitigations

| Risk                              | Mitigation                                |
| --------------------------------- | ----------------------------------------- |
| NLP policy generation inaccurate  | Always show generated YAML for review     |
| Too many policies overwhelm users | Curated categories, smart recommendations |
| Watch mode impacts performance    | Debounce, incremental evaluation          |
| Remote bundles security concerns  | Mandatory signature verification          |
| Users bypass exceptions           | Audit trail, manager notifications        |

## Tasks

### Phase A: YAML-First Architecture Rules

#### OPAE-001: Enhanced architecture YAML schema

- **Intent:** Extend architecture.yaml to support all rule types without Rego
- **Expected Outcome:** Schema supports layers, modules, file rules, import
  restrictions
- **Files:** `core/src/architecture/definition-schema.ts`
- **Validation:** `nx test core --testNamePattern="definition-schema"`
- **Confidence:** high

#### OPAE-002: Module boundary definitions

- **Intent:** Support bounded contexts with public API enforcement
- **Expected Outcome:** YAML modules section with public_api and dependency rules
- **Files:** `core/src/architecture/module-boundaries.ts`
- **Validation:** `nx test core --testNamePattern="module-boundaries"`
- **Confidence:** high

#### OPAE-003: File-level import rules

- **Intent:** Allow glob-pattern file rules for imports
- **Expected Outcome:** file_rules section enforces per-file restrictions
- **Files:** `core/src/architecture/file-rules.ts`
- **Validation:** `nx test core --testNamePattern="file-rules"`
- **Confidence:** high

#### OPAE-004: Package import restrictions

- **Intent:** Control which npm packages can be used in which layers
- **Expected Outcome:** import_rules section with ban, only_in, warn_in
- **Files:** `core/src/architecture/import-rules.ts`
- **Validation:** `nx test core --testNamePattern="import-rules"`
- **Confidence:** high

#### OPAE-005: Interactive architecture wizard

- **Intent:** Guided setup that analyses codebase and suggests layers
- **Expected Outcome:** `anvil architecture init` with TUI wizard
- **Files:** `cli/src/commands/architecture.ts`
- **Validation:** Manual test of wizard flow
- **Confidence:** medium

### Phase B: Policy Library

#### OPAE-006: Policy library infrastructure

- **Intent:** Support installable policy packages from registry
- **Expected Outcome:** `anvil policy install <name>` fetches and installs
- **Files:** `core/src/gate/policy/library.ts`
- **Validation:** `nx test core --testNamePattern="policy-library"`
- **Confidence:** high

#### OPAE-007: Security policy pack (8 policies)

- **Intent:** Pre-built security policies ready to install
- **Expected Outcome:** security-review, no-secrets, dependency-audit, etc.
- **Files:** `core/src/gate/__fixtures__/library/security/`
- **Validation:** All policies pass `opa test`
- **Confidence:** high

#### OPAE-008: Quality policy pack (6 policies)

- **Intent:** Code quality enforcement policies
- **Expected Outcome:** coverage-minimum, complexity-limit, file-length, etc.
- **Files:** `core/src/gate/__fixtures__/library/quality/`
- **Validation:** All policies pass `opa test`
- **Confidence:** high

#### OPAE-009: Scope policy pack (4 policies)

- **Intent:** PR scope and change management policies
- **Expected Outcome:** change-limit, directory-focus, blast-radius, etc.
- **Files:** `core/src/gate/__fixtures__/library/scope/`
- **Validation:** All policies pass `opa test`
- **Confidence:** high

#### OPAE-010: Compliance policy pack (5 policies)

- **Intent:** Regulatory and compliance policies
- **Expected Outcome:** license-check, gdpr-pii, audit-logging, etc.
- **Files:** `core/src/gate/__fixtures__/library/compliance/`
- **Validation:** All policies pass `opa test`
- **Confidence:** medium

#### OPAE-011: Policy browse command

- **Intent:** Interactive browser for available policies
- **Expected Outcome:** `anvil policy browse` shows categories and details
- **Files:** `cli/src/commands/policy.ts`
- **Validation:** Manual test
- **Confidence:** high

### Phase C: Policy Debugging & Explanation

#### OPAE-012: Enhanced violation messages

- **Intent:** Every violation explains why it failed and how to fix
- **Expected Outcome:** Violations include current_state, how_to_fix,
  documentation_url
- **Files:** `core/src/gate/policy/opa-executor.ts`
- **Validation:** `nx test core --testNamePattern="violation-messages"`
- **Confidence:** high

#### OPAE-013: Policy debugger foundation

- **Intent:** Step-through evaluation showing inputs and rule results
- **Expected Outcome:** `anvil policy debug <policy>` shows evaluation trace
- **Files:** `cli/src/commands/policy-debug.ts`
- **Validation:** Manual test with sample policy
- **Confidence:** medium

#### OPAE-014: Interactive debugger TUI

- **Intent:** Full TUI debugger with step/inspect/modify
- **Expected Outcome:** Navigate through evaluation with keyboard controls
- **Files:** `cli/src/tui/policy-debugger.tsx`
- **Validation:** Manual test
- **Confidence:** medium
- **Dependencies:** OPAE-013

### Phase D: Real-Time Feedback

#### OPAE-015: Policy watch mode

- **Intent:** Continuous evaluation as files change
- **Expected Outcome:** `anvil policy watch` shows live results
- **Files:** `cli/src/commands/policy-watch.ts`
- **Validation:** Manual test with file changes
- **Confidence:** medium

#### OPAE-016: Architecture watch mode

- **Intent:** Real-time architecture violation detection
- **Expected Outcome:** `anvil architecture watch` shows layer violations live
- **Files:** `cli/src/commands/architecture-watch.ts`
- **Validation:** Manual test with file changes
- **Confidence:** medium

#### OPAE-017: Watch mode performance optimisation

- **Intent:** Incremental evaluation for large codebases
- **Expected Outcome:** Only re-evaluate affected policies/rules
- **Files:** `core/src/gate/policy/incremental-evaluator.ts`
- **Validation:** Benchmark against full evaluation
- **Confidence:** medium
- **Dependencies:** OPAE-015, OPAE-016

### Phase E: Policy Impact Analysis

#### OPAE-018: Historical PR analysis

- **Intent:** Show what PRs would have been affected by a policy
- **Expected Outcome:** `anvil policy impact <policy>` analyses git history
- **Files:** `core/src/gate/policy/impact-analyser.ts`
- **Validation:** `nx test core --testNamePattern="impact-analyser"`
- **Confidence:** medium

#### OPAE-019: Impact visualisation

- **Intent:** Visual report of policy impact
- **Expected Outcome:** Charts showing blocked/warned PRs by author, directory
- **Files:** `cli/src/tui/impact-report.tsx`
- **Validation:** Manual test
- **Confidence:** medium
- **Dependencies:** OPAE-018

#### OPAE-020: Impact simulation

- **Intent:** Simulate new rules against historical PRs
- **Expected Outcome:** `anvil policy impact --rule "..."` shows would-be results
- **Files:** `core/src/gate/policy/impact-simulator.ts`
- **Validation:** `nx test core --testNamePattern="impact-simulator"`
- **Confidence:** medium
- **Dependencies:** OPAE-018

### Phase F: Natural Language Policies

#### OPAE-021: Policy description parser

- **Intent:** Parse natural language policy descriptions
- **Expected Outcome:** Extract intent, conditions, requirements from English
- **Files:** `core/src/gate/policy/nlp-parser.ts`
- **Validation:** `nx test core --testNamePattern="nlp-parser"`
- **Confidence:** low
- **Risks:** NLP accuracy may vary; always require human review

#### OPAE-022: YAML generation from NLP

- **Intent:** Generate policy YAML from parsed description
- **Expected Outcome:** `anvil policy create` generates YAML from English
- **Files:** `core/src/gate/policy/yaml-generator.ts`
- **Validation:** `nx test core --testNamePattern="yaml-generator"`
- **Confidence:** low
- **Dependencies:** OPAE-021

#### OPAE-023: Policy creation wizard

- **Intent:** Interactive wizard for natural language policy creation
- **Expected Outcome:** TUI guides user from description to working policy
- **Files:** `cli/src/tui/policy-creator.tsx`
- **Validation:** Manual test
- **Confidence:** low
- **Dependencies:** OPAE-022

### Phase G: Exception Management

#### OPAE-024: Exception request system

- **Intent:** Formal process for requesting policy exceptions
- **Expected Outcome:** `anvil exception request <policy>` creates request
- **Files:** `core/src/gate/policy/exceptions.ts`
- **Validation:** `nx test core --testNamePattern="exceptions"`
- **Confidence:** high

#### OPAE-025: Exception approval workflow

- **Intent:** Approvers can approve/reject exceptions
- **Expected Outcome:** CLI and file-based approval tracking
- **Files:** `core/src/gate/policy/exception-approval.ts`
- **Validation:** `nx test core --testNamePattern="exception-approval"`
- **Confidence:** high
- **Dependencies:** OPAE-024

#### OPAE-026: Audit trail

- **Intent:** Cryptographically signed audit log of all exceptions
- **Expected Outcome:** `.anvil/exception-log.json` with full history
- **Files:** `core/src/gate/policy/audit-trail.ts`
- **Validation:** `nx test core --testNamePattern="audit-trail"`
- **Confidence:** high
- **Dependencies:** OPAE-024, OPAE-025

#### OPAE-027: Exception CLI commands

- **Intent:** Full CLI for exception management
- **Expected Outcome:** `anvil exception list|request|approve|history`
- **Files:** `cli/src/commands/exception.ts`
- **Validation:** Manual test
- **Confidence:** high
- **Dependencies:** OPAE-024, OPAE-025, OPAE-026

### Phase H: PR Integration

#### OPAE-028: GitHub PR comments

- **Intent:** Automatically comment on GitHub PRs with policy results
- **Expected Outcome:** GitHub Action posts formatted results
- **Files:** `cli/src/commands/pr-comment.ts`
- **Validation:** Manual test on GitHub PR
- **Confidence:** high

#### OPAE-029: GitLab MR comments

- **Intent:** Automatically comment on GitLab MRs with policy results
- **Expected Outcome:** GitLab CI job posts formatted results
- **Files:** `cli/src/commands/pr-comment.ts`
- **Validation:** Manual test on GitLab MR
- **Confidence:** high

#### OPAE-030: Inline annotations

- **Intent:** Add inline annotations on specific lines
- **Expected Outcome:** GitHub/GitLab shows annotations on violated lines
- **Files:** `core/src/gate/policy/annotation-generator.ts`
- **Validation:** Manual test
- **Confidence:** medium
- **Dependencies:** OPAE-028, OPAE-029

### Phase I: Metrics & Visibility

#### OPAE-031: Compliance metrics collection

- **Intent:** Track policy compliance over time
- **Expected Outcome:** Metrics stored in `.anvil/metrics/`
- **Files:** `core/src/gate/policy/metrics.ts`
- **Validation:** `nx test core --testNamePattern="metrics"`
- **Confidence:** high

#### OPAE-032: Metrics dashboard TUI

- **Intent:** Visual dashboard showing compliance trends
- **Expected Outcome:** `anvil metrics` shows charts and tables
- **Files:** `cli/src/tui/metrics-dashboard.tsx`
- **Validation:** Manual test
- **Confidence:** medium
- **Dependencies:** OPAE-031

#### OPAE-033: Team leaderboards

- **Intent:** Gamification of policy compliance
- **Expected Outcome:** Dashboard shows top contributors by compliance
- **Files:** `cli/src/tui/leaderboard.tsx`
- **Validation:** Manual test
- **Confidence:** medium
- **Dependencies:** OPAE-031

### Phase J: Remote Bundles (Enhanced)

#### OPAE-034: Organisation policy bundles

- **Intent:** Share policies across organisation repositories
- **Expected Outcome:** Bundles synced from central registry
- **Files:** `core/src/gate/policy/bundle-manager.ts`
- **Validation:** `nx test core --testNamePattern="bundle-manager"`
- **Confidence:** high

#### OPAE-035: Bundle versioning

- **Intent:** Version control for policy bundles
- **Expected Outcome:** Pin to bundle versions, automatic updates
- **Files:** `core/src/gate/policy/bundle-versions.ts`
- **Validation:** `nx test core --testNamePattern="bundle-versions"`
- **Confidence:** high
- **Dependencies:** OPAE-034

#### OPAE-036: Bundle inheritance

- **Intent:** Override org policies at repo level
- **Expected Outcome:** Local policies can extend/override bundle policies
- **Files:** `core/src/gate/policy/bundle-inheritance.ts`
- **Validation:** `nx test core --testNamePattern="bundle-inheritance"`
- **Confidence:** medium
- **Dependencies:** OPAE-034

## Implementation Priority

### MVP (4-6 weeks)

1. YAML-first architecture rules (OPAE-001 to OPAE-004)
2. Policy library with 10+ policies (OPAE-006 to OPAE-010)
3. Enhanced violation messages (OPAE-012)
4. Policy watch mode (OPAE-015)

### Phase 2 (4-6 weeks)

5. Interactive wizards (OPAE-005, OPAE-011)
6. Policy debugger (OPAE-013, OPAE-014)
7. Impact analysis (OPAE-018 to OPAE-020)
8. PR comments (OPAE-028, OPAE-029)

### Phase 3 (4-6 weeks)

9. Exception management (OPAE-024 to OPAE-027)
10. Metrics dashboard (OPAE-031 to OPAE-033)
11. Remote bundles enhanced (OPAE-034 to OPAE-036)

### Future

12. Natural language policies (OPAE-021 to OPAE-023)
13. Full IDE integration (LSP)

## Decisions

- **D-020:** YAML-first — 90% of users should never write Rego
- **D-021:** Progressive disclosure — Simple things simple, complex possible
- **D-022:** Self-documenting — Every violation explains itself
- **D-023:** Audit everything — Exceptions must be traceable
- **D-024:** NLP optional — Always show generated config for review

## Notes

- Policy library can grow incrementally; start with most requested
- NLP features are experimental; always require human review
- Watch mode must be performant; use incremental evaluation
- PR comments should be configurable (opt-in per repo)
- Metrics should be privacy-conscious (aggregate, not individual tracking)

## Related Documents

- [OPA Enhancement Vision](../../docs/archive/planning/opa-enhancement-vision.md)
- [Custom Architecture Policies Guide](../../docs/guides/custom-architecture-policies.md)
- [OPA Policy Engine](../../docs/archive/planning/opa-policy-engine.md)
- [OPA Architecture Integration](./opa-architecture-integration.aps.md)

