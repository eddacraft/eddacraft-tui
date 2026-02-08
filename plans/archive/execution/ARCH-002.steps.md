# Steps: ARCH-002

| Field      | Value                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Source     | [../modules/architecture-safety.aps.md](../modules/architecture-safety.aps.md) |
| Task(s)    | ARCH-002 — New edge detection                                                  |
| Created by | AI                                                                             |
| Status     | Completed                                                                      |

## Prerequisites

- [x] ARCH-001 complete (baseline inference)

## Context

`AnalysisResult` in `analyzer.ts` already has:

- `violations: BoundaryViolation[]`
- `newViolations: BoundaryViolation[]`
- `existingViolations: BoundaryViolation[]`

Need to wire this up with actual diff detection.

## Steps

### 1. Create edge-detector module

- **Checkpoint:** `core/src/architecture/edge-detector.ts` exists ✅
- **Files:** `core/src/architecture/edge-detector.ts`

### 2. Implement import extraction from files

- **Checkpoint:** `extractImports(filePath, workspaceRoot): ImportEdge[]`
  function works ✅
- **Checkpoint:** `extractImportsFromFiles(filePaths[], workspaceRoot)` for
  batch extraction ✅
- **Validate:** `pnpm test` — edge-detector.test.ts passes
- **Pattern:** Uses regex for speed (IMPORT_FROM_REGEX, DYNAMIC_IMPORT_REGEX,
  REQUIRE_REGEX)
- **Files:** `core/src/architecture/edge-detector.ts`

### 3. Implement baseline comparison

- **Checkpoint:**
  `compareToBaseline(currentEdges, baseline): BaselineComparison` works ✅
- **Checkpoint:** Returns `{ existing, new, fixed }` violation arrays ✅
- **Files:** `core/src/architecture/edge-detector.ts`

### 4. Add fingerprinting for edge deduplication

- **Checkpoint:** `createEdgeFingerprint(from, to, line)` returns stable SHA-256
  hash ✅
- **Checkpoint:** `fingerprintEdge(edge)` helper exists ✅
- **Checkpoint:** `deduplicateEdges(edges)` function exists ✅
- **Files:** `core/src/architecture/edge-detector.ts`

### 5. Wire into analyzer

- **Checkpoint:** `ArchitectureAnalyzer.classifyViolations()` uses baseline
  comparison ✅
- **Checkpoint:** `filterCrossLayerEdges()` filters by layer boundaries ✅
- **Validate:** `pnpm test` — all architecture tests pass
- **Files:** `core/src/architecture/analyzer.ts`,
  `core/src/architecture/edge-detector.ts`

## Completion

- [x] All checkpoints validated
- [x] Task marked complete in source module

**Completed by:** AI (2025-12-24)
