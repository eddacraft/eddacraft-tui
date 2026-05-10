---
id: json-schema
title: Schemas and Types
description: APS type definitions and validation schemas.
sidebar_position: 1
---

# Schemas and Types

APS uses [Zod](https://zod.dev) schemas for runtime validation and TypeScript
type generation.

## TypeScript Types

The core types are exported from `@eddacraft/anvil-aps`:

```typescript
import type {
  Task,
  ModuleMetadata,
  ParsedDocument,
} from '@eddacraft/anvil-aps';
```

### Task

A unit of authorised work:

```typescript
interface Task {
  id: string; // Pattern: SCOPE-NNN (e.g., AUTH-001)
  title: string;
  intent: string; // What the task aims to achieve (required)
  expectedOutcome?: string; // Success criteria
  validation?: string; // Command to verify completion
  confidence?: 'low' | 'medium' | 'high'; // Defaults to 'medium'
  scopes?: string[]; // File access constraints
  nonScope?: string[]; // What will NOT be changed
  files?: string[]; // Files that may be affected
  tags?: string[]; // Labels for filtering
  dependencies?: string[]; // Tasks that must complete first
  inputs?: string[]; // Required inputs or context
  risks?: string[]; // Potential risks
  link?: string; // External link (e.g., Jira ticket)
  status?: 'open' | 'locked' | 'completed' | 'cancelled';
  sourcePath?: string; // Source file path
  sourceLineNumber?: number; // Line number in source
}
```

### Task ID Format

Task IDs follow the pattern `{SCOPE}-{NNN}`:

- 1-10 uppercase alphanumeric characters for the scope
- Hyphen separator
- 3-digit zero-padded number

```
AUTH-001   ✓
PAY-042   ✓
CORE-001  ✓
LLM2-007  ✓
auth-001  ✗  (must be uppercase)
AUTH-1    ✗  (must be 3 digits)
```

### Module Metadata

```typescript
interface ModuleMetadata {
  id?: string; // Module identifier
  title?: string; // From H1 heading
  path?: string; // Path to spec file
  scope?: string; // Scope prefix for task IDs
  owner?: string; // Person/team responsible
  status?: 'Proposed' | 'Ready' | 'In Progress' | 'Done' | 'Blocked';
  priority?: 'low' | 'medium' | 'high';
  tags?: string[];
  packages?: string[]; // Package names in monorepo modules
  dependencies?: string[]; // Other modules this depends on
}
```

## Operating Model Extension

The current exported schemas above are the package-level validation contract.
The Anvil repository is migrating to a richer operating-model lifecycle for
active APS planning:

```text
APS Draft -> APS Proposed -> APS Ready -> In Progress -> Merged -> Released/Shipped -> Complete/Archived
```

Target-state APS work items may also carry release reconstruction metadata:

```typescript
interface OperatingModelReleaseMetadata {
  changeType?: 'fix' | 'feature' | 'docs' | 'internal' | 'breaking';
  releaseIntent?: 'candidate' | 'hold' | 'never';
  holdCondition?: string; // Required when releaseIntent is 'hold'
  releaseScope?: 'patch' | 'minor' | 'major' | 'none';
  releaseNote?: {
    audience: 'user' | 'operator' | 'developer' | 'none';
    type: 'added' | 'fixed' | 'changed' | 'removed' | 'security';
    text?: string;
  };
}
```

Until the package schemas are updated, this lifecycle is repository guidance,
not the exported `@eddacraft/anvil-aps` runtime schema.

### Parsed Document

Result of parsing a leaf spec file:

```typescript
interface ParsedDocument {
  title: string; // From H1 heading
  metadata?: ModuleMetadata;
  tasks: Task[];
  sourcePath?: string;
}
```

## Programmatic Validation

Use the Zod schemas directly for runtime validation:

```typescript
import { TaskSchema, ModuleMetadataSchema } from '@eddacraft/anvil-aps';

const result = TaskSchema.safeParse(data);

if (!result.success) {
  console.error(result.error.issues);
}
```

## Validation Errors

Common validation errors:

### Missing Required Field

```
ZodError: Required at "intent"
```

The `intent` field is required for all tasks.

### Invalid Task ID

```
ZodError: Invalid at "id" — must match pattern ^[A-Z0-9]{1,10}-\d{3}$
```

### Invalid Status

```
ZodError: Invalid enum value at "status"
  Expected: 'open' | 'locked' | 'completed' | 'cancelled'
  Received: 'done'
```

---

**Next:** [Examples →](/aps/schemas/examples)
