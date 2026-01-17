---
id: json-schema
title: JSON Schema
description: Machine-readable schema for APS validation.
sidebar_position: 1
---

# JSON Schema

APS provides a JSON Schema for machine validation of plans.

## Schema Location

```
https://eddacraft.dev/schemas/aps/v1.0/aps.schema.json
```

Or in the repository:

```
packages/aps/schemas/aps.schema.json
```

## Using the Schema

### In VS Code

Add to `.vscode/settings.json`:

```json
{
  "yaml.schemas": {
    "https://eddacraft.dev/schemas/aps/v1.0/aps.schema.json": "plans/**/*.aps.md"
  }
}
```

### Programmatic Validation

```typescript
import Ajv from 'ajv';
import apsSchema from '@anvil/aps/schemas/aps.schema.json';

const ajv = new Ajv();
const validate = ajv.compile(apsSchema);

const plan = parsePlan('plans/index.aps.md');
const valid = validate(plan);

if (!valid) {
  console.error(validate.errors);
}
```

## Schema Structure

### Root Object

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "format": { "const": "aps" },
    "version": { "type": "string", "pattern": "^\\d+\\.\\d+$" },
    "hash": { "type": "string", "pattern": "^sha256:[a-f0-9]{64}$" },
    "title": { "type": "string" },
    "modules": { "type": "array", "items": { "$ref": "#/$defs/Module" } }
  },
  "required": ["format", "version"]
}
```

### Module Definition

```json
{
  "$defs": {
    "Module": {
      "type": "object",
      "properties": {
        "id": { "type": "string", "pattern": "^[a-z][a-z0-9-]*$" },
        "title": { "type": "string" },
        "depends_on": { "type": "array", "items": { "type": "string" } },
        "files": { "type": "array", "items": { "type": "string" } },
        "tasks": { "type": "array", "items": { "$ref": "#/$defs/Task" } }
      },
      "required": ["id"]
    }
  }
}
```

### Task Definition

```json
{
  "$defs": {
    "Task": {
      "type": "object",
      "properties": {
        "id": { "type": "string", "pattern": "^[A-Z]+-\\d{3}$" },
        "title": { "type": "string" },
        "status": {
          "type": "string",
          "enum": ["pending", "in_progress", "complete", "blocked"]
        },
        "outcome": { "type": "string" },
        "validation": { "type": "string" },
        "dependencies": { "type": "array", "items": { "type": "string" } },
        "steps": { "type": "array", "items": { "$ref": "#/$defs/Step" } }
      },
      "required": ["id", "outcome", "validation"]
    }
  }
}
```

### Step Definition

```json
{
  "$defs": {
    "Step": {
      "type": "object",
      "properties": {
        "text": { "type": "string", "maxLength": 100 },
        "complete": { "type": "boolean" }
      },
      "required": ["text"]
    }
  }
}
```

## Validation Errors

Common validation errors:

### Missing Required Field

```json
{
  "keyword": "required",
  "message": "must have required property 'outcome'",
  "params": { "missingProperty": "outcome" }
}
```

### Invalid Task ID

```json
{
  "keyword": "pattern",
  "message": "must match pattern \"^[A-Z]+-\\d{3}$\"",
  "instancePath": "/modules/0/tasks/0/id"
}
```

### Invalid Status

```json
{
  "keyword": "enum",
  "message": "must be equal to one of the allowed values",
  "params": {
    "allowedValues": ["pending", "in_progress", "complete", "blocked"]
  }
}
```

## TypeScript Types

Generated from the schema:

```typescript
export interface APSPlan {
  format: 'aps';
  version: string;
  hash?: string;
  title?: string;
  modules?: APSModule[];
}

export interface APSModule {
  id: string;
  title?: string;
  depends_on?: string[];
  files?: string[];
  tasks?: APSTask[];
}

export interface APSTask {
  id: string;
  title?: string;
  status?: 'pending' | 'in_progress' | 'complete' | 'blocked';
  outcome: string;
  validation: string;
  dependencies?: string[];
  steps?: APSStep[];
}

export interface APSStep {
  text: string;
  complete?: boolean;
}
```

Import from the package:

```typescript
import type { APSPlan, APSTask } from '@anvil/aps';
```

---

**Next:** [Examples →](/docs/aps/schemas/examples)
