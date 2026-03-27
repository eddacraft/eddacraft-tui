---
id: database-migration
name: Database Migration
description: Create and apply database schema migration
category: database
tags: [database, migration, schema, sql, orm]
variables:
  - name: migration_name
    description: Name of the migration
    required: true
  - name: table_name
    description: Table to modify
    required: true
  - name: operation
    description: Migration operation type
    default: alter
    required: false
---

# Database Migration: {{ migration_name }}

## Intent

Create database migration to {{ operation }} the {{ table_name }} table with
proper rollback support.

## Changes

### 1. Create Migration File

- **File**: `migrations/{{ migration_name }}.ts`
- **Action**: Create
- **Description**: Migration with up() and down() methods

### 2. Update Schema Types

- **File**: `src/types/database.types.ts`
- **Action**: Modify
- **Description**: Update TypeScript types for {{ table_name }}

### 3. Update Model

- **File**: `src/models/{{ table_name }}.model.ts`
- **Action**: Modify
- **Description**: Reflect schema changes in model

### 4. Add Migration Tests

- **File**: `migrations/__tests__/{{ migration_name }}.test.ts`
- **Action**: Create
- **Description**: Test migration up and down operations

## Migration Structure

```typescript
export async function up(db: Database): Promise<void> {
  // Apply schema changes
}

export async function down(db: Database): Promise<void> {
  // Rollback schema changes
}
```

## Commands

```bash
# Apply migration
npm run migrate:up

# Rollback migration
npm run migrate:down

# Check migration status
npm run migrate:status
```

## Pre-Migration Checklist

- [ ] Database backed up
- [ ] Migration tested locally
- [ ] Rollback tested
- [ ] Team notified of downtime (if required)
- [ ] CI/CD pipeline updated

## Post-Migration Verification

- [ ] Schema changes applied correctly
- [ ] Existing data preserved
- [ ] Application functionality verified
- [ ] Performance acceptable
- [ ] Monitoring checked

## Rollback Plan

If migration fails:

1. Run `npm run migrate:down`
2. Restore from backup if necessary
3. Investigate failure cause
4. Fix and re-attempt

## Acceptance Criteria

- [ ] Migration applies without errors
- [ ] Rollback works correctly
- [ ] Data integrity preserved
- [ ] TypeScript types updated
- [ ] Tests passing
