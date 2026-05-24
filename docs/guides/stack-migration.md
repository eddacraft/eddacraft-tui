# Stack Migration Guide

| Type  | Authority     | Owner  | Status | Freshness                                                                                                           |
| ----- | ------------- | ------ | ------ | ------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | @aneki | Live   | Last reviewed 2026-05-25 against `packages/edda-stack/src/contracts/` and `packages/edda-stack/src/edda/migration/` |

| Upstream                                                                                                                                                                             | Downstream                                                       |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------- |
| `packages/edda-stack/src/contracts/`, `packages/edda-stack/src/edda/migration/`, `packages/edda-stack/src/contracts/type-mappings.ts`, `packages/edda-stack/src/testing/validators/` | Edda Stack schema changes, migration reviews, provenance testing |

> How to coordinate schema changes across the Edda Stack layers.

## Overview

The Edda Stack uses versioned schemas across three layers:

- **Kindling** - Observation schemas
- **Ember** - Proposal schemas
- **Edda** - Memory schemas

Schema changes require coordination because:

1. Provenance links connect data across layers
2. Type mappings depend on schema compatibility
3. Breaking changes can corrupt provenance chains

## Schema Versioning

Each layer maintains explicit schema versions:

```typescript
// Ember proposal schema
const PROPOSAL_SCHEMA_VERSION = '1.0.0';

// Edda memory schema
const MEMORY_SCHEMA_VERSION = '1.0.0';
```

### Version Format

Versions follow semantic versioning:

- **MAJOR** - Breaking changes (migration required)
- **MINOR** - Backwards-compatible additions
- **PATCH** - Bug fixes, documentation

## Migration Types

### Type 1: Additive Changes (Minor Version)

Adding new optional fields that don't break existing data.

**Example:** Adding `metadata` field to proposals

```typescript
// Before (1.0.0)
interface CandidateProposal {
  id: ProposalId;
  type: ProposalType;
  statement: string;
}

// After (1.1.0)
interface CandidateProposal {
  id: ProposalId;
  type: ProposalType;
  statement: string;
  metadata?: Record<string, unknown>; // New optional field
}
```

**Migration steps:**

1. Update schema definition with new field
2. Update validation to accept both old and new format
3. Bump minor version
4. No data migration needed

### Type 2: Restructuring Changes (Major Version)

Changes that alter existing field structures or semantics.

**Example:** Restructuring confidence from numeric to semantic

```typescript
// Before (1.x)
interface MemoryObject {
  confidence: number; // 0.0-1.0
}

// After (2.x)
interface MemoryObject {
  confidence: 'high' | 'medium' | 'low' | 'inferred';
}
```

**Migration steps:**

1. Create migration script to transform existing data
2. Test migration on copy of production data
3. Update schema definition
4. Run migration script
5. Verify data integrity
6. Bump major version

### Type 3: Cross-Layer Changes

Changes that affect type mappings between layers.

**Example:** Adding new proposal type that maps to new memory type

**Migration steps:**

1. Add new type to Ember schemas
2. Add new type to Edda schemas
3. Update type mapping functions
4. Update validation rules
5. Test promotion workflow with new type
6. Bump versions in both layers

## Migration Protocol

### Pre-Migration Checklist

- [ ] Document the change and its impact
- [ ] Identify affected layers (Kindling, Ember, Edda)
- [ ] Create migration script if data transformation needed
- [ ] Test migration on staging/copy environment
- [ ] Prepare rollback plan
- [ ] Schedule migration window

### Migration Order

Always migrate bottom-up to preserve provenance integrity:

```
1. Kindling (if affected)
   └── Observation schemas
   └── Test observation queries

2. Ember (if affected)
   └── Proposal schemas
   └── Test proposal creation/queries
   └── Test Kindling→Ember type mappings

3. Edda (if affected)
   └── Memory schemas
   └── Test memory creation/queries
   └── Test Ember→Edda type mappings
   └── Verify provenance resolution
```

### Post-Migration Verification

```bash
# Validate stack configuration
anvil stack validate

# Check layer health
anvil stack status

# Verify provenance integrity (if implemented)
anvil stack validate --check-provenance
```

## Writing Migration Scripts

### File Location

```
packages/edda-stack/
└── migrations/
    ├── ember/
    │   └── v1-to-v2.ts
    └── edda/
        └── v1-to-v2.ts
```

### Script Structure

