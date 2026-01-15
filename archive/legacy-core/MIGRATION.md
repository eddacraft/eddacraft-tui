# Migration Guide: From Manual JSON to APS Core

This guide helps you migrate from manually creating and managing APS JSON files
to using the APS Core library.

## Table of Contents

- [Why Migrate?](#why-migrate)
- [Quick Start](#quick-start)
- [Migration Paths](#migration-paths)
- [Step-by-Step Migration](#step-by-step-migration)
- [Common Patterns](#common-patterns)
- [Validation Migration](#validation-migration)
- [Troubleshooting](#troubleshooting)

---

## Why Migrate?

Using the APS Core library instead of manual JSON provides several benefits:

### Type Safety

- **Before**: No TypeScript types, easy to make mistakes
- **After**: Full TypeScript support with autocomplete and type checking

### Automatic Hash Generation

- **Before**: Manually calculate SHA-256 hashes
- **After**: Automatic hash generation with `generateHash()`

### Schema Validation

- **Before**: Manual validation or external tools
- **After**: Built-in validation with detailed error messages

### Consistency

- **Before**: Easy to forget required fields or use wrong formats
- **After**: Helpers like `createPlan()` ensure correct structure

### Maintainability

- **Before**: Hard to refactor when schema changes
- **After**: TypeScript compiler catches breaking changes

---

## Quick Start

### Before (Manual JSON)

```json
{
  "id": "aps-12345678",
  "intent": "Add authentication",
  "schema_version": "0.1.0",
  "proposed_changes": [
    {
      "type": "file_create",
      "path": "src/auth.ts",
      "description": "Create auth module"
    }
  ],
  "provenance": {
    "timestamp": "2024-01-01T00:00:00.000Z",
    "source": "cli",
    "version": "1.0.0"
  },
  "validations": {
    "required_checks": ["lint", "test"],
    "skip_checks": []
  },
  "hash": "..."
}
```

### After (APS Core)

```typescript
import { createPlan, generatePlanId, generateHash } from '@anvil/core';

const planData = createPlan({
  id: generatePlanId(),
  intent: 'Add authentication',
  provenance: {
    timestamp: new Date().toISOString(),
    source: 'cli',
    version: '1.0.0',
  },
  changes: [
    {
      type: 'file_create',
      path: 'src/auth.ts',
      description: 'Create auth module',
    },
  ],
});

const plan = {
  ...planData,
  hash: generateHash(planData),
};
```

**Benefits:**

- ✅ Auto-generated plan ID
- ✅ Auto-generated hash
- ✅ Type-safe changes array
- ✅ Default validations included
- ✅ Compile-time checking

---

## Migration Paths

Choose the migration path that fits your situation:

### Path 1: New Projects

Start fresh with APS Core from day one.

**Recommended for:**

- New projects
- No existing APS plans
- Green field development

**Action:** Follow the [Quick Start](#quick-start)

### Path 2: Existing Plans - Read Only

Keep existing JSON plans, use APS Core for new plans.

**Recommended for:**

- Large number of existing plans
- Plans are stable/archived
- Gradual migration preferred

**Action:** See [Reading Existing Plans](#reading-existing-plans)

### Path 3: Existing Plans - Full Migration

Convert all existing JSON plans to use APS Core.

**Recommended for:**

- Small to medium number of plans
- Active plans that need updates
- Want full benefits of APS Core

**Action:** See [Converting Existing Plans](#converting-existing-plans)

---

## Step-by-Step Migration

### Step 1: Install APS Core

```bash
pnpm add @anvil/core
```

### Step 2: Reading Existing Plans

If you have existing APS JSON files, you can validate and load them:

```typescript
import fs from 'node:fs/promises';
import { validateAPSPlan } from '@anvil/core';

async function loadExistingPlan(filePath: string) {
  // Read the JSON file
  const content = await fs.readFile(filePath, 'utf-8');
  const data = JSON.parse(content);

  // Validate it
  const result = await validateAPSPlan(data);

  if (!result.valid) {
    console.error('Invalid plan:', result.formattedErrors);
    throw new Error('Plan validation failed');
  }

  // Now you have a typed APSPlan object
  return result.data!;
}

// Usage
const plan = await loadExistingPlan('./plans/plan-001.json');
console.log('Loaded plan:', plan.id);
```

### Step 3: Converting Existing Plans

Create a migration script to convert existing plans:

```typescript
import fs from 'node:fs/promises';
import { glob } from 'glob';
import { validateAPSPlan, generateHash, type APSPlan } from '@anvil/core';

async function migratePlans(pattern: string) {
  const files = await glob(pattern);

  for (const file of files) {
    try {
      // Load existing plan
      const content = await fs.readFile(file, 'utf-8');
      const data = JSON.parse(content);

      // Validate structure
      const result = await validateAPSPlan(data);

      if (!result.valid) {
        console.error(`❌ ${file}: Invalid plan`);
        console.error(result.formattedErrors);
        continue;
      }

      // Verify hash
      const { hash: existingHash, ...planWithoutHash } = result.data!;
      const calculatedHash = generateHash(planWithoutHash);

      if (existingHash !== calculatedHash) {
        console.warn(`⚠️  ${file}: Hash mismatch, recalculating`);
        const updatedPlan: APSPlan = {
          ...planWithoutHash,
          hash: calculatedHash,
        };

        // Save updated plan
        await fs.writeFile(file, JSON.stringify(updatedPlan, null, 2), 'utf-8');
        console.log(`✅ ${file}: Updated hash`);
      } else {
        console.log(`✅ ${file}: Valid`);
      }
    } catch (error) {
      console.error(`❌ ${file}: Error -`, error);
    }
  }
}

// Run migration
await migratePlans('./plans/**/*.json');
```

### Step 4: Creating New Plans

Replace manual JSON creation with APS Core:

**Before:**

```javascript
const plan = {
  id: `aps-${generateRandomHex(8)}`,
  intent: "Add feature",
  schema_version: "0.1.0",
  proposed_changes: [...],
  provenance: {
    timestamp: new Date().toISOString(),
    source: "cli",
    version: "1.0.0"
  },
  validations: {
    required_checks: ["lint", "test", "coverage", "secrets"],
    skip_checks: []
  }
};

// Calculate hash manually
const { hash: _, ...dataForHash } = plan;
const hashInput = JSON.stringify(dataForHash, Object.keys(dataForHash).sort());
plan.hash = sha256(hashInput);
```

**After:**

```typescript
import { createPlan, generatePlanId, generateHash } from '@anvil/core';

const planData = createPlan({
  id: generatePlanId(),
  intent: 'Add feature',
  provenance: {
    timestamp: new Date().toISOString(),
    source: 'cli',
    version: '1.0.0'
  },
  changes: [...]
});

const plan = {
  ...planData,
  hash: generateHash(planData)
};
```

### Step 5: Update Validation Logic

**Before:**

```javascript
function validatePlanManually(plan) {
  if (!plan.id || !/^aps-[a-f0-9]{8}$/.test(plan.id)) {
    throw new Error('Invalid plan ID');
  }
  if (!plan.intent || plan.intent.length < 10) {
    throw new Error('Intent too short');
  }
  // ... many more manual checks
}
```

**After:**

```typescript
import { validateAPSPlan } from '@anvil/core';

async function validatePlan(plan: unknown) {
  const result = await validateAPSPlan(plan);

  if (!result.valid) {
    console.error(result.formattedErrors);
    throw new Error('Validation failed');
  }

  return result.data;
}
```

---

## Common Patterns

### Pattern 1: Hash Generation

**Before:**

```javascript
import crypto from 'crypto';

function calculateHash(plan) {
  const { hash, ...planWithoutHash } = plan;

  // Manual canonical serialization
  const sorted = {};
  Object.keys(planWithoutHash)
    .sort()
    .forEach((key) => {
      sorted[key] = planWithoutHash[key];
    });

  const content = JSON.stringify(sorted);
  return crypto.createHash('sha256').update(content).digest('hex');
}

plan.hash = calculateHash(plan);
```

**After:**

```typescript
import { generateHash } from '@anvil/core';

const { hash: _, ...planWithoutHash } = plan;
plan.hash = generateHash(planWithoutHash);
```

### Pattern 2: Plan ID Generation

**Before:**

```javascript
import crypto from 'crypto';

function generatePlanId() {
  const bytes = crypto.randomBytes(4);
  return `aps-${bytes.toString('hex')}`;
}

const id = generatePlanId();
```

**After:**

```typescript
import { generatePlanId } from '@anvil/core';

const id = generatePlanId();
```

### Pattern 3: Schema Validation

**Before:**

```javascript
function validatePlan(data) {
  const errors = [];

  if (!data.id?.match(/^aps-[a-f0-9]{8}$/)) {
    errors.push('Invalid ID format');
  }

  if (!data.intent || data.intent.length < 10 || data.intent.length > 500) {
    errors.push('Intent must be 10-500 characters');
  }

  if (data.schema_version !== '0.1.0') {
    errors.push('Invalid schema version');
  }

  // ... many more checks

  return { valid: errors.length === 0, errors };
}
```

**After:**

```typescript
import { validateAPSPlan } from '@anvil/core';

const result = await validateAPSPlan(data);
// Returns: { valid: boolean, data?: APSPlan, issues?: [], formattedErrors?: string }
```

### Pattern 4: Type Safety

**Before (JavaScript):**

```javascript
// No type checking, errors at runtime
const plan = {
  id: 'wrong-format', // Wrong format, no warning
  intent: 'Too short', // Too short, no warning
  proposed_changes: [
    {
      type: 'invalid_type', // Invalid type, no warning
      path: 123, // Wrong type, no warning
      description: 'Test',
    },
  ],
};
```

**After (TypeScript):**

```typescript
import { createPlan, type Change } from '@anvil/core';

const changes: Change[] = [
  {
    type: 'invalid_type', // ❌ Compile error: Invalid type
    path: 123, // ❌ Compile error: Must be string
    description: 'Test',
  },
];

const plan = createPlan({
  id: generatePlanId(), // ✅ Always correct format
  intent: 'Add feature', // ✅ Type-checked
  changes, // ✅ Type-checked
});
```

### Pattern 5: Default Values

**Before:**

```javascript
const plan = {
  // ... other fields ...
  validations: {
    required_checks: ['lint', 'test', 'coverage', 'secrets'], // Easy to forget
    skip_checks: [], // Easy to forget
  },
};
```

**After:**

```typescript
const planData = createPlan({
  // ... other fields ...
  // validations has sensible defaults automatically!
});
// planData.validations = { required_checks: ['lint', 'test', 'coverage', 'secrets'], skip_checks: [] }
```

---

## Validation Migration

### Before: Manual Validation

```javascript
// manual-validator.js
export function validateAPSPlan(data) {
  const errors = [];

  // Check ID
  if (!data.id || typeof data.id !== 'string') {
    errors.push({ path: 'id', message: 'ID is required' });
  } else if (!/^aps-[a-f0-9]{8}$/.test(data.id)) {
    errors.push({ path: 'id', message: 'Invalid ID format' });
  }

  // Check hash
  if (!data.hash || typeof data.hash !== 'string') {
    errors.push({ path: 'hash', message: 'Hash is required' });
  } else if (!/^[a-f0-9]{64}$/.test(data.hash)) {
    errors.push({ path: 'hash', message: 'Invalid hash format' });
  }

  // Check intent
  if (!data.intent || typeof data.intent !== 'string') {
    errors.push({ path: 'intent', message: 'Intent is required' });
  } else if (data.intent.length < 10) {
    errors.push({ path: 'intent', message: 'Intent too short (min 10 chars)' });
  } else if (data.intent.length > 500) {
    errors.push({ path: 'intent', message: 'Intent too long (max 500 chars)' });
  }

  // Check schema_version
  if (data.schema_version !== '0.1.0') {
    errors.push({ path: 'schema_version', message: 'Invalid schema version' });
  }

  // Check proposed_changes
  if (!Array.isArray(data.proposed_changes)) {
    errors.push({ path: 'proposed_changes', message: 'Must be an array' });
  } else {
    data.proposed_changes.forEach((change, i) => {
      const validTypes = [
        'file_create',
        'file_update',
        'file_delete',
        'config_update',
        'dependency_add',
        'dependency_remove',
        'dependency_update',
        'script_execute',
      ];

      if (!validTypes.includes(change.type)) {
        errors.push({
          path: `proposed_changes[${i}].type`,
          message: `Invalid change type: ${change.type}`,
        });
      }

      if (!change.path) {
        errors.push({
          path: `proposed_changes[${i}].path`,
          message: 'Path is required',
        });
      }

      if (!change.description) {
        errors.push({
          path: `proposed_changes[${i}].description`,
          message: 'Description is required',
        });
      }
    });
  }

  // ... many more checks for provenance, validations, etc.

  return {
    valid: errors.length === 0,
    errors,
  };
}
```

### After: APS Core Validation

```typescript
import { validateAPSPlan } from '@anvil/core';

// One line replaces hundreds of lines above
const result = await validateAPSPlan(data);

if (!result.valid) {
  console.error(result.formattedErrors);
}
```

**Benefits:**

- ✅ Comprehensive validation with ~5 lines vs 100+ lines
- ✅ Better error messages with paths and context
- ✅ Automatically updated when schema changes
- ✅ Type-safe throughout

---

## Troubleshooting

### Issue 1: Hash Mismatch After Migration

**Problem:**

```
Hash validation failed: expected abc123..., got def456...
```

**Cause:** Different canonicalization or hash algorithm

**Solution:**

```typescript
import { generateHash } from '@anvil/core';

// Recalculate hash using APS Core
const { hash: _, ...planWithoutHash } = plan;
plan.hash = generateHash(planWithoutHash);
```

### Issue 2: Schema Validation Errors

**Problem:**

```
Schema validation failed: id: Invalid format
```

**Cause:** Old plan doesn't match current schema

**Solution:**

```typescript
// Check specific issue
const result = await validateAPSPlan(plan);

if (!result.valid && result.issues) {
  result.issues.forEach((issue) => {
    console.log(`${issue.path}: ${issue.message}`);
  });
}

// Fix the issues
if (issue.path === 'id') {
  plan.id = generatePlanId();
}

// Recalculate hash after fixes
const { hash, ...fixed } = plan;
plan.hash = generateHash(fixed);
```

### Issue 3: TypeScript Errors

**Problem:**

```
Property 'proposed_changes' does not exist on type APSPlan
```

**Cause:** Using `proposed_changes` instead of the internal field name

**Solution:**

```typescript
// The type uses 'proposed_changes'
plan.proposed_changes.forEach((change) => {
  // ... process change
});
```

### Issue 4: Missing Required Fields

**Problem:**

```
Validation failed: provenance.timestamp is required
```

**Cause:** Manually creating plan object without all required fields

**Solution:**

```typescript
// Use createPlan() helper instead
const plan = createPlan({
  id: generatePlanId(),
  intent: 'My intent',
  provenance: {
    timestamp: new Date().toISOString(), // ✅ Don't forget this
    source: 'cli',
    version: '1.0.0',
  },
});
```

### Issue 5: Cannot Import Types

**Problem:**

```
Cannot find module '@anvil/core' or its corresponding type declarations
```

**Cause:** Package not installed or wrong import path

**Solution:**

```bash
# Ensure package is installed
pnpm add @anvil/core

# Check tsconfig.json has correct paths
{
  "compilerOptions": {
    "paths": {
      "@anvil/core": ["./core/src/index.ts"]
    }
  }
}
```

---

## Checklist

Use this checklist to track your migration progress:

### Planning

- [ ] Identify all locations where APS plans are created
- [ ] Identify all locations where APS plans are validated
- [ ] Choose migration path (new only, read existing, or full migration)
- [ ] Set up testing environment

### Installation

- [ ] Install `@anvil/core` package
- [ ] Update TypeScript configuration if needed
- [ ] Verify imports work

### Reading Existing Plans

- [ ] Create utility to load and validate existing JSON files
- [ ] Test with sample plans
- [ ] Verify hash validation works

### Creating New Plans

- [ ] Replace manual plan creation with `createPlan()`
- [ ] Replace manual ID generation with `generatePlanId()`
- [ ] Replace manual hash calculation with `generateHash()`
- [ ] Add TypeScript types where needed

### Validation

- [ ] Replace manual validation with `validateAPSPlan()`
- [ ] Update error handling for new validation results
- [ ] Test with invalid data

### Testing

- [ ] Write tests for plan creation
- [ ] Write tests for plan validation
- [ ] Write tests for hash generation
- [ ] Test migration of existing plans

### Documentation

- [ ] Update team documentation
- [ ] Add code examples
- [ ] Document any custom patterns

### Deployment

- [ ] Migrate existing plans if needed
- [ ] Deploy new code
- [ ] Monitor for issues

---

## Migration Timeline

**Week 1: Preparation**

- Set up APS Core
- Create utilities for reading existing plans
- Train team on new API

**Week 2-3: Gradual Migration**

- Start using APS Core for new plans
- Validate existing plans
- Fix any validation issues

**Week 4: Full Migration**

- Convert remaining manual code
- Run full test suite
- Deploy to production

---

## Getting Help

If you encounter issues during migration:

1. Check the [API Documentation](./API.md)
2. Review [Usage Examples](./EXAMPLES.md)
3. Look at test files in `core/src/**/*.test.ts`
4. Open an issue on GitHub

---

## Example: Complete Migration Script

Here's a complete script you can adapt for your migration:

```typescript
#!/usr/bin/env node
import fs from 'node:fs/promises';
import path from 'path';
import { glob } from 'glob';
import {
  validateAPSPlan,
  generateHash,
  isValidPlanId,
  isValidHash,
  type APSPlan,
} from '@anvil/core';

interface MigrationResult {
  file: string;
  status: 'valid' | 'fixed' | 'error';
  message: string;
}

async function migratePlans(directory: string): Promise<MigrationResult[]> {
  const pattern = path.join(directory, '**/*.json');
  const files = await glob(pattern);
  const results: MigrationResult[] = [];

  console.log(`Found ${files.length} JSON files\n`);

  for (const file of files) {
    try {
      // Read file
      const content = await fs.readFile(file, 'utf-8');
      const data = JSON.parse(content);

      // Check if it looks like an APS plan
      if (!data.schema_version || !data.intent) {
        results.push({
          file,
          status: 'error',
          message: 'Not an APS plan',
        });
        continue;
      }

      // Validate
      const validation = await validateAPSPlan(data);

      if (!validation.valid) {
        // Try to fix common issues
        let fixed = false;

        // Fix ID format
        if (!isValidPlanId(data.id)) {
          console.log(`⚠️  ${file}: Invalid ID, keeping as-is`);
        }

        // Recalculate hash
        const { hash: oldHash, ...planWithoutHash } = data;
        const newHash = generateHash(planWithoutHash);

        if (oldHash !== newHash) {
          data.hash = newHash;
          fixed = true;
        }

        // Validate again
        const revalidation = await validateAPSPlan(data);

        if (revalidation.valid) {
          // Save fixed plan
          await fs.writeFile(
            file,
            JSON.stringify(data, null, 2) + '\n',
            'utf-8'
          );

          results.push({
            file,
            status: 'fixed',
            message: fixed ? 'Hash recalculated' : 'Minor fixes applied',
          });
        } else {
          results.push({
            file,
            status: 'error',
            message: revalidation.summary || 'Validation failed',
          });
        }
      } else {
        results.push({
          file,
          status: 'valid',
          message: 'Already valid',
        });
      }
    } catch (error) {
      results.push({
        file,
        status: 'error',
        message: error instanceof Error ? error.message : 'Unknown error',
      });
    }
  }

  return results;
}

// Run migration
const directory = process.argv[2] || './plans';
const results = await migratePlans(directory);

// Print summary
console.log('\n' + '='.repeat(60));
console.log('Migration Summary');
console.log('='.repeat(60));

const valid = results.filter((r) => r.status === 'valid').length;
const fixed = results.filter((r) => r.status === 'fixed').length;
const errors = results.filter((r) => r.status === 'error').length;

console.log(`✅ Valid: ${valid}`);
console.log(`🔧 Fixed: ${fixed}`);
console.log(`❌ Errors: ${errors}`);

if (errors > 0) {
  console.log('\nErrors:');
  results
    .filter((r) => r.status === 'error')
    .forEach((r) => {
      console.log(`  ${r.file}: ${r.message}`);
    });
}

process.exit(errors > 0 ? 1 : 0);
```

Save as `migrate-plans.ts` and run:

```bash
npx tsx migrate-plans.ts ./plans
```

---

## Next Steps

After completing the migration:

1. Update your documentation to reference APS Core APIs
2. Train team members on the new patterns
3. Set up CI/CD to use APS Core validation
4. Consider contributing improvements back to APS Core

Happy migrating! 🚀
