<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# AI Guardrail Profile

| Scope   | Owner | Priority | Status   | Progress |
| ------- | ----- | -------- | -------- | -------- |
| AIGUARD | —     | high     | Complete | 4/4      |

**Last reviewed:** 2026-04-28

> **Reconciliation note (2026-04-28):** Checked against the Rust
> implementation. AIGUARD remains **Complete 4/4**: `anvil gate --profile ai`
> is the implemented surface, the profile allow-list and strict-config defaults
> live in `crates/anvil-cli/src/commands/gate.rs`, and AIGUARD-002 publishes the
> canonical `anvil.diagnostic.v1` type in
> `crates/anvil-kernel-types/src/diagnostics.rs`. The stale dependency list and
> `--profile ai` / `--ai` ambiguity from the 2026-04-26 audit are resolved below.
>
> **Envelope coordination resolved for this module:**
> `plans/specs/2026-04-26-diagnostic-envelope-coordination.md` is the canonical
> cross-module spec. RTAI, INTD, DRVR, RMCP and later producers should import or
> wrap the shared diagnostic shape rather than defining parallel payloads.

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

- `crates/anvil-cli` — Rust `anvil gate --profile ai` surface, strict-config handling, JSON gate-result envelope
- `crates/anvil-kernel-types` — canonical `Diagnostic` / `anvil.diagnostic.v1` shape
- `crates/anvil-kernel` — parser support for architecture edge extraction
- `crates/anvil-architecture` — import-boundary validation
- `crates/anvil-checks` — secret detection, antipattern scan, command-safety checks
- `crates/anvil-policy` — OPA policy evaluation
- `plans/specs/2026-04-26-diagnostic-envelope-coordination.md` — shared diagnostic-envelope contract for AIGUARD/RTAI/INTD/DRVR/RMCP

**Exposes:**

- `anvil gate --profile ai` CLI entry point
- `AiGuardrailProfile` configuration
- `anvil.gate-result.v1` envelope wrapping `anvil.diagnostic.v1` diagnostics for AI tooling

## Acceptance Criteria

- [x] Single command runs all guardrail checks with one exit code
- [x] Missing or invalid config returns a clear blocking diagnostic
- [x] Output includes rule id, severity, summary, and fix guidance
- [x] JSON output format is documented and stable
- [x] Profile can be enabled without changing default behaviour
- [x] Typical run completes in < 5 seconds for mid-size repos

## Tasks

### AIGUARD-001: Profile definition

- **Status:** Complete
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

- **Status:** Complete
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
- **Validation:** `cargo test -p eddacraft-anvil -- ai_guardrail` and `cargo test -p eddacraft-anvil --test ai_guardrail_profile`
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
