# APS Markdown Adapter

**Scope:** APSMD **Owner:** @team **Priority:** high **Status:** Complete

## Purpose

Convert APS markdown planning documents (`.aps.md` files) to the APSPlan
execution schema. This bridges the human-readable planning format with the
deterministic execution layer, enabling plans written in APS markdown to be
executed by Anvil.

## In Scope / Out of Scope

**In Scope:**

- Parse APS markdown documents (index, module, simple formats)
- Convert Tasks to APSPlan proposed_changes
- Map task metadata (intent, validation, confidence) to change metadata
- Detect APS markdown format with confidence scoring
- Serialise APSPlan back to APS markdown format
- Handle multi-module plans via index files
- Register adapter in format registry

**Out of Scope:**

- APS markdown validation (handled by `@eddacraft/anvil-aps` validator)
- Task state management (handled by `@eddacraft/anvil-aps` state module)
- Execution of plans (handled by core execution layer)

## Interfaces

**Depends on:**

- `@eddacraft/anvil-core` — APSPlan schema, createPlan, generateHash
- `@eddacraft/anvil-aps` — parsePlanningDocument, loadPlan, Task types
- `../base/types.js` — BaseFormatAdapter, AdapterMetadata

**Exposes:**

- `APSMarkdownFormatAdapter` class
- `createAPSMarkdownAdapter()` factory function
- `detect()`, `parse()`, `serialize()`, `validate()` methods

## Tasks

### APSMD-001: Design task-to-change mapping strategy ✅

**Status:** Complete
**Intent:** Define how APS Tasks map to APSPlan proposed_changes
**Expected Outcome:** Documented mapping strategy for all task fields
**Validation:** Review document covers all Task fields from types/index.ts
**Confidence:** high
**Scopes:** design, documentation
**Tags:** design

### APSMD-002: Implement APS markdown detection ✅

**Status:** Complete
**Intent:** Detect APS markdown format with confidence scoring
**Expected Outcome:** `detect()` returns high confidence for .aps.md files with Tasks section
**Validation:** `pnpm nx run adapters:test -- --grep "APSMarkdown.*detect"`
**Confidence:** high
**Scopes:** adapters
**Tags:** parser

### APSMD-003: Implement task-to-change conversion ✅

**Status:** Complete
**Intent:** Convert parsed Tasks to APSPlan proposed_changes
**Expected Outcome:** Each task produces one or more Change objects with appropriate type
**Validation:** `pnpm nx run adapters:test -- --grep "APSMarkdown.*convert"`
**Confidence:** medium
**Scopes:** adapters
**Dependencies:** APSMD-001, APSMD-002
**Tags:** conversion
**Inputs:**

- Task fields: intent, expectedOutcome, validation, scopes, files
- Change types: file_create, file_update, config_update, script_execute

### APSMD-004: Implement multi-module plan support

**Status:** Descoped (leaf specs only for v1.1)
**Intent:** Handle index files that reference multiple module specs
**Expected Outcome:** `parse()` loads and merges all modules from index file
**Validation:** `pnpm nx run adapters:test -- --grep "APSMarkdown.*multi"`
**Confidence:** medium
**Scopes:** adapters
**Dependencies:** APSMD-003
**Tags:** parser

### APSMD-005: Implement APSPlan to APS markdown serialisation

**Status:** Descoped (parse-only for v1.1)
**Intent:** Serialise APSPlan back to APS markdown format
**Expected Outcome:** `serialize()` produces valid .aps.md with Tasks section
**Validation:** `pnpm nx run adapters:test -- --grep "APSMarkdown.*serialize"`
**Confidence:** medium
**Scopes:** adapters
**Dependencies:** APSMD-004
**Tags:** serialization

### APSMD-006: Register adapter in format registry ✅

**Status:** Complete
**Intent:** Make APS markdown adapter discoverable via format registry
**Expected Outcome:** `FormatRegistry.detect()` includes APS markdown in auto-detection
**Validation:** `pnpm nx run adapters:test -- --grep "registry.*aps"`
**Confidence:** high
**Scopes:** adapters
**Dependencies:** APSMD-005
**Tags:** integration

## Decisions

- **D-001:** Task → Change mapping: Change type inferred from task intent keywords
  (create/add → file_create, update/modify/fix → file_update, delete/remove → file_delete,
  config/setting → config_update, default → script_execute)
- **D-002:** Task description format: `{taskId}: {title}\n\n{intent}`
- **D-003:** First file from task.files array used as change.path
- **D-004:** Multi-module support descoped to v2.0 (leaf specs only for v1.1)
- **D-005:** Serialization descoped to v2.0 (parse-only for v1.1)

## Implementation Notes

- Reuses `@eddacraft/anvil-aps` parseDocument() for markdown parsing
- Confidence scoring based on APS indicators (Tasks section, SCOPE-NNN patterns, etc.)
- Detection threshold: 50% confidence minimum
- 24 tests covering detection, parsing, and type inference
- Auto-registered in AdapterRegistry with highest priority (native format)
