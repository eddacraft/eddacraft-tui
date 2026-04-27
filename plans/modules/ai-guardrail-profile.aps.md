<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# AI Guardrail Profile

| Scope   | Owner | Priority | Status   | Progress |
| ------- | ----- | -------- | -------- | -------- |
| AIGUARD | —     | high     | Complete | 4/4      |

**Last reviewed:** 2026-04-26

> **Audit note (2026-04-26):** Tier A — current-release candidate. Council C
> recommended archival on the basis that AIGUARD is "one CLI flag, not a
> module" — that was too dismissive. AIGUARD-002 (stable JSON diagnostic
> schema in `crates/anvil-kernel-types/src/diagnostics.rs`) is the
> strategic piece: it defines what AI tools consume when they invoke
> `anvil gate --profile ai`. That is launch-aligned with RTAI's
> "trust-in-AI-generated-code" thesis — RTAI fires the validation, AI
> tool reads gate-result JSON in the **same envelope shape**.
>
> Earlier audit pass framed the archived dependency planning modules
> (`architecture-safety`, `antipattern-library`,
> `opa-architecture-integration`, `policy-pack-validation`,
> `architecture-config-validation`, `llms-txt-export`) as evidence of
> staleness — that was a misread. Those *planning modules* are archived
> because their work-item lists completed; the equivalent Rust capability
> is **live** in `crates/anvil-kernel` (architecture invariants,
> secret/anti-pattern checks, command safety) and `crates/anvil-policy`
> (policy validation). All 4 task scopes already target Rust crates with
> `cargo test` validations — no rescope work needed.
>
> **Cross-coordination required:** AIGUARD-002 diagnostic envelope must
> share a single schema with RTAI-007 (notification mirror) and INTD-013
> (telemetry control envelope). Whichever module lands first publishes
> the canonical shape; the others reference it. Coordinate before
> implementation.
>
> **Followup work** (tracked separately):
> 1. Update Interfaces "Depends on" block to remove the archived planning
>    modules and rely on the live Rust crate references already present.
> 2. Add cross-reference to RTAI-007 / INTD-013 envelope coordination.
> 3. Confirm `--profile ai` is the right surface name (vs. `--ai`).

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

- [x] Single command runs all guardrail checks with one exit code
- [x] Missing or invalid config returns a clear blocking diagnostic
- [x] Output includes rule id, severity, summary, and fix guidance
- [x] JSON output format is documented and stable
- [x] Profile can be enabled without changing default behaviour
- [x] Typical run completes in < 5 seconds for mid-size repos

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

- **Status:** Complete
- **Intent:** Expose the profile via CLI flags
- **Expected Outcome:** `anvil gate --profile ai` runs guardrail profile
- **Scope:** `crates/anvil-cli/src/commands/gate.rs`
- **Non-scope:** IDE integration
- **Files:**
  - `crates/anvil-cli/src/commands/gate.rs` (allow-list selection, strict-config + JSON-output defaults, diagnostic envelope, command-safety dispatcher)
  - `crates/anvil-cli/src/commands/check_catalog.rs` (command-safety registration in CHECK_DEFINITIONS + GATE_INTERNAL_CHECKS)
  - `crates/anvil-cli/tests/ai_guardrail_profile.rs` (end-to-end JSON envelope assertion)
- **Dependencies:** AIGUARD-001, AIGUARD-002
- **Validation:** `cargo test -p eddacraft-anvil -- ai_guardrail` and the colocated unit tests in `gate.rs`
- **Confidence:** medium

### AIGUARD-004: Documentation and examples

- **Status:** Complete
- **Intent:** Provide AI workflow guidance and sample outputs
- **Expected Outcome:** Guide shows how to use the profile with external AI
- **Scope:** `docs/public/anvil/guides/`
- **Non-scope:** Marketing copy
- **Files:**
  - `docs/public/anvil/guides/ai-guardrail-profile.md`
- **Dependencies:** AIGUARD-003
- **Validation:** Manual doc review
- **Confidence:** medium
