# Open-Spec Adapter

**Scope:** OPENSPEC **Owner:** @team **Priority:** medium **Status:** Draft

**Last reviewed:** 2026-04-26

> **Audit note (2026-04-26):** Module premise still holds — `packages/adapters/`
> remains a TypeScript package owning planning-format adapters (BMAD, SpecKit,
> APS markdown). Open-spec parsing belongs in the same TS adapters layer rather
> than in any Rust crate. References to `@eddacraft/anvil-core` and
> `../base/types.js` are TS-package-internal and remain valid post-migration.

## Purpose

Add support for [open-spec](https://github.com/open-spec/open-spec) format as a
planning document source. Open-spec is an open standard for software
specifications that can be converted to APS plans for execution.

## In Scope / Out of Scope

**In Scope:**

- Parse open-spec format documents
- Detect open-spec format with confidence scoring
- Convert open-spec to APSPlan (core schema)
- Serialise APSPlan back to open-spec format
- Register adapter in format registry

**Out of Scope:**

- Converting to APS markdown format (different abstraction layer)
- Open-spec validation beyond format detection
- Open-spec editing or creation tools

## Interfaces

**Depends on:**

- `@eddacraft/anvil-core` — APSPlan schema, createPlan, generateHash
- `../base/types.js` — BaseFormatAdapter, AdapterMetadata

**Exposes:**

- `OpenSpecFormatAdapter` class
- `createOpenSpecAdapter()` factory function
- `detect()`, `parse()`, `serialize()`, `validate()` methods

## Work Items

### OPENSPEC-001: Research open-spec format structure

**Intent:** Understand the open-spec format to design the parser correctly
**Expected Outcome:** Documentation of open-spec structure and mapping to APSPlan
**Confidence:** high
**Scopes:** research, documentation
**Tags:** research

### OPENSPEC-002: Implement open-spec detection

**Intent:** Detect open-spec format with confidence scoring
**Expected Outcome:** `detect()` method returns confidence score based on format indicators
**Validation:** `pnpm nx run adapters:test -- --grep "OpenSpec.*detect"`
**Confidence:** high
**Scopes:** adapters
**Tags:** parser

### OPENSPEC-003: Implement open-spec parser

**Intent:** Parse open-spec documents to internal representation
**Expected Outcome:** `parseOpenSpecDocument()` extracts structure from open-spec markdown
**Validation:** `pnpm nx run adapters:test -- --grep "OpenSpec.*parse"`
**Confidence:** medium
**Scopes:** adapters
**Dependencies:** OPENSPEC-001, OPENSPEC-002
**Tags:** parser

### OPENSPEC-004: Implement open-spec to APS conversion

**Intent:** Convert parsed open-spec to APSPlan schema
**Expected Outcome:** `openSpecToAPS()` produces valid APSPlan with proposed_changes
**Validation:** `pnpm nx run adapters:test -- --grep "OpenSpec.*APS"`
**Confidence:** medium
**Scopes:** adapters
**Dependencies:** OPENSPEC-003
**Tags:** conversion

### OPENSPEC-005: Implement APS to open-spec serialisation

**Intent:** Serialise APSPlan back to open-spec format
**Expected Outcome:** `serialize()` method produces valid open-spec markdown
**Validation:** `pnpm nx run adapters:test -- --grep "OpenSpec.*serialize"`
**Confidence:** medium
**Scopes:** adapters
**Dependencies:** OPENSPEC-004
**Tags:** serialization

### OPENSPEC-006: Register adapter in format registry

**Intent:** Make open-spec adapter discoverable via format registry
**Expected Outcome:** `FormatRegistry.detect()` includes open-spec in auto-detection
**Validation:** `pnpm nx run adapters:test -- --grep "registry.*openspec"`
**Confidence:** high
**Scopes:** adapters
**Dependencies:** OPENSPEC-005
**Tags:** integration

## Decisions

- **D-001:** Convert to APSPlan (core schema), not APS markdown — aligns with
  BMAD and SpecKit adapters which operate at execution level, not planning level

## Notes

- Similar structure to BMAD and SpecKit adapters
- Will need to research open-spec format structure before implementation
- Consider adding test fixtures from real open-spec documents
