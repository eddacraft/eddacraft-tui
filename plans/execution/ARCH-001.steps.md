# Steps: ARCH-001

| Field      | Value                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Source     | [../modules/architecture-safety.aps.md](../modules/architecture-safety.aps.md) |
| Task(s)    | ARCH-001 — Baseline inference                                                  |
| Created by | AI                                                                             |
| Status     | Completed                                                                      |

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

- **Checkpoint:** `createBaseline()` function exists in baseline.ts ✅
- **Checkpoint:** `ArchitectureAnalyzer.createBaseline(result)` method exists ✅
- **Validate:** `pnpm test` — baseline.test.ts passes
- **Files:** `core/src/architecture/baseline.ts`,
  `core/src/architecture/analyzer.ts`

### 2. Wire analyzer to baseline manager

- **Checkpoint:** `inferBaseline(workspaceRoot)` function exists in analyzer.ts
  ✅
- **Checkpoint:** `BaselineManager.create()` and `save()` methods work ✅
- **Validate:** `pnpm test` — architecture tests pass
- **Files:** `core/src/architecture/baseline.ts`,
  `core/src/architecture/analyzer.ts`

### 3. Add confidence scoring for inferred boundaries

- **Checkpoint:** `DetectionConfidenceSchema` with `high | medium | low` exists
  ✅
- **Checkpoint:** `EntryPoint.confidence` field exists ✅
- **Files:** `core/src/architecture/types.ts`,
  `core/src/architecture/entry-detector.ts`

### 4. Export from index

- **Checkpoint:** `inferBaseline` exported from `core/src/architecture/index.ts`
  ✅
- **Checkpoint:** All baseline utilities exported ✅
- **Validate:** `pnpm typecheck` — passes

## Completion

- [x] All checkpoints validated
- [x] Task marked complete in source module

**Completed by:** AI (2025-12-24)
