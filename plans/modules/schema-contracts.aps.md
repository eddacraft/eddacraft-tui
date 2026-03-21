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
- `crates/anvil-kernel-types` — Rust type definitions
- `packages/anvil/core` — schema validation, JSON schema generation

**Exposes:**

- Schema evolution policy
- Breaking change checklist
- Cross-language type parity validation

## Estimated Scope

- **Effort:** 1 week

## Tasks

- SCHEMA-001: Schema evolution policy and breaking change definition
- SCHEMA-002: Cross-language type parity validation framework
- SCHEMA-003: Golden hash management automation
- SCHEMA-004: Schema version compatibility matrix
- SCHEMA-005: Contract testing (TS ↔ Rust output parity)
- SCHEMA-006: Breaking change migration guide template
