# Steps: ARCH-002

| Field      | Value                                                                          |
| ---------- | ------------------------------------------------------------------------------ |
| Source     | [../modules/architecture-safety.aps.md](../modules/architecture-safety.aps.md) |
| Task(s)    | ARCH-002 — New edge detection                                                  |
| Created by | AI                                                                             |
| Status     | Draft                                                                          |

## Prerequisites

- [ ] ARCH-001 complete (baseline inference)

## Context

`AnalysisResult` in `analyzer.ts` already has:

- `violations: BoundaryViolation[]`
- `newViolations: BoundaryViolation[]`
- `existingViolations: BoundaryViolation[]`

Need to wire this up with actual diff detection.

## Steps

### 1. Create edge-detector module

- **Checkpoint:** `core/src/architecture/edge-detector.ts` exists
- **Files:** `core/src/architecture/edge-detector.ts`

### 2. Implement import extraction from files

- **Checkpoint:** `extractImports(filePath): ImportEdge[]` function works
- **Validate:** `nx test core --testNamePattern="edge-detector"`
- **Pattern:** Use TypeScript compiler API or regex for speed
- **Files:** `core/src/architecture/edge-detector.ts`

### 3. Implement baseline comparison

- **Checkpoint:** `compareToBaseline(currentEdges, baseline): { new, existing }`
  works
- **Files:** `core/src/architecture/edge-detector.ts`

### 4. Add fingerprinting for edge deduplication

- **Checkpoint:** Edges have stable fingerprint (from:to:type hash)
- **Files:** `core/src/architecture/types.ts`

### 5. Wire into analyzer

- **Checkpoint:** `ArchitectureAnalyzer.analyzeChanges(files)` uses edge
  detector
- **Validate:** `nx test core`
- **Files:** `core/src/architecture/analyzer.ts`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module

**Completed by:** —
