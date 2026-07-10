<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Pack Validation

| ID  | Owner | Priority | Status |
| ------ | ----- | -------- | ------ |
| POLVAL | —     | high     | Done |

**Last reviewed:** 2026-07-11 (post-POLRESET downstream coherence review —
`plans/reviews/2026-07-11-polreset-downstream-coherence.md`: all five items
Done, module advances to Done. The open gate-preflight acceptance criterion is
resolved as satisfied: install-time admission (OPAE-004) plus the gate's
fail-fast compile admission (ADR-098 AD-1 PR-B) deliver its intent; a
manifest-level preflight inside `anvil gate` itself remains available as
future intake, not owed by this module).

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
> `crates/anvil-policy-engine` (`src/pack/`, per the 2026-07-04 retarget
> above; this note originally said `crates/anvil-policy`, corrected
> 2026-07-11); execution tests must prove the pack through
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

- Policy authoring wizards or generators — deferred, not rejected (2026-07-06):
  the exclusion targets a generator that produces *novel* custom policy logic
  from user intent (an authoring wizard / NL-to-Rego assistant), which is a
  substantially harder problem than pack scaffolding. The prerequisite this
  once waited on — a real base pack to generate from — is now satisfied by
  CPACKS' `anvil-baseline` starter pack (POLRESET-007). Revisit only as a
  deliberate scope change here (or in OPAE, which separately excludes
  natural-language policy generation), not a drive-by addition.
- Remote bundle signing — the OPA-020 item this once deferred to lives in the
  archived `opa-architecture-integration` module (its OPA-subprocess
  architecture was deleted by ADR-098 PR-C); distribution/signing is future
  POLFED-adjacent intake with no live owner
- Auto-fixing policy errors

## Interfaces

**Depends on:**

<!-- Audit 2026-04-26: TS core paths superseded by Rust crates per ADR-026; opa-architecture-integration archived. -->
- `crates/anvil-policy-engine/` — pack discovery, manifest loading, validation,
  test enforcement (`src/pack/`), plus the regorus evaluation facade and result
  semantics for execution tests
- `crates/anvil-kernel/` — Configuration loading

**Exposes:**

- `PolicyPackValidator` — Validation API
- `anvil policy validate` — CLI entry point
- Validation report format for CI and AI tools

## Acceptance Criteria

- [x] Missing policy tests cause validation failure (POLVAL-004 test_runner, 8
      tests green)
- [x] Missing required metadata fields are reported with rule ids (POLVAL-001,
      14 tests green)
- [x] Duplicate policy ids and packages are blocked (POLVAL-003 validator, 7
      tests green)
- [x] Manifest references only existing policy files (POLVAL-002 manifest, 12
      tests green)
- [x] Validation report supports human and JSON output (POLVAL-005
      `anvil policy validate`, 6 tests green)
- [ ] Typical pack validates in < 200ms — never benchmarked; no recorded
      evidence. Non-blocking: carry as later intake if pack sizes grow
- [x] Gate preflight can block policy evaluation when validation fails —
      satisfied by install-time admission (OPAE-004) plus the gate's fail-fast
      compile admission (ADR-098 AD-1 PR-B); recorded 2026-07-11

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
- **Status:** Done — `cargo test -p eddacraft-anvil-policy-engine -- policy_test_runner` passes (8 tests green).

### POLVAL-005: CLI and gate integration

- **Intent:** Make validation available to users and CI
- **Expected Outcome:** `anvil policy validate` runs and gate can preflight
- **Scope:** `crates/anvil-cli/src/commands/`, `crates/anvil-policy-engine/src/`
- **Non-scope:** IDE integration
- **Files:**
  - `crates/anvil-cli/src/commands/policy/` (replaces the `validate` stub that punts to `opa check`, including colocated tests)
  - `crates/anvil-policy-engine/src/pack/validator.rs` (gate preflight hooks)
  - `docs/guides/policy-validation.md`
- **Dependencies:** POLVAL-004
- **Validation:** `cargo test -p eddacraft-anvil -- policy_validate`
- **Confidence:** medium
- **Status:** Done — `cargo test -p eddacraft-anvil -- policy_validate` passes (6 tests green). CLI command + `docs/guides/policy-validation.md` shipped; gate preflight deferred to the OPAE-003 PR-B gate repoint (not wired here). 2026-07-11: PR-B landed with fail-fast compile admission at the gate; with install-time admission (OPAE-004) this closes the deferral — see the acceptance-criteria note.
