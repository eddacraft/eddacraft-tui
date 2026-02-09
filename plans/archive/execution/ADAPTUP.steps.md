<!--
APS Steps: ADAPTUP
==================
Adapter upstream updates for BMAD v6 and SpecKit agent-first architecture.
See: plans/aps-rules.md
-->

# Steps: ADAPTUP

| Field  | Value                                                                                  |
| ------ | -------------------------------------------------------------------------------------- |
| Source | [../modules/adapter-upstream-updates.aps.md](../modules/adapter-upstream-updates.aps.md) |
| Task   | ADAPTUP — Adapter upstream updates (BMAD v6, SpecKit agent-first)                      |
| Status | In Progress                                                                            |

## Prerequisites

- [ ] `@eddacraft/anvil-core` builds successfully
- [ ] `@eddacraft/anvil-adapters` builds successfully
- [ ] Existing adapter tests pass (`pnpm nx run @eddacraft/anvil-adapters:test`)

## Steps

### 1. Extend base types with path detection

- **Checkpoint:** `PathDetectionHint` interface and optional `detectWithPath` in `packages/adapters/src/base/types.ts`
- **Validate:** `pnpm nx run adapters:build`
- **Tasks:** ADAPTUP-001, ADAPTUP-006

### 2. Update BMAD types for v6

- **Checkpoint:** `hasSidecar`, `AGENT` doc type, folder/config constants, extended indicators in `packages/adapters/src/bmad/types.ts`
- **Validate:** `pnpm nx run adapters:build`
- **Tasks:** ADAPTUP-001, ADAPTUP-004

### 3. BMAD utils — folder detection, variable syntax, boolean frontmatter

- **Checkpoint:** `packages/adapters/src/bmad/utils.ts` handles path analysis, both variable syntaxes, boolean YAML values, config folder constants
- **Validate:** `pnpm nx run adapters:test -- --grep "BMAD.*(folder|config|variable)"`
- **Tasks:** ADAPTUP-001, ADAPTUP-002, ADAPTUP-003

### 4. BMAD format adapter — detectWithPath, hasSidecar validation

- **Checkpoint:** `BMADFormatAdapter` has `detectWithPath` method and warns on missing `hasSidecar` in agent docs
- **Validate:** `pnpm nx run adapters:test -- --grep "BMAD.*(folder|sidecar)"`
- **Tasks:** ADAPTUP-001, ADAPTUP-004

### 5. SpecKit — namespace detection and AGENTS.md

- **Checkpoint:** `SpecKitFormatAdapter` detects `speckit.*` namespace patterns and supports `detectWithPath` with AGENTS.md hints
- **Validate:** `pnpm nx run adapters:test -- --grep "SpecKit.*(namespace|agents)"`
- **Tasks:** ADAPTUP-005, ADAPTUP-006

### 6. Registry — path-aware detection

- **Checkpoint:** `AdapterRegistry` has `detectAdapterWithPath` method
- **Validate:** `pnpm nx run adapters:test -- --grep "registry"`
- **Tasks:** ADAPTUP-001, ADAPTUP-006

### 7. Test fixtures and tests

- **Checkpoint:** New fixtures (`valid-v6-prd.md`, `valid-agent.md`, `sample-spec-namespaced.md`) exist with matching test blocks
- **Validate:** `pnpm nx run adapters:test`
- **Tasks:** ADAPTUP-007

### 8. Documentation

- **Checkpoint:** `BMAD_ADAPTER_SPEC.md` has v6 compatibility section
- **Validate:** Documentation review
- **Tasks:** ADAPTUP-008

## Verification

```bash
pnpm nx run adapters:build
pnpm nx run adapters:test
```
