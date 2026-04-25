<!--
APS Module: Schema & Contracts
====================================
Schema evolution strategy for packages/anvil/contracts.
See: plans/aps-rules.md
-->

# Schema & Contracts

| ID     | Owner | Status    |
| ------ | ----- | --------- |
| SCHEMA | —     | Draft |

**Last reviewed:** 2026-04-26

## Purpose

Govern schema evolution in `packages/anvil/contracts` and
`crates/anvil-kernel-types`. Establish breaking-change governance, schema
generation workflow, golden hash management, and cross-language type
generation (TypeScript ↔ Rust).

**Problem:** The contracts package defines the core schemas (APS, gate results,
changes, etc.) via Zod. Schema changes propagate to JSON schema, golden test
hashes, and the Rust kernel types — but there's no module governing this
process. The AGENTS.md says "Run generate:schema + update-golden-hashes after
schema changes" but there's no plan for who owns this or how breaking changes
are handled.

## In Scope

- **Schema evolution policy:** What constitutes a breaking change in schemas
- **Breaking-change governance:** Migration path for schema consumers
- **Schema generation workflow:** `generate:schema` and `update-golden-hashes`
- **Cross-language types:** TypeScript Zod ↔ Rust serde type parity
- **Golden hash management:** When to update, validation strategy
- **Version compatibility:** Schema versioning and backward compatibility
- **Contract testing:** Validate that TS and Rust types produce identical outputs

## Out of Scope

- Individual schema implementation (covered by each feature module)
- API versioning (covered by API governance)

## Interfaces

**Depends on:**

- `packages/anvil/contracts` — Zod schema definitions
- `crates/anvil-kernel-types` — Rust type definitions (serde-parity with Zod)
- `crates/anvil-kernel` — kernel runtime (watcher/parser/graph/policy engine)
- `packages/anvil/core` — schema validation, JSON schema generation

**Exposes:**

- Schema evolution policy
- Breaking change checklist
- Cross-language type parity validation

## Estimated Scope

- **Effort:** 1 week

## Tasks

### SCHEMA-001: Schema evolution policy and breaking change definition

- **Intent:** Define what constitutes a breaking change in schemas
- **Expected Outcome:** Policy document detailing TS Zod ↔ Rust serde parity rules
- **Scope:** `packages/anvil/contracts/src/` and `crates/anvil-kernel-types/src/`
- **Validation:** Policy documented in docs/guides/schema-evolution.md
- **Confidence:** high

### SCHEMA-002: Cross-language type parity validation framework

- **Intent:** Validate that TypeScript Zod types and Rust serde types produce identical outputs
- **Expected Outcome:** Framework runs both parsers on sample data, diffs results
- **Scope:** `packages/anvil/core/src/validation/parity.ts` and `crates/anvil-kernel-types/tests/`
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- parity`
- **Confidence:** high

### SCHEMA-003: Golden hash management automation

- **Intent:** Automate golden hash updates when schemas change
- **Expected Outcome:** `cargo test` and `pnpm test` update hashes atomically
- **Scope:** Build scripts in both workspaces
- **Validation:** `cargo test` with --locked flag passes
- **Confidence:** high

### SCHEMA-004: Schema version compatibility matrix

- **Intent:** Track which schema versions are compatible with which kernel versions
- **Expected Outcome:** Matrix documented; migration paths defined
- **Scope:** `crates/anvil-kernel-types/src/version.rs`
- **Validation:** Compatibility matrix in docs/guides/schema-compatibility.md
- **Confidence:** high

### SCHEMA-005: Contract testing (TS ↔ Rust output parity)

- **Intent:** Test that TS and Rust types serialise/deserialise identically
- **Expected Outcome:** Contract tests pass for all shared types
- **Scope:** `crates/anvil-kernel-types/tests/parity.rs`
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types`
- **Confidence:** high

### SCHEMA-006: Breaking change migration guide template

- **Intent:** Standardise migration guides when schemas change
- **Expected Outcome:** Template available in docs/guides/
- **Scope:** `docs/guides/schema-migration-template.md`
- **Validation:** Template exists and follows format
- **Confidence:** high
