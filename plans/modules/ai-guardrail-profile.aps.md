<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# AI Guardrail Profile

| Scope   | Owner | Priority | Status |
| ------- | ----- | -------- | ------ |
| AIGUARD | —     | high     | Draft  |

## Purpose

Provide a strict, AI-friendly validation profile that bundles architecture,
policy, and antipattern checks into a single command with structured feedback.
This gives teams using external AI tools a predictable safety harness.

## In Scope

- Validation profile that runs architecture, policy, and antipattern checks
- Strict handling of missing or invalid configuration
- Structured diagnostic format with rule ids and remediation hints
- CLI flag to select the profile
- Documentation for AI-assisted workflows

## Out of Scope

- First-party AI integration or MCP server features
- Auto-fixing violations
- Live IDE feedback (handled by IDE module)

## Interfaces

**Depends on:**

- `architecture-safety` — Architecture analysis
- `antipattern-library` — Antipattern checks
- `opa-architecture-integration` — Policy evaluation
- `policy-pack-validation` — Policy validation preflight
- `architecture-config-validation` — Config validation preflight
- `llms-txt-export` — Constraint export for AI tools
- `crates/anvil-kernel` — kernel checks (secret scan, anti-pattern, command safety, architecture invariants)
- `crates/anvil-cli` — Rust CLI `--profile ai` flag

**Exposes:**

- `--profile ai` (or `--ai`) CLI entry point
- `AiGuardrailProfile` configuration
- Structured diagnostics format for AI tooling

## Acceptance Criteria

- [ ] Single command runs all guardrail checks with one exit code
- [ ] Missing or invalid config returns a clear blocking diagnostic
- [ ] Output includes rule id, severity, summary, and fix guidance
- [ ] JSON output format is documented and stable
- [ ] Profile can be enabled without changing default behaviour
- [ ] Typical run completes in < 5 seconds for mid-size repos

## Tasks

### AIGUARD-001: Profile definition

- **Intent:** Define the AI guardrail profile and its default checks
- **Expected Outcome:** Profile describes strict rules and required inputs
- **Scope:** `crates/anvil-cli/src/commands/gate.rs` (profile integration)
- **Non-scope:** IDE integration and external AI tooling
- **Files:**
  - `crates/anvil-cli/src/commands/gate.rs` (ai profile config, including colocated tests)
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil -- ai_guardrail_profile`
- **Confidence:** medium

### AIGUARD-002: Structured diagnostics format

- **Intent:** Standardise diagnostics across policy, architecture, and rules
- **Expected Outcome:** Consistent schema with remediation hints
- **Scope:** `crates/anvil-kernel-types/src/diagnostics.rs`
- **Non-scope:** CLI rendering details
- **Files:**
  - `crates/anvil-kernel-types/src/diagnostics.rs` (including `#[cfg(test)]` unit tests)
- **Dependencies:** AIGUARD-001
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- diagnostic_schema`
- **Confidence:** medium

### AIGUARD-003: CLI profile integration

- **Intent:** Expose the profile via CLI flags
- **Expected Outcome:** `anvil gate --profile ai` runs guardrail profile
- **Scope:** `crates/anvil-cli/src/commands/gate.rs`
- **Non-scope:** IDE integration
- **Files:**
  - `crates/anvil-cli/src/commands/gate.rs` (profile flag)
  - `crates/anvil-cli/src/commands/gate_test.rs`
- **Dependencies:** AIGUARD-001, AIGUARD-002
- **Validation:** `cargo test -p eddacraft-anvil -- gate_profile`
- **Confidence:** medium

### AIGUARD-004: Documentation and examples

- **Intent:** Provide AI workflow guidance and sample outputs
- **Expected Outcome:** Guide shows how to use the profile with external AI
- **Scope:** `docs/guides/`
- **Non-scope:** Marketing copy
- **Files:**
  - `docs/guides/ai-guardrail-profile.md`
- **Dependencies:** AIGUARD-003
- **Validation:** Manual doc review
- **Confidence:** medium
