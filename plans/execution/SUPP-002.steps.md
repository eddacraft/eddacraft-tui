# Steps: SUPP-002

| Field      | Value                                                            |
| ---------- | ---------------------------------------------------------------- |
| Source     | [../modules/suppressions.aps.md](../modules/suppressions.aps.md) |
| Task(s)    | SUPP-002 — Suppression store                                     |
| Created by | AI                                                               |
| Status     | Completed                                                        |

## Prerequisites

- [x] SUPP-001 complete (suppression parser)

## Context

Manage suppressions in `.anvil/suppressions.json` with provenance tracking.
Provides API for checking if a warning at a given location is suppressed.

## Steps

### 1. Create suppression store module

- **Checkpoint:** `core/src/suppression/store.ts` exists
- **Files:** `core/src/suppression/store.ts`

### 2. Define store schema

```typescript
interface SuppressionStoreData {
  version: 1;
  suppressions: SuppressionRecord[];
  lastUpdated: string; // ISO datetime
}
```

Uses existing `SuppressionRecordSchema` from `antipattern/types.ts`.

### 3. Implement SuppressionStore class

```typescript
class SuppressionStore {
  constructor(anvilDir: string);

  // Load from .anvil/suppressions.json
  load(): Promise<void>;

  // Save to .anvil/suppressions.json
  save(): Promise<void>;

  // Add a suppression record
  add(record: SuppressionRecord): void;

  // Check if warning is suppressed at location
  isSuppressed(
    warningId: string,
    file: string,
    line: number
  ): SuppressionMatch | null;

  // Get all suppressions
  getAll(): SuppressionRecord[];

  // Get expired suppressions (for reporting)
  getExpired(): SuppressionRecord[];

  // Remove expired suppressions
  pruneExpired(): number;
}
```

### 4. Implement suppression matching

Match logic:

1. Find suppressions for the given file
2. Check if warning ID matches
3. Check if line is within scope (line, statement, file)
4. Check if not expired (for time-boxed suppressions)

### 5. Implement expiry logic

- Parse `expiresAt` from ISO 8601 date
- Compare against current date
- Expired suppressions should NOT suppress warnings
- Track expired for drift reporting

### 6. Add tests

- **Checkpoint:** `core/src/suppression/store.test.ts` exists
- **Files:** `core/src/suppression/store.test.ts`

## Acceptance Criteria

- [ ] Store loads/saves from `.anvil/suppressions.json`
- [ ] `isSuppressed()` correctly matches by ID, file, line
- [ ] Expired suppressions do not suppress warnings
- [ ] `getExpired()` returns list of expired suppressions
- [ ] All tests pass