```typescript
// migrations/edda/v1-to-v2.ts

import type { MemoryObject } from '../../contracts/edda-memory.js';

/**
 * Migration: v1 → v2
 *
 * Changes:
 * - Converts confidence from numeric to semantic
 */
export interface MigrationResult {
  success: boolean;
  migrated: number;
  failed: number;
  errors: string[];
}

export function migrateMemoryV1ToV2(memory: MemoryObjectV1): MemoryObjectV2 {
  return {
    ...memory,
    schema_version: '2.0.0',
    confidence: mapConfidenceNumericToSemantic(memory.confidence),
  };
}

function mapConfidenceNumericToSemantic(numeric: number): SemanticConfidence {
  if (numeric >= 0.8) return 'high';
  if (numeric >= 0.5) return 'medium';
  if (numeric >= 0.2) return 'low';
  return 'inferred';
}

export async function runMigration(
  memories: MemoryObjectV1[]
): Promise<MigrationResult> {
  const errors: string[] = [];
  let migrated = 0;
  let failed = 0;

  for (const memory of memories) {
    try {
      const updated = migrateMemoryV1ToV2(memory);
      // Save updated memory
      migrated++;
    } catch (err) {
      errors.push(`Failed to migrate ${memory.id}: ${err}`);
      failed++;
    }
  }

  return {
    success: failed === 0,
    migrated,
    failed,
    errors,
  };
}
```

## Rollback Procedures

### Before Migration

1. Export current data as backup:

```bash
# Export Edda memories
anvil edda export --output backup-$(date +%Y%m%d).json

# Export Ember proposals (if needed)
anvil ember export --output proposals-backup-$(date +%Y%m%d).json
```

2. Record current schema versions:

```bash
anvil stack status --json > schema-versions-before.json
```

### Rolling Back

1. Stop any processes using the stack
2. Restore from backup:

```bash
anvil edda import --input backup-20240115.json --replace
```

3. Revert schema changes in code
4. Restart processes
5. Verify stack health:

```bash
anvil stack validate
```

## Compatibility Rules

### Guaranteed Compatibility

The stack guarantees:

1. **Forward reading** - New code can read old data
2. **Provenance resolution** - Links remain resolvable across versions
3. **Type mapping stability** - Existing mappings don't break

### Breaking Changes Require

1. Explicit migration path documented
2. Migration script provided
3. Major version bump
4. Announcement in changelog

## Testing Migrations

### Unit Tests

```typescript
describe('v1 to v2 migration', () => {
  it('should convert confidence correctly', () => {
    const v1Memory = createV1Memory({ confidence: 0.9 });
    const v2Memory = migrateMemoryV1ToV2(v1Memory);

    expect(v2Memory.confidence).toBe('high');
    expect(v2Memory.schema_version).toBe('2.0.0');
  });

  it('should preserve provenance chain', () => {
    const v1Memory = createV1Memory({
      provenance: {
        /* ... */
      },
    });
    const v2Memory = migrateMemoryV1ToV2(v1Memory);

    expect(v2Memory.provenance).toEqual(v1Memory.provenance);
  });
});
```

### Integration Tests

```typescript
describe('migration integration', () => {
  it('should maintain provenance resolution after migration', async () => {
    // Setup: Create connected data across layers
    const observation = await kindling.createObservation(/* ... */);
    const proposal = await ember.createProposal({
      kindling_sources: [observation.id],
      /* ... */
    });
    const memory = await edda.promoteProposal(proposal);

    // Run migration
    await runMigration([memory]);

    // Verify: Provenance still resolves
    const migrated = await edda.getMemory(memory.id);
    const resolved = await edda.resolveProvenance(migrated.provenance);

    expect(resolved.complete).toBe(true);
  });
});
```

## Common Migration Scenarios

### Scenario 1: Adding New Memory Type

1. Add type to `MemoryType` enum in contracts
2. Add validation rules for new type
3. Update type mapping if new proposal type exists
4. Add fixtures for testing
5. Bump minor version

### Scenario 2: Changing Field Requirements

1. Write migration to populate required field
2. Update schema with new requirement
3. Update validation
4. Run migration
5. Bump major version

### Scenario 3: Renaming Fields

1. Write migration to copy old field to new name
2. Support both field names in validation (transition period)
3. Update code to use new field name
4. Run migration
5. Later: remove old field support
6. Bump major version

## Best Practices

1. **Never modify production data without backup**
2. **Test migrations on realistic data volumes**
3. **Document all breaking changes in CHANGELOG**
4. **Keep migration scripts idempotent when possible**
5. **Prefer additive changes over restructuring**
6. **Coordinate cross-layer changes carefully**
7. **Validate provenance integrity after every migration**

## Related Documentation

- [Edda Stack Architecture](../architecture/edda-stack.md)
- [Stack Integration Plan](../../plans/archive/modules/edda-stack-integration.aps.md)
