<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Pack Validation

| Scope  | Owner | Priority | Status |
| ------ | ----- | -------- | ------ |
| POLVAL | —     | high     | Draft  |

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

- `opa-architecture-integration` — Policy loading and OPA execution
- `core/src/config/` — Configuration loading
- `core/src/gate/policy/` — Policy storage and execution

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

## Tasks

### POLVAL-001: Policy metadata schema

- **Intent:** Define required metadata fields for each policy and pack
- **Expected Outcome:** Schema validates metadata and provides clear errors
- **Scope:** `core/src/gate/policy/`
- **Non-scope:** Policy execution logic
- **Files:**
  - `core/src/gate/policy/policy-metadata.ts`
  - `core/src/gate/policy/policy-metadata.test.ts`
- **Dependencies:** —
- **Validation:** `nx test core --testNamePattern="PolicyMetadata"`
- **Confidence:** high

### POLVAL-002: Policy pack manifest loader

- **Intent:** Standardise policy pack manifests and load them consistently
- **Expected Outcome:** Pack metadata is parsed and attached to policy sets
- **Scope:** `core/src/gate/policy/`
- **Non-scope:** Validation rules
- **Files:**
  - `core/src/gate/policy/policy-pack-manifest.ts`
  - `core/src/gate/policy/policy-pack-manifest.test.ts`
- **Dependencies:** POLVAL-001
- **Validation:** `nx test core --testNamePattern="PolicyPackManifest"`
- **Confidence:** high

### POLVAL-003: Policy pack validator

- **Intent:** Validate pack structure, metadata completeness, and uniqueness
- **Expected Outcome:** Validator returns issues with severity and guidance
- **Scope:** `core/src/gate/policy/`
- **Non-scope:** OPA execution
- **Files:**
  - `core/src/gate/policy/policy-pack-validator.ts`
  - `core/src/gate/policy/policy-pack-validator.test.ts`
- **Dependencies:** POLVAL-002
- **Validation:** `nx test core --testNamePattern="PolicyPackValidator"`
- **Confidence:** high

### POLVAL-004: Policy test enforcement

- **Intent:** Require policy packs to include tests and pass validation
- **Expected Outcome:** Missing or failing tests block pack validation
- **Scope:** `core/src/gate/policy/`
- **Non-scope:** Test authoring guidance
- **Files:**
  - `core/src/gate/policy/policy-test-runner.ts`
  - `core/src/gate/policy/policy-test-runner.test.ts`
- **Dependencies:** POLVAL-003
- **Validation:** `nx test core --testNamePattern="PolicyTestRunner"`
- **Confidence:** high

### POLVAL-005: CLI and gate integration

- **Intent:** Make validation available to users and CI
- **Expected Outcome:** `anvil policy validate` runs and gate can preflight
- **Scope:** `cli/src/commands/`, `core/src/gate/`
- **Non-scope:** IDE integration
- **Files:**
  - `cli/src/commands/policy-validate.ts`
  - `core/src/gate/checks/policy-pack-validation.check.ts`
  - `docs/guides/policy-validation.md`
- **Dependencies:** POLVAL-004
- **Validation:** `nx test cli --testNamePattern="policy validate"`
- **Confidence:** medium
