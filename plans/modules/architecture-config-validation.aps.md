<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Architecture Config Validation

| Scope   | Owner | Priority | Status |
| ------- | ----- | -------- | ------ |
| ARCHCFG | —     | high     | Draft  |

## Purpose

Ensure `.anvil/architecture.yaml` is unambiguous, consistent, and safe to apply.
This protects teams who scaffold with AI by rejecting overlapping paths,
undefined layers, and incomplete definitions before analysis runs.

## In Scope

- Semantic validation beyond schema checks
- Detection of overlapping layer paths and ambiguous glob rules
- Validation of layer and module references
- Warnings for empty or unused layers and rules
- Diagnostics mapped to configuration sections
- CLI validation command and gate preflight option

## Out of Scope

- Dependency analysis and gate evaluation
- Auto-fixing configuration issues
- Cross-repo architecture baselines

## Interfaces

**Depends on:**

- `opa-architecture-integration` — Architecture YAML schema and parser
- `architecture-safety` — Layer definitions and baseline usage
- `core/src/config/` — Configuration loading

**Exposes:**

- `ArchitectureConfigValidator` — Validation API
- `anvil architecture validate` — CLI entry point
- Diagnostic report format for CI and AI tools

## Acceptance Criteria

- [ ] Overlapping layer paths are blocked with clear diagnostics
- [ ] Duplicate layer ids or names are blocked
- [ ] Rules referencing unknown layers are blocked
- [ ] Empty layers and unused rules emit warnings
- [ ] Validation report includes section or key identifiers
- [ ] Typical config validates in < 100ms
- [ ] Gate preflight can block architecture checks when validation fails

## Tasks

### ARCHCFG-001: Semantic validation rules

- **Intent:** Define semantic rules for architecture config integrity
- **Expected Outcome:** Validator detects overlaps, duplicates, and unknowns
- **Scope:** `core/src/architecture/`
- **Non-scope:** Gate evaluation
- **Files:**
  - `core/src/architecture/config-validator.ts`
  - `core/src/architecture/config-validator.test.ts`
- **Dependencies:** —
- **Validation:** `nx test core --testNamePattern="ArchitectureConfigValidator"`
- **Confidence:** high

### ARCHCFG-002: Diagnostic mapping

- **Intent:** Surface validation errors with clear configuration locations
- **Expected Outcome:** Errors map to section keys and rule ids
- **Scope:** `core/src/architecture/`
- **Non-scope:** CLI presentation
- **Files:**
  - `core/src/architecture/config-diagnostics.ts`
  - `core/src/architecture/config-diagnostics.test.ts`
- **Dependencies:** ARCHCFG-001
- **Validation:** `nx test core --testNamePattern="ArchitectureConfigDiagnostics"`
- **Confidence:** medium

### ARCHCFG-003: CLI validation command

- **Intent:** Provide a direct validation entry point for users and CI
- **Expected Outcome:** `anvil architecture validate` returns structured output
- **Scope:** `cli/src/commands/`
- **Non-scope:** IDE integration
- **Files:**
  - `cli/src/commands/architecture-validate.ts`
  - `cli/src/commands/architecture-validate.test.ts`
- **Dependencies:** ARCHCFG-001, ARCHCFG-002
- **Validation:** `nx test cli --testNamePattern="architecture validate"`
- **Confidence:** medium

### ARCHCFG-004: Gate preflight integration

- **Intent:** Prevent architecture checks from running on invalid config
- **Expected Outcome:** Gate preflight blocks with validation report
- **Scope:** `core/src/gate/`
- **Non-scope:** Architecture analysis logic
- **Files:**
  - `core/src/gate/checks/architecture-config-validation.check.ts`
  - `core/src/gate/checks/architecture-config-validation.check.test.ts`
- **Dependencies:** ARCHCFG-001
- **Validation:** `nx test core --testNamePattern="ArchitectureConfigPreflight"`
- **Confidence:** medium

### ARCHCFG-005: Documentation and examples

- **Intent:** Explain config validation rules and remediation steps
- **Expected Outcome:** Guide and examples for common failures
- **Scope:** `docs/guides/`
- **Non-scope:** Marketing content
- **Files:**
  - `docs/guides/architecture-config-validation.md`
- **Dependencies:** ARCHCFG-003
- **Validation:** Manual doc review
- **Confidence:** medium
