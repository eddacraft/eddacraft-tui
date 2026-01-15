# APS Core Usage Examples

This document provides practical examples for common use cases with the APS Core
library.

## Table of Contents

- [Basic Plan Creation](#basic-plan-creation)
- [Plan Validation](#plan-validation)
- [Working with Changes](#working-with-changes)
- [Gate Execution](#gate-execution)
- [Plan Lifecycle](#plan-lifecycle)
- [Error Handling](#error-handling)
- [Advanced Usage](#advanced-usage)

---

## Basic Plan Creation

### Creating a Simple Plan

```typescript
import {
  createPlan,
  generatePlanId,
  generateHash,
  type APSPlan,
} from '@anvil/core';

// Create plan data without hash
const planData = createPlan({
  id: generatePlanId(),
  intent: 'Add logging utility to the project',
  provenance: {
    timestamp: new Date().toISOString(),
    source: 'cli',
    version: '1.0.0',
    author: 'developer@example.com',
  },
  changes: [
    {
      type: 'file_create',
      path: 'src/utils/logger.ts',
      description: 'Create logger utility',
      content: `export function log(message: string) {
  console.log(\`[\${new Date().toISOString()}] \${message}\`);
}`,
    },
  ],
});

// Add hash to complete the plan
const plan: APSPlan = {
  ...planData,
  hash: generateHash(planData),
};

console.log('Created plan:', plan.id);
```

### Creating a Plan with Multiple Changes

```typescript
import { createPlan, generatePlanId, generateHash } from '@anvil/core';

const planData = createPlan({
  id: generatePlanId(),
  intent: 'Implement user authentication system',
  provenance: {
    timestamp: new Date().toISOString(),
    source: 'cli',
    version: '1.0.0',
    repository: 'https://github.com/example/repo',
    branch: 'feature/auth',
    commit: 'abc123def456',
  },
  changes: [
    {
      type: 'file_create',
      path: 'src/auth/types.ts',
      description: 'Define authentication types',
      content: 'export interface User { id: string; email: string; }',
    },
    {
      type: 'file_create',
      path: 'src/auth/service.ts',
      description: 'Create authentication service',
    },
    {
      type: 'dependency_add',
      path: 'package.json',
      description: 'Add bcrypt for password hashing',
      metadata: {
        package: 'bcrypt',
        version: '^5.1.0',
      },
    },
    {
      type: 'config_update',
      path: '.env.example',
      description: 'Add JWT secret configuration',
    },
  ],
  validations: {
    required_checks: ['lint', 'test', 'coverage', 'secrets'],
    skip_checks: [],
    policy_version: '1.0.0',
  },
});

const plan = {
  ...planData,
  hash: generateHash(planData),
};
```

### Creating a Plan with Metadata and Tags

```typescript
const planData = createPlan({
  id: generatePlanId(),
  intent: 'Refactor API endpoints for better performance',
  provenance: {
    timestamp: new Date().toISOString(),
    source: 'automation',
    version: '2.1.0',
  },
  changes: [
    {
      type: 'file_update',
      path: 'src/api/users.ts',
      description: 'Optimize user query',
      diff: `@@ -10,5 +10,7 @@
-  const users = await db.query('SELECT * FROM users');
+  const users = await db.query(
+    'SELECT id, email FROM users WHERE active = true'
+  );`,
    },
  ],
});

const plan = {
  ...planData,
  hash: generateHash(planData),
  tags: ['refactoring', 'performance', 'api'],
  metadata: {
    estimatedImpact: 'medium',
    priority: 'high',
    jiraTicket: 'PROJ-123',
  },
};
```

---

## Plan Validation

### Basic Schema Validation

```typescript
import { validateAPSPlan } from '@anvil/core';

const result = await validateAPSPlan(planData);

if (result.valid) {
  console.log('✅ Plan is valid');
  console.log('Plan ID:', result.data?.id);
} else {
  console.error('❌ Validation failed');
  console.error(result.formattedErrors);
}
```

### Validation with Hash Check

```typescript
import { APSValidator, generateHash } from '@anvil/core';

const validator = new APSValidator();
validator.setHashValidator(generateHash);

const result = await validator.validate(plan, {
  validateHash: true,
  strict: true,
  format: 'cli',
});

if (!result.valid) {
  console.error('Validation issues found:');
  result.issues?.forEach((issue) => {
    console.error(`  ${issue.path}: ${issue.message}`);
  });
  process.exit(1);
}
```

### Quick Schema Check

```typescript
import { APSValidator } from '@anvil/core';

const validator = new APSValidator();

if (validator.isSchemaValid(data)) {
  console.log('Schema is valid');
  // Continue processing
} else {
  console.log('Schema is invalid');
  // Handle error
}
```

### Validating External Data

```typescript
import fs from 'node:fs/promises';
import { validateAPSPlan } from '@anvil/core';

async function validatePlanFile(filePath: string) {
  try {
    const content = await fs.readFile(filePath, 'utf-8');
    const data = JSON.parse(content);

    const result = await validateAPSPlan(data, {
      format: 'cli',
    });

    if (result.valid) {
      console.log(`✅ ${filePath} is a valid APS plan`);
      return result.data;
    } else {
      console.error(`❌ ${filePath} validation failed:`);
      console.error(result.formattedErrors);
      return null;
    }
  } catch (error) {
    console.error(`Failed to read/parse ${filePath}:`, error);
    return null;
  }
}

// Usage
const plan = await validatePlanFile('./plans/plan-001.json');
```

---

## Working with Changes

### File Operations

```typescript
import { type Change } from '@anvil/core';

// Create a new file
const createChange: Change = {
  type: 'file_create',
  path: 'src/components/Button.tsx',
  description: 'Add reusable button component',
  content: `import React from 'react';

export const Button: React.FC<{ label: string }> = ({ label }) => (
  <button>{label}</button>
);`,
};

// Update existing file
const updateChange: Change = {
  type: 'file_update',
  path: 'src/App.tsx',
  description: 'Import and use Button component',
  diff: `@@ -1,3 +1,4 @@
 import React from 'react';
+import { Button } from './components/Button';

 export default function App() {
-  return <div>Hello</div>;
+  return <Button label="Click me" />;
 }`,
};

// Delete a file
const deleteChange: Change = {
  type: 'file_delete',
  path: 'src/legacy/OldButton.tsx',
  description: 'Remove deprecated button component',
};
```

### Dependency Management

```typescript
import { type Change } from '@anvil/core';

// Add dependency
const addDep: Change = {
  type: 'dependency_add',
  path: 'package.json',
  description: 'Add React Query for data fetching',
  metadata: {
    package: '@tanstack/react-query',
    version: '^5.0.0',
    devDependency: false,
  },
};

// Update dependency
const updateDep: Change = {
  type: 'dependency_update',
  path: 'package.json',
  description: 'Update TypeScript to latest version',
  metadata: {
    package: 'typescript',
    from: '5.2.0',
    to: '5.3.0',
  },
};

// Remove dependency
const removeDep: Change = {
  type: 'dependency_remove',
  path: 'package.json',
  description: 'Remove unused lodash dependency',
  metadata: {
    package: 'lodash',
  },
};
```

### Configuration Updates

```typescript
const configChange: Change = {
  type: 'config_update',
  path: 'tsconfig.json',
  description: 'Enable strict mode',
  diff: `@@ -2,5 +2,6 @@
   "compilerOptions": {
     "target": "ES2022",
+    "strict": true,
     "module": "ESNext"
   }
 }`,
};
```

### Script Execution

```typescript
const scriptChange: Change = {
  type: 'script_execute',
  path: 'scripts/migrate-db.sh',
  description: 'Run database migration',
  metadata: {
    command: 'npm run migrate:up',
    environment: 'development',
    requiresConfirmation: true,
  },
};
```

---

## Gate Execution

### Running Basic Gate Checks

```typescript
import { GateRunner, type APSPlan } from '@anvil/core';

const plan: APSPlan = /* ... */;

const runner = new GateRunner({
  workingDir: process.cwd(),
  checksToRun: ['lint', 'test', 'secrets']
});

const result = await runner.execute(plan);

console.log(`Gate status: ${result.overall_status}`);
console.log(`Duration: ${result.duration}ms`);

result.checks.forEach(check => {
  const icon = check.status === 'passed' ? '✅' : '❌';
  console.log(`${icon} ${check.name}: ${check.message}`);
});

if (!result.passed) {
  console.error('Gate failed. Cannot proceed with plan execution.');
  process.exit(1);
}
```

### Running Gates with Custom Configuration

```typescript
import { GateRunner } from '@anvil/core';

const runner = new GateRunner({
  workingDir: process.cwd(),
  checksToRun: ['lint', 'test', 'coverage', 'secrets'],
  stopOnFailure: false,
  parallel: true,
  timeout: 60000, // 60 seconds
});

const result = await runner.execute(plan);

// Access individual check results
const testCheck = result.checks.find((c) => c.name === 'test');
if (testCheck?.status === 'failed') {
  console.error('Test failures:', testCheck.details);
}
```

### Adding Evidence to Plan

```typescript
import { type APSPlan, type Evidence } from '@anvil/core';

async function executeGateAndAddEvidence(plan: APSPlan): Promise<APSPlan> {
  const runner = new GateRunner({
    workingDir: process.cwd(),
  });

  const gateResult = await runner.execute(plan);

  // Convert gate result to evidence
  const evidence: Evidence = {
    gate_version: '1.0.0',
    timestamp: new Date().toISOString(),
    overall_status: gateResult.overall_status,
    checks: gateResult.checks.map((check) => ({
      check: check.name,
      status: check.status,
      timestamp: new Date().toISOString(),
      message: check.message,
      details: check.details,
    })),
    summary: gateResult.summary,
  };

  // Add evidence to plan (append-only)
  const updatedPlan: APSPlan = {
    ...plan,
    evidence: [...(plan.evidence || []), evidence],
  };

  return updatedPlan;
}
```

---

## Plan Lifecycle

### Complete Plan Workflow

```typescript
import {
  createPlan,
  generatePlanId,
  generateHash,
  validateAPSPlan,
  GateRunner,
  type APSPlan,
  type Approval,
  type ExecutionResult,
} from '@anvil/core';

// 1. Create plan
const planData = createPlan({
  id: generatePlanId(),
  intent: 'Add new feature X',
  provenance: {
    timestamp: new Date().toISOString(),
    source: 'cli',
    version: '1.0.0',
    author: 'developer@example.com',
  },
  changes: [
    /* ... */
  ],
});

let plan: APSPlan = {
  ...planData,
  hash: generateHash(planData),
};

// 2. Validate plan
const validationResult = await validateAPSPlan(plan);
if (!validationResult.valid) {
  throw new Error('Plan validation failed');
}

// 3. Run gates
const runner = new GateRunner({
  workingDir: process.cwd(),
});

const gateResult = await runner.execute(plan);

// 4. Add evidence
plan = {
  ...plan,
  evidence: [
    {
      gate_version: '1.0.0',
      timestamp: new Date().toISOString(),
      overall_status: gateResult.overall_status,
      checks: gateResult.checks.map((c) => ({
        check: c.name,
        status: c.status,
        timestamp: new Date().toISOString(),
        message: c.message,
      })),
      summary: gateResult.summary,
    },
  ],
};

// 5. Approve plan (manual or automated)
const approval: Approval = {
  approved: true,
  approved_by: 'manager@example.com',
  approved_at: new Date().toISOString(),
  approval_notes: 'LGTM',
};

plan = { ...plan, approval };

// 6. Execute plan (implementation specific)
// This would be your actual execution logic
const executionResult: ExecutionResult = {
  operation: 'apply',
  status: 'success',
  timestamp: new Date().toISOString(),
  executed_by: 'automation@example.com',
  changes_applied: plan.proposed_changes.map((c) => c.path),
  changes_failed: [],
};

plan = {
  ...plan,
  executions: [executionResult],
};

console.log('Plan lifecycle complete:', plan.id);
```

### Reading and Updating Plans

```typescript
import fs from 'node:fs/promises';
import { validateAPSPlan, generateHash, type APSPlan } from '@anvil/core';

// Load plan from file
async function loadPlan(filePath: string): Promise<APSPlan> {
  const content = await fs.readFile(filePath, 'utf-8');
  const data = JSON.parse(content);

  const result = await validateAPSPlan(data);
  if (!result.valid || !result.data) {
    throw new Error('Invalid plan file');
  }

  return result.data;
}

// Save plan to file
async function savePlan(plan: APSPlan, filePath: string): Promise<void> {
  const content = JSON.stringify(plan, null, 2);
  await fs.writeFile(filePath, content, 'utf-8');
}

// Usage
const plan = await loadPlan('./plans/plan-001.json');

// Update plan (e.g., add approval)
const updatedPlan: APSPlan = {
  ...plan,
  approval: {
    approved: true,
    approved_by: 'reviewer@example.com',
    approved_at: new Date().toISOString(),
  },
};

await savePlan(updatedPlan, './plans/plan-001.json');
```

---

## Error Handling

### Handling Validation Errors

```typescript
import {
  validateAPSPlan,
  SchemaValidationError,
  HashValidationError,
} from '@anvil/core';

try {
  const result = await validateAPSPlan(data);

  if (!result.valid) {
    // Handle validation failure
    console.error('Validation failed with issues:');
    result.issues?.forEach((issue) => {
      console.error(`  [${issue.severity}] ${issue.path}: ${issue.message}`);
    });

    // You can also show formatted errors
    if (result.formattedErrors) {
      console.error('\nFormatted errors:\n', result.formattedErrors);
    }
  }
} catch (error) {
  if (error instanceof SchemaValidationError) {
    console.error('Schema validation error:', error.message);
    console.error('Issues:', error.issues);
  } else if (error instanceof HashValidationError) {
    console.error('Hash validation error:', error.message);
    console.error('Expected hash:', error.expectedHash);
  } else {
    console.error('Unexpected error:', error);
  }
}
```

### Handling Gate Errors

```typescript
import { GateRunner } from '@anvil/core';

async function runGatesWithErrorHandling(plan: APSPlan) {
  const runner = new GateRunner({
    workingDir: process.cwd(),
    checksToRun: ['lint', 'test', 'coverage'],
  });

  try {
    const result = await runner.execute(plan);

    if (!result.passed) {
      // Collect all failures
      const failures = result.checks.filter((c) => c.status === 'failed');

      console.error(`Gate failed: ${failures.length} check(s) failed`);

      failures.forEach((check) => {
        console.error(`\n❌ ${check.name}`);
        console.error(`   ${check.message}`);

        if (check.error) {
          console.error(`   Error: ${check.error.message}`);
        }

        if (check.details) {
          console.error(`   Details:`, JSON.stringify(check.details, null, 2));
        }
      });

      return false;
    }

    return true;
  } catch (error) {
    console.error('Gate execution failed:', error);
    return false;
  }
}
```

---

## Advanced Usage

### Custom Hash Validation

```typescript
import { APSValidator, generateHash, verifyHash } from '@anvil/core';

// Create validator with custom hash function
const validator = new APSValidator();
validator.setHashValidator(generateHash);

// Validate with hash check
const result = await validator.validate(plan, {
  validateHash: true,
});

// Or manually verify hash
if (!verifyHash(planDataWithoutHash, plan.hash)) {
  console.error('Hash mismatch detected!');
}
```

### Working with Plan IDs

```typescript
import { generatePlanId, isValidPlanId, isValidHash } from '@anvil/core';

// Generate new ID
const id = generatePlanId();
console.log('New plan ID:', id); // e.g., 'aps-a1b2c3d4'

// Validate ID format
if (isValidPlanId(id)) {
  console.log('Valid plan ID');
}

// Validate hash format
const hash = generateHash(data);
if (isValidHash(hash)) {
  console.log('Valid hash format');
}

// Extract IDs from string
const text = 'Review plan aps-12345678 and aps-abcdef00';
const planIds = text.match(/aps-[a-f0-9]{8}/g);
console.log('Found plan IDs:', planIds);
```

### Programmatic Schema Generation

```typescript
import { generateJSONSchema, getJSONSchemaString } from '@anvil/core';

// Get JSON Schema object
const schema = generateJSONSchema();

// Use with validation library
import Ajv from 'ajv';
const ajv = new Ajv();
const validate = ajv.compile(schema);

const isValid = validate(planData);
if (!isValid) {
  console.error('Validation errors:', validate.errors);
}

// Export schema for documentation
const schemaString = getJSONSchemaString(true);
await fs.writeFile('aps-schema.json', schemaString);
```

### Batch Plan Validation

```typescript
import { validateAPSPlan } from '@anvil/core';
import { glob } from 'glob';

async function validateAllPlans(pattern: string) {
  const files = await glob(pattern);
  const results = await Promise.all(
    files.map(async (file) => {
      const content = JSON.parse(await fs.readFile(file, 'utf-8'));
      const result = await validateAPSPlan(content);
      return { file, result };
    })
  );

  const invalid = results.filter((r) => !r.result.valid);

  if (invalid.length > 0) {
    console.error(`Found ${invalid.length} invalid plans:`);
    invalid.forEach(({ file, result }) => {
      console.error(`\n${file}:`);
      console.error(result.formattedErrors);
    });
    return false;
  }

  console.log(`✅ All ${results.length} plans are valid`);
  return true;
}

// Usage
await validateAllPlans('./plans/**/*.json');
```

### Creating Plan Templates

```typescript
import { createPlan, generatePlanId, generateHash } from '@anvil/core';

function createFeaturePlanTemplate(featureName: string, author: string) {
  const planData = createPlan({
    id: generatePlanId(),
    intent: `Add ${featureName} feature`,
    provenance: {
      timestamp: new Date().toISOString(),
      source: 'cli',
      version: '1.0.0',
      author,
    },
    changes: [
      {
        type: 'file_create',
        path: `src/features/${featureName}/index.ts`,
        description: `Create ${featureName} module`,
      },
      {
        type: 'file_create',
        path: `src/features/${featureName}/__tests__/${featureName}.test.ts`,
        description: `Create ${featureName} tests`,
      },
    ],
  });

  return {
    ...planData,
    hash: generateHash(planData),
  };
}

// Usage
const plan = createFeaturePlanTemplate('user-profile', 'dev@example.com');
```

### Plan Diffing

```typescript
import { type APSPlan } from '@anvil/core';

function comparePlans(plan1: APSPlan, plan2: APSPlan) {
  const differences = {
    intentChanged: plan1.intent !== plan2.intent,
    changesAdded: plan2.proposed_changes.length - plan1.proposed_changes.length,
    evidenceAdded:
      (plan2.evidence?.length || 0) - (plan1.evidence?.length || 0),
    approvalChanged: plan1.approval?.approved !== plan2.approval?.approved,
    executionsAdded:
      (plan2.executions?.length || 0) - (plan1.executions?.length || 0),
  };

  return differences;
}

// Usage
const original = await loadPlan('./plan-v1.json');
const updated = await loadPlan('./plan-v2.json');
const diff = comparePlans(original, updated);

console.log('Plan differences:', diff);
```

---

## Next Steps

- Review the [API Documentation](./API.md) for detailed API reference
- Check the [Migration Guide](./MIGRATION.md) for migrating from manual JSON
- Explore the [Schema Specification](./src/schema/aps.schema.ts) for the
  complete schema definition
