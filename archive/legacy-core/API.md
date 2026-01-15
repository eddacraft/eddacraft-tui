# APS Core API Documentation

## Overview

The APS Core (`@anvil/core`) package provides the foundational functionality for
working with Anvil Plan Specification (APS) documents. It includes schema
definitions, validation, cryptographic utilities, and gate execution
capabilities.

## Installation

```bash
pnpm add @anvil/core
```

## Table of Contents

- [Schema](#schema)
- [Validation](#validation)
- [Crypto](#crypto)
- [Gate](#gate)
- [Types](#types)

---

## Schema

The schema module provides Zod schemas and TypeScript types for APS plans.

### `APSPlanSchema`

Zod schema defining the complete structure of an APS plan.

```typescript
import { APSPlanSchema, type APSPlan } from '@anvil/core';
```

### Core Types

#### `APSPlan`

The main plan type containing all plan data.

```typescript
interface APSPlan {
  // Identification
  id: string; // Format: 'aps-[8 hex chars]'
  hash: string; // SHA-256 hash (64 hex chars)

  // Core content
  intent: string; // 10-500 characters
  schema_version: '0.1.0'; // Current schema version
  proposed_changes: Change[]; // List of changes

  // Metadata
  provenance: Provenance; // Creation information
  validations: Validation; // Validation requirements

  // Optional lifecycle fields
  evidence?: Evidence[]; // Gate execution results (immutable)
  approval?: Approval; // Approval information
  executions?: ExecutionResult[]; // Execution history

  // Additional metadata
  tags?: string[]; // Tags for categorization
  metadata?: Record<string, unknown>; // Custom metadata
}
```

#### `Change`

Represents a single proposed change in the plan.

```typescript
type ChangeType =
  | 'file_create'
  | 'file_update'
  | 'file_delete'
  | 'config_update'
  | 'dependency_add'
  | 'dependency_remove'
  | 'dependency_update'
  | 'script_execute';

interface Change {
  type: ChangeType;
  path: string; // File or resource path
  description: string; // Human-readable description
  content?: string; // New content for file changes
  diff?: string; // Unified diff for updates
  metadata?: Record<string, unknown>; // Additional metadata
}
```

#### `Provenance`

Tracks the origin and creation context of a plan.

```typescript
interface Provenance {
  timestamp: string; // ISO 8601 datetime
  author?: string; // Creator user/system
  source: 'cli' | 'api' | 'automation' | 'manual';
  version: string; // Tool version
  repository?: string; // Repository URL
  branch?: string; // Git branch
  commit?: string; // Git commit hash
}
```

#### `Validation`

Defines validation requirements for the plan.

```typescript
interface Validation {
  required_checks: string[]; // Default: ['lint', 'test', 'coverage', 'secrets']
  policy_version?: string; // Policy bundle version
  skip_checks: string[]; // Checks to skip (default: [])
  custom_rules?: Record<string, unknown>; // Custom rules
}
```

#### `Evidence`

Contains gate execution results (immutable, append-only).

```typescript
interface Evidence {
  gate_version: string; // Gate version that ran
  timestamp: string; // ISO 8601 datetime
  overall_status: 'passed' | 'failed' | 'partial';
  checks: EvidenceEntry[]; // Individual check results
  summary?: string; // Execution summary
  artifacts?: Record<string, string>; // Artifact references
}

interface EvidenceEntry {
  check: string; // Check name
  status: 'passed' | 'failed' | 'skipped' | 'warning';
  timestamp: string; // ISO 8601 datetime
  details?: Record<string, unknown>; // Detailed results
  message?: string; // Human-readable message
}
```

#### `Approval`

Tracks plan approval status.

```typescript
interface Approval {
  approved: boolean;
  approved_by?: string; // Approver user
  approved_at?: string; // ISO 8601 datetime
  approval_notes?: string; // Comments
}
```

#### `ExecutionResult`

Records plan execution results.

```typescript
interface ExecutionResult {
  operation: 'apply' | 'rollback' | 'dry-run';
  status: 'success' | 'failed' | 'partial';
  timestamp: string; // ISO 8601 datetime
  executed_by?: string; // Executor user
  changes_applied?: string[]; // Successful changes
  changes_failed?: string[]; // Failed changes
  rollback_point?: string; // Snapshot ID for rollback
  logs?: string[]; // Execution logs
}
```

### Functions

#### `validatePlan(data: unknown): SchemaValidationResult`

Validates data against the APS schema.

```typescript
const result = validatePlan(planData);
if (result.success) {
  console.log('Valid plan:', result.data);
} else {
  console.error('Validation errors:', result.errors);
}
```

**Returns:**

```typescript
interface SchemaValidationResult {
  success: boolean;
  data?: APSPlan;
  errors?: Array<{
    path: string;
    message: string;
    code?: string;
  }>;
}
```

#### `createPlan(params): Omit<APSPlan, 'hash'>`

Creates a new plan with defaults (hash must be added separately).

```typescript
const plan = createPlan({
  id: 'aps-12345678',
  intent: 'Add user authentication feature',
  provenance: {
    timestamp: new Date().toISOString(),
    source: 'cli',
    version: '1.0.0',
  },
  changes: [
    {
      type: 'file_create',
      path: 'src/auth.ts',
      description: 'Create authentication module',
    },
  ],
});
```

#### `generateJSONSchema(): JSONSchema7`

Generates JSON Schema representation of the APS schema for external tooling.

```typescript
import { generateJSONSchema } from '@anvil/core';

const jsonSchema = generateJSONSchema();
// Use with JSON Schema validators
```

#### `getJSONSchemaString(pretty?: boolean): string`

Returns JSON Schema as a formatted string.

```typescript
const schemaString = getJSONSchemaString(true);
console.log(schemaString);
```

### Constants

#### `APS_SCHEMA_VERSION`

Current version of the APS schema: `'0.1.0'`

```typescript
import { APS_SCHEMA_VERSION } from '@anvil/core';
```

---

## Validation

The validation module provides comprehensive validation for APS plans.

### `APSValidator`

Main validation class with schema and hash verification.

```typescript
import { APSValidator } from '@anvil/core';

const validator = new APSValidator();
```

#### Methods

##### `validate(plan: unknown, options?: ValidationOptions): Promise<ValidationResult>`

Performs complete validation including schema and optional hash verification.

```typescript
const result = await validator.validate(planData, {
  validateHash: true,
  strict: true,
  format: 'cli',
});

if (result.valid) {
  console.log('✅', result.summary);
} else {
  console.error('❌', result.formattedErrors);
}
```

**Options:**

```typescript
interface ValidationOptions {
  validateHash?: boolean; // Enable hash verification (default: false)
  strict?: boolean; // Stop on first error (default: true)
  format?: 'cli' | 'json'; // Error format (default: 'cli')
}
```

**Returns:**

```typescript
interface ValidationResult {
  valid: boolean;
  data?: APSPlan;
  issues?: ValidationIssue[];
  summary: string;
  formattedErrors?: string;
}

interface ValidationIssue {
  path: string;
  message: string;
  code: string;
  severity: 'error' | 'warning';
  details?: unknown;
}
```

##### `validateSchema(plan: unknown): Promise<ValidationResult>`

Validates only the schema (no hash check).

```typescript
const result = await validator.validateSchema(planData);
```

##### `validateHash(plan: APSPlan): Promise<ValidationResult>`

Validates hash integrity of a plan.

```typescript
const result = await validator.validateHash(validPlan);
```

##### `isSchemaValid(plan: unknown): boolean`

Quick synchronous check if schema is valid.

```typescript
if (validator.isSchemaValid(data)) {
  // Schema is valid
}
```

##### `setHashValidator(validator: (data: unknown) => string): void`

Injects hash validation function (typically uses `generateHash` from crypto
module).

```typescript
import { generateHash } from '@anvil/core';

validator.setHashValidator(generateHash);
```

### Convenience Functions

#### `validateAPSPlan(plan: unknown, options?: ValidationOptions): Promise<ValidationResult>`

Singleton validator for quick validation.

```typescript
import { validateAPSPlan } from '@anvil/core';

const result = await validateAPSPlan(planData);
```

### Singleton Instance

#### `validator`

Pre-configured singleton instance.

```typescript
import { validator } from '@anvil/core';

await validator.validate(plan);
```

---

## Crypto

The crypto module provides hashing and ID generation utilities.

### `generateHash(data: unknown): string`

Generates deterministic SHA-256 hash of data.

```typescript
import { generateHash } from '@anvil/core';

const planWithoutHash = {
  /* plan data */
};
const hash = generateHash(planWithoutHash);
// Returns: '64-character hex string'
```

Features:

- Canonical JSON serialization (sorted keys)
- Deterministic output
- Handles nested objects and arrays

### `canonicalizeJSON(obj: unknown): string`

Converts object to canonical JSON string with sorted keys.

```typescript
import { canonicalizeJSON } from '@anvil/core';

const canonical = canonicalizeJSON({ b: 2, a: 1 });
// Returns: '{"a":1,"b":2}'
```

### `verifyHash(data: unknown, expectedHash: string): boolean`

Verifies hash matches the data.

```typescript
import { verifyHash } from '@anvil/core';

const isValid = verifyHash(planData, plan.hash);
```

### `generatePlanId(): string`

Generates unique plan ID in format `aps-[8 hex chars]`.

```typescript
import { generatePlanId } from '@anvil/core';

const id = generatePlanId();
// Returns: 'aps-a1b2c3d4'
```

### `isValidPlanId(id: string): boolean`

Validates plan ID format.

```typescript
import { isValidPlanId } from '@anvil/core';

if (isValidPlanId('aps-12345678')) {
  // Valid format
}
```

### `isValidHash(hash: string): boolean`

Validates SHA-256 hash format (64 hex characters).

```typescript
import { isValidHash } from '@anvil/core';

if (isValidHash(someHash)) {
  // Valid hash format
}
```

---

## Gate

The gate module provides quality gate execution and checking capabilities.

### `GateRunner`

Executes quality gate checks on plans.

```typescript
import { GateRunner } from '@anvil/core';

const runner = new GateRunner({
  workingDir: '/path/to/project',
  checksToRun: ['lint', 'test', 'coverage', 'secrets'],
});
```

#### Constructor Options

```typescript
interface GateConfig {
  workingDir: string; // Project directory
  checksToRun?: string[]; // Checks to execute
  stopOnFailure?: boolean; // Stop on first failure (default: false)
  timeout?: number; // Check timeout in ms
  parallel?: boolean; // Run checks in parallel (default: true)
  reporters?: GateReporter[]; // Custom reporters
}
```

#### Methods

##### `execute(plan: APSPlan): Promise<GateResult>`

Executes all configured checks against a plan.

```typescript
const result = await runner.execute(plan);

if (result.passed) {
  console.log('✅ All checks passed');
} else {
  console.log('❌ Gate failed:', result.failures);
}
```

**Returns:**

```typescript
interface GateResult {
  passed: boolean;
  overall_status: 'passed' | 'failed' | 'partial';
  checks: CheckResult[];
  timestamp: string;
  duration: number;
  summary: string;
}

interface CheckResult {
  name: string;
  status: 'passed' | 'failed' | 'skipped' | 'warning';
  message?: string;
  duration: number;
  details?: Record<string, unknown>;
  error?: Error;
}
```

### Built-in Checks

The following checks are available by default:

#### `lint`

Runs ESLint on the codebase.

#### `test`

Executes test suite (supports Vitest, Jest).

#### `coverage`

Verifies test coverage thresholds.

#### `secrets`

Scans for exposed secrets and credentials.

### Custom Checks

Create custom checks by implementing the `GateCheck` interface:

```typescript
import { GateCheck, CheckResult } from '@anvil/core';

class CustomCheck implements GateCheck {
  name = 'custom-check';

  async execute(plan: APSPlan): Promise<CheckResult> {
    // Implement check logic
    return {
      name: this.name,
      status: 'passed',
      duration: 100,
      message: 'Check passed',
    };
  }
}
```

### Gate Configuration

Configure gates using `gate.config.ts`:

```typescript
import { defineGateConfig } from '@anvil/core';

export default defineGateConfig({
  workingDir: process.cwd(),
  checksToRun: ['lint', 'test', 'coverage'],
  stopOnFailure: false,
  parallel: true,
});
```

---

## Types

### Common Type Exports

All types are exported from the main package entry:

```typescript
import type {
  // Core types
  APSPlan,
  Change,
  ChangeType,
  Provenance,
  Validation,
  Evidence,
  EvidenceEntry,
  Approval,
  ExecutionResult,

  // Validation types
  ValidationResult,
  ValidationOptions,
  ValidationIssue,
  SchemaValidationResult,

  // Gate types
  GateConfig,
  GateResult,
  CheckResult,
  GateCheck,

  // Error types
  SchemaValidationError,
  HashValidationError,
} from '@anvil/core';
```

---

## Error Handling

### Custom Errors

#### `SchemaValidationError`

Thrown when schema validation fails.

```typescript
try {
  await validator.validateSchema(data);
} catch (error) {
  if (error instanceof SchemaValidationError) {
    console.error('Schema errors:', error.issues);
  }
}
```

#### `HashValidationError`

Thrown when hash validation fails.

```typescript
try {
  await validator.validateHash(plan);
} catch (error) {
  if (error instanceof HashValidationError) {
    console.error('Hash mismatch:', error.expectedHash);
  }
}
```

### Error Formatting

#### `formatValidationErrors(issues: ValidationIssue[]): string`

Formats validation issues for CLI output.

```typescript
import { formatValidationErrors } from '@anvil/core';

const formatted = formatValidationErrors(result.issues);
console.error(formatted);
```

#### `createValidationSummary(valid: boolean, issues: ValidationIssue[]): string`

Creates a summary message for validation results.

```typescript
import { createValidationSummary } from '@anvil/core';

const summary = createValidationSummary(false, issues);
// Returns: '❌ Validation failed with 3 errors'
```

---

## Complete Example

```typescript
import {
  createPlan,
  generatePlanId,
  generateHash,
  APSValidator,
  GateRunner,
  type APSPlan,
} from '@anvil/core';

// 1. Create a new plan
const planWithoutHash = createPlan({
  id: generatePlanId(),
  intent: 'Add user authentication',
  provenance: {
    timestamp: new Date().toISOString(),
    source: 'cli',
    version: '1.0.0',
    author: 'developer@example.com',
  },
  changes: [
    {
      type: 'file_create',
      path: 'src/auth.ts',
      description: 'Create auth module',
      content: 'export function authenticate() { }',
    },
  ],
});

// 2. Add hash
const plan: APSPlan = {
  ...planWithoutHash,
  hash: generateHash(planWithoutHash),
};

// 3. Validate the plan
const validator = new APSValidator();
validator.setHashValidator(generateHash);

const validationResult = await validator.validate(plan, {
  validateHash: true,
  format: 'cli',
});

if (!validationResult.valid) {
  console.error(validationResult.formattedErrors);
  process.exit(1);
}

// 4. Run quality gates
const runner = new GateRunner({
  workingDir: process.cwd(),
  checksToRun: ['lint', 'test', 'secrets'],
});

const gateResult = await runner.execute(plan);

if (!gateResult.passed) {
  console.error('Gate checks failed:', gateResult.summary);
  process.exit(1);
}

console.log('✅ Plan validated and gates passed');
```

---

## Version Information

- **Package Version**: 0.0.1
- **Schema Version**: 0.1.0
- **Node.js**: >= 18.0.0

## See Also

- [Usage Examples](./EXAMPLES.md)
- [Migration Guide](./MIGRATION.md)
- [Schema Specification](./src/schema/aps.schema.ts)
