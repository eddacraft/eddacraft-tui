<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Pack Validation

| ID  | Owner | Priority | Status |
| ------ | ----- | -------- | ------ |
| POLVAL | —     | high     | Proposed |

**Last reviewed:** 2026-07-04 (POLRESET-002 retarget under ADR-098).

> **Retarget (POLRESET-002 / ADR-098, 2026-07-04):** pack admission code must
> NOT extend `crates/anvil-policy`'s OPA-era `loader.rs`/`library.rs` — those
> modules are deleted by ADR-098 AD-1 PR-C, and the crate itself ultimately
> dissolves under AD-2. New pack metadata/manifest/validator/test-runner code
> lives in the product-path crate, `crates/anvil-policy-engine` (`src/pack/`),
> honouring ADR-040 D-2 validation-before-load at the facade boundary.
> Validation targets updated from `-p eddacraft-anvil-policy` to
> `-p eddacraft-anvil-policy-engine` accordingly.

> **Policy-solution validation (2026-06-24):** POLVAL should validate packs for
> the Rust/regorus policy path. Structural pack checks live in
> `crates/anvil-policy`; execution tests must prove the pack through
> `crates/anvil-policy-engine` / `anvil policy eval` semantics. Go OPA
> (`opa test`) may remain an optional compatibility/reference check for Rego
> syntax, but it is not sufficient completion evidence for a pack that Anvil will
> ship.

## Purpose

Ensure policy packs produced by humans or AI are complete, tested, and safe to
load. Missing tests, metadata gaps, and inconsistent manifests are caught
before gate evaluation so policies do not fail silently.

## In Scope

- Policy metadata schema with required fields (id, title, severity, owner,
  rationale, scope, tags)
- Pack manifest format describing policies, ownership, and intent
- Policy pack validator (structure, metadata completeness, duplicate ids,
  missing files)
- Enforcement of policy tests for each pack
- CLI validation command and gate preflight option
- Machine-readable validation report

## Out of Scope

- Policy authoring wizards or generators
- Remote bundle signing (covered by OPA-020)
- Auto-fixing policy errors

## Interfaces

**Depends on:**

<!-- Audit 2026-04-26: TS core paths superseded by Rust crates per ADR-026; opa-architecture-integration archived. -->
- `crates/anvil-policy/` — Policy loading, storage, manifest validation, and
  pack-level orchestration
- `crates/anvil-policy-engine/` — regorus evaluation facade and result
  semantics for execution tests
- `crates/anvil-kernel/` — Configuration loading

**Exposes:**

- `PolicyPackValidator` — Validation API
- `anvil policy validate` — CLI entry point
- Validation report format for CI and AI tools

## Acceptance Criteria

- [ ] Missing policy tests cause validation failure
- [ ] Missing required metadata fields are reported with rule ids
- [ ] Duplicate policy ids and packages are blocked
- [ ] Manifest references only existing policy files
- [ ] Validation report supports human and JSON output
- [ ] Typical pack validates in < 200ms
- [ ] Gate preflight can block policy evaluation when validation fails

## Work Items

### POLVAL-001: Policy metadata schema

- **Intent:** Define required metadata fields for each policy and pack
- **Expected Outcome:** Schema validates metadata and provides clear errors
- **Scope:** `crates/anvil-policy-engine/src/pack/`
- **Non-scope:** Policy execution logic
- **Files:**
  - `crates/anvil-policy-engine/src/pack/metadata.rs` (new file, including `#[cfg(test)]` unit tests)
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_metadata`
- **Confidence:** high
- **Status:** Done — `cargo test -p eddacraft-anvil-policy-engine -- policy_metadata` passes (14 tests green).

### POLVAL-002: Policy pack manifest loader

- **Intent:** Standardise policy pack manifests and load them consistently
- **Expected Outcome:** Pack metadata is parsed and attached to policy sets
- **Scope:** `crates/anvil-policy-engine/src/pack/`
- **Non-scope:** Validation rules
- **Files:**
  - `crates/anvil-policy-engine/src/pack/manifest.rs` (new file — does NOT extend the doomed OPA-era `loader.rs`; including `#[cfg(test)]` unit tests)
- **Dependencies:** POLVAL-001
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_pack_manifest`
- **Confidence:** high
- **Status:** Done — `cargo test -p eddacraft-anvil-policy-engine -- policy_pack_manifest` passes (12 tests green).

### POLVAL-003: Policy pack validator

- **Intent:** Validate pack structure, metadata completeness, and uniqueness
- **Expected Outcome:** Validator returns issues with severity and guidance
- **Scope:** `crates/anvil-policy-engine/src/pack/`
- **Non-scope:** OPA execution
- **Files:**
  - `crates/anvil-policy-engine/src/pack/validator.rs` (new file, including `#[cfg(test)]` unit tests)
- **Dependencies:** POLVAL-002
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_pack_validator`
- **Confidence:** high
- **Status:** Done — `cargo test -p eddacraft-anvil-policy-engine -- policy_pack_validator` passes (7 tests green).

### POLVAL-004: Policy test enforcement

- **Intent:** Require policy packs to include tests and pass validation
- **Expected Outcome:** Missing or failing tests block pack validation
- **Scope:** `crates/anvil-policy-engine/src/pack/`
- **Non-scope:** Test authoring guidance
- **Files:**
  - `crates/anvil-policy-engine/src/pack/test_runner.rs` (new file — runs pack `*_test.rego` through the regorus facade, closing the gap that only Go OPA could execute pack tests; including `#[cfg(test)]` unit tests)
- **Dependencies:** POLVAL-003
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_test_runner`
- **Confidence:** high

### POLVAL-005: CLI and gate integration

- **Intent:** Make validation available to users and CI
- **Expected Outcome:** `anvil policy validate` runs and gate can preflight
- **Scope:** `crates/anvil-cli/src/commands/`, `crates/anvil-policy/src/`
- **Non-scope:** IDE integration
- **Files:**
  - `crates/anvil-cli/src/commands/policy/` (replaces the `validate` stub that punts to `opa check`, including colocated tests)
  - `crates/anvil-policy-engine/src/pack/validator.rs` (gate preflight hooks)
  - `docs/guides/policy-validation.md`
- **Dependencies:** POLVAL-004
- **Validation:** `cargo test -p eddacraft-anvil -- policy_validate`
- **Confidence:** medium
