# BMAD v4 Backward Compatibility

| ID    | Owner | Priority | Status   | Progress |
| ----- | ----- | -------- | -------- | -------- |
| BMAD4 | @team | low      | Proposed | 0/8      |

**Last reviewed:** 2026-04-26 — adapter remains in TS (`packages/adapters/`); no
Rust port planned for the BMAD format adapter.

## Purpose

Add backward compatibility for BMAD v4 (v4.0.0–v4.44.1) documents to the BMAD
format adapter. Currently the adapter supports v6.0.3 (latest) and legacy v5
paths (`.bmad`, `_cfg`). The v4 format has distinct folder structures, agent
definitions, workflow schemas, and team bundle layouts that are not detected or
parsed by the current adapter.

## Background

The BMAD-METHOD project went through significant structural changes between v4
and v6:

- **v4.0.0** (June 2025): `.bmad-core/` directory, `.yml` workflow files, pure
  markdown templates with `[[LLM:]]` / `^^CONDITION^^` / `<<REPEAT>>` markers
- **v4.44.1** (September 2025): `bmad-core/` (no dot prefix), `.yaml` files,
  YAML-based templates, `core-config.yaml` central configuration
- **v6.0.3** (February 2026): `_bmad/` directory, sharded workflows, agent YAML
  schema, module system

## Gap Analysis

### What Already Works for v4

- **Generated markdown output** (PRDs, architecture docs, epics, stories) uses
  the same FR/NFR/US patterns and change log tables — parsing works identically
- **Mustache-style `{{variables}}`** — same syntax across all versions
- **`.yml` file extension** — already in `metadata.extensions`

### What Does NOT Work for v4

| Gap                  | Severity | Details                                                                               |
| -------------------- | -------- | ------------------------------------------------------------------------------------- |
| Folder detection     | HIGH     | `bmad-core/` and `.bmad-core/` not in `BMAD_FOLDERS` or `analyzePath()`               |
| Agent format         | HIGH     | v4 agents are `.md` files with flat YAML (`agent:` + `persona:` + `commands:` at top) |
| Workflow schema      | MEDIUM   | v4 uses `id/type/sequence[]` not `instructions/config_source`                         |
| Team nesting         | MEDIUM   | v4 nests `agents:` inside `bundle:`, v6 has it top-level                              |
| `{root}` variable    | MEDIUM   | `expandVariables()` only handles `{project-root}` / `{project_root}`                 |
| `core-config.yaml`   | LOW      | No parser for v4 project configuration                                                |
| Template markers     | LOW      | `[[LLM:]]`, `^^CONDITION^^`, `<<REPEAT>>` in v4.0.0 templates                        |
| Task/Checklist types | LOW      | No `BMADDocumentType` for v4 task/checklist `.md` files                               |

## In Scope / Out of Scope

**In Scope:**

- Add `bmad-core` and `.bmad-core` folder detection (HIGH)
- Add v4 agent format detection and parsing (HIGH)
- Add v4 workflow format detection (MEDIUM)
- Add v4 team bundle nesting support (MEDIUM)
- Add `{root}` variable expansion (MEDIUM)
- Create v4 test fixtures
- Update BMAD_ADAPTER_SPEC.md with v4 section

**Out of Scope:**

- v4 template markers (`[[LLM:]]`, `^^CONDITION^^`, `<<REPEAT>>`) — these appear
  only in templates, not generated output
- `core-config.yaml` parsing — project configuration, not documents
- v4 task/checklist document types — supporting files, not primary documents
- v4 distribution bundle format (`dist/`)

## Work Items

### BMAD4-001: Add v4 folder constants and path detection

**Intent:** Recognize `bmad-core/` and `.bmad-core/` as BMAD project directories
**Expected Outcome:** `analyzePath()` returns `isBmadFolder: true` for v4 paths;
`BMAD_FOLDERS` includes `PROJECT_V4` and `PROJECT_V4_DOT` constants
**Validation:** `pnpm nx run adapters:test -- --grep "v4.*folder"`
**Confidence:** high
**Scopes:** adapters
**Tags:** bmad, v4, detection

### BMAD4-002: Add v4 agent format detection

**Intent:** Detect v4 agent `.md` files with flat YAML structure
**Expected Outcome:** `isV4AgentContent()` identifies files with top-level
`agent:` + `persona:` + `commands:` keys (without `metadata:` wrapper). Returns
`BMADDocumentType.AGENT` via `identifyDocumentType()`.
**Validation:** `pnpm nx run adapters:test -- --grep "v4.*agent.*detect"`
**Confidence:** medium
**Scopes:** adapters
**Dependencies:** BMAD4-001
**Tags:** bmad, v4, agent

### BMAD4-003: Add v4 agent parser

**Intent:** Parse v4 agent markdown files into `BMADAgentYaml` structures
**Expected Outcome:** `parseV4AgentMd()` maps v4 fields to the existing agent
types: `agent.name/id/title` → `metadata`, `persona.*` → persona,
`commands:` → menu items, `dependencies:` → file references. The parser handles
the flat YAML-in-markdown structure (no `---` front-matter, no `metadata:`
wrapper).
**Validation:** `pnpm nx run adapters:test -- --grep "v4.*agent.*parse"`
**Confidence:** medium
**Scopes:** adapters
**Dependencies:** BMAD4-002
**Tags:** bmad, v4, agent

