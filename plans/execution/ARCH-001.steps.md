# Steps: ARCH-001

| Field      | Value                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Source     | [../modules/architecture-safety.aps.md](../modules/architecture-safety.aps.md) |
| Task(s)    | ARCH-001 — Baseline inference                                                  |
| Created by | AI                                                                             |
| Status     | Draft                                                                          |

## Prerequisites

- [x] CORE-002 complete (analyzeFiles method)
- [x] Existing `core/src/architecture/` infrastructure reviewed

## Context

Existing infrastructure in `core/src/architecture/`:

- `analyzer.ts` — ArchitectureAnalyzer with layer detection
- `baseline.ts` — BaselineManager for .anvil/architecture.json
- `layer-detector.ts` — Detects
  presentation/application/domain/infrastructure/shared
- `entry-detector.ts` — Detects entry points (index, main, routes, etc.)
- `types.ts` — ArchitectureBaseline, Layers, Boundary schemas

## Steps

### 1. Add baseline generation from analysis

- **Checkpoint:** `generateBaseline(analysisResult)` function exists
- **Validate:** `nx test core --testNamePattern="baseline"`
- **Files:** `core/src/architecture/baseline.ts`

### 2. Wire analyzer to baseline manager

- **Checkpoint:** `BaselineManager.inferFromCodebase(workspaceRoot)` method
  exists
- **Validate:** `nx test core`
- **Files:** `core/src/architecture/baseline.ts`

### 3. Add confidence scoring for inferred boundaries

- **Checkpoint:** Each boundary has `confidence: high | medium | low`
- **Files:** `core/src/architecture/types.ts`,
  `core/src/architecture/analyzer.ts`

### 4. Export from index

- **Checkpoint:** `inferBaseline` exported from `core/src/architecture/index.ts`
- **Validate:** `pnpm typecheck`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module

**Completed by:** —
