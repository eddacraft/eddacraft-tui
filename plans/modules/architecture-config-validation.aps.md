<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Architecture Config Validation

| ID   | Owner | Priority | Status |
| ------- | ----- | -------- | ------ |
| ARCHCFG | —     | high     | Draft  |

**Last reviewed:** 2026-04-26

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

<!-- Audit 2026-04-26: opa-architecture-integration and architecture-safety archived; their work landed in crates/anvil-architecture and crates/anvil-policy. -->
- `crates/anvil-architecture` — Architecture YAML schema, parser, layer definitions, and baseline
- `crates/anvil-kernel` — kernel architecture config loading (KERN-030)
- `crates/anvil-cli` — Rust CLI commands

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

## Work Items

### ARCHCFG-001: Semantic validation rules

- **Intent:** Define semantic rules for architecture config integrity
- **Expected Outcome:** Validator detects overlaps, duplicates, and unknowns
- **Scope:** `crates/anvil-kernel/src/policy/config.rs` (extends KERN-030 loader)
- **Non-scope:** Gate evaluation
- **Files:**
  - `crates/anvil-kernel/src/policy/config_validator.rs` (including `#[cfg(test)]` unit tests)
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil-kernel -- architecture_config_validator`
- **Confidence:** high

### ARCHCFG-002: Diagnostic mapping

- **Intent:** Surface validation errors with clear configuration locations
- **Expected Outcome:** Errors map to section keys and rule ids
- **Scope:** `crates/anvil-kernel/src/policy/config_diagnostics.rs`
- **Non-scope:** CLI presentation
- **Files:**
  - `crates/anvil-kernel/src/policy/config_diagnostics.rs` (including `#[cfg(test)]` unit tests)
- **Dependencies:** ARCHCFG-001
- **Validation:** `cargo test -p eddacraft-anvil-kernel -- architecture_config_diagnostics`
- **Confidence:** medium

### ARCHCFG-003: CLI validation command

- **Intent:** Provide a direct validation entry point for users and CI
- **Expected Outcome:** `anvil architecture validate` returns structured output
- **Scope:** `crates/anvil-cli/src/commands/architecture.rs`
- **Non-scope:** IDE integration
- **Files:**
  - `crates/anvil-cli/src/commands/architecture.rs` (validate subcommand, including colocated tests)
- **Dependencies:** ARCHCFG-001, ARCHCFG-002
- **Validation:** `cargo test -p eddacraft-anvil -- architecture_validate`
- **Confidence:** medium

### ARCHCFG-004: Gate preflight integration

- **Intent:** Prevent architecture checks from running on invalid config
- **Expected Outcome:** Gate preflight blocks with validation report
- **Scope:** `crates/anvil-cli/src/commands/gate.rs`
- **Non-scope:** Architecture analysis logic
- **Files:**
  - `crates/anvil-cli/src/commands/gate.rs` (preflight integration, including colocated tests)
- **Dependencies:** ARCHCFG-001
- **Validation:** `cargo test -p eddacraft-anvil -- architecture_config_preflight`
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
