# Adapter Upstream Updates

**Scope:** ADAPTUP **Owner:** @team **Priority:** medium **Status:** Draft

## Purpose

Track and implement updates to format adapters based on upstream specification
changes. Both BMAD and SpecKit have had significant updates since our adapters
were built.

## Upstream Changes Summary

### BMAD v6.0.0-alpha.23 (January 2026)

Source: [github.com/bmad-code-org/BMAD-METHOD](https://github.com/bmad-code-org/BMAD-METHOD)

**Breaking Changes:**

| Change | Old | New | Impact |
|--------|-----|-----|--------|
| Folder structure | `.bmad` | `_bmad` | Path detection |
| Config folder | `_cfg` | `_config` | Config parsing |
| Variable syntax | `{project_root}` | `{project-root}` | Template expansion |
| Module config | Various | `module.yaml` | Config loading |

**New Fields:**

- `hasSidecar` — Required boolean in agent validation
- Agent memory in `_bmad/_memory`
- PRD validation as mandatory workflow checkpoint

**Documentation Changes:**

- Diataxis framework adoption
- Site-relative link format

### SpecKit (tracked from upstream main)

Source: [github.com/github/spec-kit](https://github.com/github/spec-kit)

**Architecture Changes:**

- Shifted from prompt-based to agent-first architecture
- Now uses `AGENTS.md` for Copilot workloads
- Hand-offs to VS Code functionality

**New Commands:**

- `/speckit.clarify` — Targeted clarification questions
- `/speckit.analyze` — Cross-artifact discrepancy reporting

**Namespace Changes:**

- All commands now prefixed with `speckit.*` format

**No Breaking Changes** — Backward compatible updates

## In Scope / Out of Scope

**In Scope:**

- Update BMAD adapter for v6.0.0 folder structure
- Add support for new BMAD fields (hasSidecar, agent memory)
- Update SpecKit adapter for agent-first architecture
- Add detection for new command namespace
- Update test fixtures for new formats

**Out of Scope:**

- Full BMAD agent/workflow system support
- SpecKit VS Code hand-off implementation
- BMAD CLI integration

## Tasks

### ADAPTUP-001: Update BMAD folder structure detection

**Intent:** Support both legacy `.bmad` and new `_bmad` folder patterns
**Expected Outcome:** Adapter detects BMAD projects using either folder structure
**Validation:** `pnpm nx run adapters:test -- --grep "BMAD.*folder"`
**Confidence:** high
**Scopes:** adapters
**Tags:** bmad, detection

### ADAPTUP-002: Update BMAD config path handling

**Intent:** Support `_config` folder alongside legacy `_cfg`
**Expected Outcome:** Config loading works with both folder names
**Validation:** `pnpm nx run adapters:test -- --grep "BMAD.*config"`
**Confidence:** high
**Scopes:** adapters
**Dependencies:** ADAPTUP-001
**Tags:** bmad, config

### ADAPTUP-003: Update BMAD variable syntax

**Intent:** Support new `{project-root}` hyphenated variable syntax
**Expected Outcome:** Template expansion handles both underscore and hyphen syntax
**Validation:** `pnpm nx run adapters:test -- --grep "BMAD.*variable"`
**Confidence:** medium
**Scopes:** adapters
**Tags:** bmad, templates

### ADAPTUP-004: Add BMAD hasSidecar field support

**Intent:** Parse and validate new hasSidecar boolean field
**Expected Outcome:** Agent documents with hasSidecar field are correctly parsed
**Validation:** `pnpm nx run adapters:test -- --grep "BMAD.*sidecar"`
**Confidence:** medium
**Scopes:** adapters
**Dependencies:** ADAPTUP-002
**Tags:** bmad, fields

### ADAPTUP-005: Update SpecKit command namespace detection

**Intent:** Detect `speckit.*` prefixed command files
**Expected Outcome:** Adapter recognises both old and new command naming
**Validation:** `pnpm nx run adapters:test -- --grep "SpecKit.*namespace"`
**Confidence:** high
**Scopes:** adapters
**Tags:** speckit, detection

### ADAPTUP-006: Add SpecKit AGENTS.md support

**Intent:** Parse AGENTS.md files as part of SpecKit project detection
**Expected Outcome:** Projects with AGENTS.md detected as SpecKit format
**Validation:** `pnpm nx run adapters:test -- --grep "SpecKit.*agents"`
**Confidence:** medium
**Scopes:** adapters
**Dependencies:** ADAPTUP-005
**Tags:** speckit, detection

### ADAPTUP-007: Update adapter test fixtures

**Intent:** Add test fixtures for new BMAD v6 and SpecKit v0.0.22 formats
**Expected Outcome:** Test coverage includes new format variations
**Validation:** All adapter tests pass with new fixtures
**Confidence:** high
**Scopes:** adapters, tests
**Dependencies:** ADAPTUP-001, ADAPTUP-005
**Tags:** testing

### ADAPTUP-008: Update adapter documentation

**Intent:** Document supported format versions and compatibility
**Expected Outcome:** BMAD_ADAPTER_SPEC.md updated with v6 support notes
**Validation:** Documentation review
**Confidence:** high
**Scopes:** documentation
**Dependencies:** ADAPTUP-007
**Tags:** docs

## Decisions

- **D-001:** Support both old and new formats for backward compatibility
- **D-002:** Prioritise BMAD updates (breaking changes) over SpecKit (compatible)

## Notes

- BMAD changes are more significant due to breaking folder structure changes
- SpecKit changes are additive and don't require immediate updates
- Consider version detection to apply format-specific parsing rules
