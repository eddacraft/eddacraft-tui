# Adapter Development Guide

| Type   | Authority | Owner  | Status | Freshness                                                                  |
| ------ | --------- | ------ | ------ | -------------------------------------------------------------------------- |
| README | Advisory  | DOCGOV | Live   | Last reviewed 2026-08-12 against `docs/guides/documentation-governance.md` |

| Upstream                                  | Downstream              |
| ----------------------------------------- | ----------------------- |
| `docs/guides/documentation-governance.md` | Adapter guide discovery |

**Last Updated:** 2025-10-23

## Overview

This guide covers developing format adapters for Anvil, which enable conversion
between external planning formats (SpecKit, BMAD, etc.) and the internal Anvil
Plan Specification (APS).

## What is an Adapter?

An adapter is a bidirectional translator between an external format and APS:

```
External Format (e.g., SpecKit)
        ↕ Adapter
Anvil Plan Specification (APS)
```

## Adapter Types

### Import Adapter

Converts external format → APS

**Use cases:**

- Load existing plans from SpecKit/BMAD
- Validate external documents
- Generate APS from intent

### Export Adapter

Converts APS → external format

**Use cases:**

- Export APS to user's preferred format
- Generate documentation
- Preserve original format conventions

## Implementation Guides

### [Adapter Workflow Guide](workflow-guide.md)

Step-by-step process for building a new adapter.

**Contents:**

- Project setup
- Implementing import/export
- Testing strategies
- Integration with CLI

### Package Documentation

- [Adapters Package README](../../../packages/adapters/README.md)
- Adapter Workflow Guide (removed — see [workflow-guide.md](workflow-guide.md))

## Reference Implementations

### SpecKit Adapter (Complete)

**Location:** `packages/adapters/src/speckit/`

**Files:**

- `import.ts` - V1 import adapter
- `import-v2.ts` - V2 official format
- `export.ts` - Export adapter
- `parser.ts` - Core markdown parser
- `parsers/` - Specialized parsers

**Test Coverage:** 69/69 tests passing (100%)

**Learn from:**

- Markdown parsing patterns
- Change extraction logic
- Metadata handling
- Error reporting

### BMAD Adapter (Planned)

**Status:** Planning phase **PRD:** In development **Templates:**
[BMAD Templates](../../archive/bmad-adapter-spec.md)

## Key Concepts

### APS (Anvil Plan Specification)

The internal canonical format:

```typescript
interface APSPlan {
  id: string;
  hash: string;
  intent: string;
  proposed_changes: Change[];
  provenance: Provenance;
  validations: Validation;
  // ... see core/src/schema/aps.schema.ts
}
```

### Deterministic Hashing

All APS plans have a SHA-256 hash for:

- Version tracking
- Change detection
- Evidence correlation

### Adapter Registry

Central registry for format detection:

- Auto-detects format from content
- Routes to appropriate adapter
- Confidence-based selection

## Development Workflow

### 1. Setup

```bash
cd packages/adapters
pnpm install
```

### 2. Create Adapter Files

```
packages/adapters/src/my-format/
├── import.ts       # Import adapter
├── export.ts       # Export adapter
├── parser.ts       # Format-specific parser
└── __tests__/
    ├── import.test.ts
    └── export.test.ts
```

### 3. Implement BaseAdapter

```typescript
export class MyFormatImportAdapter extends BaseAdapter {
  async convertToAPS(spec: ExternalSpec): Promise<ConversionResult<APSPlan>> {
    // Parse external format
    // Extract changes
    // Build APS structure
  }
}
```

### 4. Add Tests

```bash
pnpm test
```

### 5. Register Adapter

```typescript
// packages/adapters/src/index.ts
import { MyFormatAdapter } from './my-format/index.js';
registry.register(new MyFormatAdapter());
```

## Testing

### Unit Tests

Test individual functions:

```typescript
describe('MyFormatParser', () => {
  it('should parse valid format', () => {
    const result = parse(validInput);
    expect(result.success).toBe(true);
  });
});
```

### Integration Tests

Test end-to-end conversion:

```typescript
describe('MyFormatAdapter', () => {
  it('should convert to APS', async () => {
    const aps = await adapter.convertToAPS(externalSpec);
    expect(aps.proposed_changes).toBeDefined();
  });
});
```

### Fixtures

Use real-world examples:

```
packages/adapters/src/__tests__/fixtures/my-format/
├── sample-spec.md
├── sample-plan.md
└── expected-aps.json
```

## Best Practices

### 1. Preserve User Intent

- Don't modify original meaning
- Keep terminology consistent
- Maintain section structure when possible

### 2. Handle Edge Cases

- Empty sections
- Missing required fields
- Invalid syntax
- Special characters

### 3. Provide Clear Errors

```typescript
if (!spec.intent) {
  return {
    success: false,
    errors: [
      {
        code: 'MISSING_INTENT',
        message: 'Intent section is required',
        line: 0,
      },
    ],
  };
}
```

### 4. Test Thoroughly

- Happy path
- Edge cases
- Error conditions
- Real-world examples

### 5. Document Format Assumptions

```typescript
/**
 * Parses SpecKit v2 format
 *
 * Expects:
 * - Markdown with ## headers
 * - Intent in ## Intent section
 * - Changes in ## User Scenarios
 * - FR-XXX: pattern for requirements
 */
```

## Common Pitfalls

### ❌ Don't hardcode assumptions

```typescript
// Bad
const intent = lines[5]; // Assumes intent is always line 5

// Good
const intent = extractSection(content, 'Intent');
```

### ❌ Don't ignore validation

```typescript
// Bad
return { success: true, data: aps };

// Good
const validationResult = validateAPS(aps);
if (!validationResult.valid) {
  return { success: false, errors: validationResult.errors };
}
```

### ❌ Don't lose information

```typescript
// Bad - loses priority info
const change = { type: 'file_create', path: file };

// Good - preserves metadata
const change = {
  type: 'file_create',
  path: file,
  metadata: { priority: 'P1' },
};
```

## Next Steps

1. Review [Adapter Workflow Guide](workflow-guide.md)
2. Study
   [SpecKit adapter implementation](../../../packages/adapters/src/speckit/)
3. Check format templates in the adapters package
4. Start building your adapter!

## Navigation

- [Back to Guides](../README.md)
- [Adapter Workflow Guide](workflow-guide.md)
- [Package Documentation](../../../packages/adapters/README.md)
- [Documentation Index](../README.md)