### BMAD4-004: Add v4 workflow format detection

**Intent:** Detect v4 workflow YAML files with `id/type/sequence` schema
**Expected Outcome:** `isV4WorkflowContent()` identifies files with `id:` +
`name:` + `type:` + `sequence:` top-level keys. Returns
`BMADDocumentType.WORKFLOW` via `identifyDocumentType()`.
**Validation:** `pnpm nx run adapters:test -- --grep "v4.*workflow"`
**Confidence:** medium
**Scopes:** adapters
**Tags:** bmad, v4, workflow

### BMAD4-005: Add v4 team bundle nesting support

**Intent:** Detect v4 team YAML where `agents:` is nested inside `bundle:`
**Expected Outcome:** `isTeamYamlContent()` handles both v6 (top-level `agents:`)
and v4 (nested `bundle.agents:`). `parseTeamYaml()` extracts agents from either
location.
**Validation:** `pnpm nx run adapters:test -- --grep "v4.*team"`
**Confidence:** high
**Scopes:** adapters
**Tags:** bmad, v4, team

### BMAD4-006: Add `{root}` variable expansion

**Intent:** Handle v4's `{root}` path variable alongside v6's `{project-root}`
**Expected Outcome:** `expandVariables()` treats `{root}` as a synonym for
`{project-root}` when the `project-root` key is provided.
**Validation:** `pnpm nx run adapters:test -- --grep "root.*variable"`
**Confidence:** high
**Scopes:** adapters
**Tags:** bmad, v4, templates

### BMAD4-007: Create v4 test fixtures

**Intent:** Add realistic v4 format test fixtures
**Expected Outcome:** Fixtures directory includes: `valid-v4-agent.md` (flat YAML
agent), `valid-v4-workflow.yaml` (sequence-based workflow),
`valid-v4-team.yaml` (nested agents bundle)
**Validation:** All v4 tests pass with new fixtures
**Confidence:** high
**Scopes:** adapters, tests
**Dependencies:** BMAD4-002, BMAD4-004, BMAD4-005
**Tags:** bmad, v4, testing

### BMAD4-008: Update adapter spec and version

**Intent:** Document v4 backward compatibility in BMAD_ADAPTER_SPEC.md
**Expected Outcome:** Spec includes v4 compatibility section with supported
formats, limitations, and version detection approach
**Validation:** Documentation review
**Confidence:** high
**Scopes:** documentation
**Dependencies:** BMAD4-007
**Tags:** docs

## v4 Agent Format Reference

v4 agents are `.md` files whose body is YAML (no `---` front-matter):

```yaml
agent:
  name: Mary
  id: analyst
  title: Business Analyst
  icon: (emoji)
  whenToUse: "..."
  customization: null

persona:
  role: "Insightful Analyst..."
  style: "Analytical, inquisitive..."
  identity: "Strategic analyst..."
  focus: "Research planning..."
  core_principles:
    - "Curiosity-Driven Inquiry..."

commands:
  - help: "Show numbered list..."
  - brainstorm {topic}: "..."

dependencies:
  data:
    - bmad-kb.md
  tasks:
    - create-doc.md
  templates:
    - project-brief-tmpl.yaml
```

Maps to v6 types:

- `agent.name/id/title/icon` → `BMADAgentMetadata`
- `persona.role/style/identity` → `BMADAgentPersona`
- `commands[]` → `BMADMenuItem[]` (trigger = key, description = value)
- `dependencies` → file reference changes

## v4 Workflow Format Reference

```yaml
id: greenfield-fullstack
name: "Greenfield Full-Stack Application Development"
description: "Agent workflow for building full-stack applications..."
type: greenfield
project_types: [web-app, saas, enterprise-app, prototype, mvp]

sequence:
  - agent: analyst
    creates: project-brief.md
  - agent: pm
    creates: prd.md
    requires: project-brief.md
```

## v4 Team Bundle Format Reference

```yaml
bundle:
  name: Team Fullstack
  icon: (emoji)
  description: Team capable of full stack development.
  agents:
    - bmad-orchestrator
    - analyst
    - pm
  workflows:
    - brownfield-fullstack.yaml
    - greenfield-fullstack.yaml
```

Note: `agents:` is nested inside `bundle:` (unlike v6 where it's top-level).

## Decisions

- **D-001:** Map v4 types to existing v6 type interfaces rather than creating
  separate v4 types — reduces surface area and keeps the adapter unified
- **D-002:** Version detection heuristic: check for `metadata:` wrapper (v6) vs
  flat `agent:` (v4) rather than requiring explicit version markers
- **D-003:** Only support v4 document types that map cleanly to existing
  `BMADDocumentType` — skip task/checklist/data as they are supporting files

## Notes

- The v4 generated output documents (PRDs, architecture) already work correctly
  since they use the same markdown conventions as v6
- Priority is low because v6 is the current stable release and most active
  projects have migrated
- Implementation should be purely additive — no changes to existing v6 detection
  or parsing behavior
