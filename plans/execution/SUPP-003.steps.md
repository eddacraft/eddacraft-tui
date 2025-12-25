# Steps: SUPP-003

| Field      | Value                                                            |
| ---------- | ---------------------------------------------------------------- |
| Source     | [../modules/suppressions.aps.md](../modules/suppressions.aps.md) |
| Task(s)    | SUPP-003 — Suppression integration                               |
| Created by | AI                                                               |
| Status     | Completed                                                        |

## Prerequisites

- [x] SUPP-001 complete (suppression parser)
- [x] SUPP-002 complete (suppression store)

## Context

Wire suppression checking into the warning analysis pipeline. When
`analyzeFiles()` runs, check each file for inline suppressions and mark matching
warnings as suppressed.

## Steps

### 1. Create suppression service

- **Checkpoint:** `core/src/suppression/service.ts` exists
- **Files:** `core/src/suppression/service.ts`

### 2. Implement SuppressionService

```typescript
class SuppressionService {
  constructor(
    private parser: typeof parseSuppressions,
    private store: SuppressionStore
  )

  // Parse suppressions from file content and update store
  async processFile(filePath: string, content: string): Promise<ParsedSuppression[]>

  // Apply suppressions to warnings, returning modified warnings
  applyToWarnings(warnings: Warning[], file: string): Warning[]

  // Sync inline suppressions to store (for provenance)
  async syncToStore(suppressions: ParsedSuppression[], file: string, gitContext?: GitContext): Promise<void>
}
```

### 3. Integrate with analyzeFiles

In `gate-runner.ts` `analyzeFiles()`:

1. For each file being analysed, parse inline suppressions
2. After collecting warnings, apply suppressions
3. Mark matched warnings with `suppressed` metadata

### 4. Update AnalyzeOptions

Add suppression-related options:

```typescript
interface AnalyzeOptions {
  // ... existing options

  /** Enable suppression checking (default: true) */
  suppressions?: boolean;

  /** Suppression store instance (created if not provided) */
  suppressionStore?: SuppressionStore;
}
```

### 5. Update AnalyzeResult

Add suppression statistics:

```typescript
interface AnalyzeResult {
  // ... existing fields

  /** Suppression statistics */
  suppressionStats?: {
    total: number;
    active: number;
    expired: number;
  };
}
```

### 6. Create index.ts for suppression module

- **Checkpoint:** `core/src/suppression/index.ts` exports all public APIs
- **Files:** `core/src/suppression/index.ts`

### 7. Export from core index

- **Checkpoint:** `core/src/index.ts` exports suppression module
- **Files:** `core/src/index.ts`

### 8. Add integration tests

- **Checkpoint:** Integration tests verify end-to-end suppression flow
- **Files:** `core/src/suppression/integration.test.ts`

## Acceptance Criteria

- [ ] `analyzeFiles()` respects inline suppressions
- [ ] Suppressed warnings have `suppressed` metadata populated
- [ ] Expired suppressions do not suppress (warnings resurface)
- [ ] Suppression stats included in AnalyzeResult
- [ ] All tests pass
