# Adapters Package (@eddacraft/anvil-adapters)

> Format conversion framework: SpecKit, BMAD, Generic Markdown → APS

**Parent**: See root `AGENTS.md` for project-wide conventions.

## Structure

```
packages/adapters/src/
├── base/               # Framework foundation
│   ├── types.ts        # FormatAdapter interface (313 lines)
│   ├── registry.ts     # AdapterRegistry singleton (231 lines)
│   ├── utils.ts        # Helper functions
│   └── file-discovery.ts  # Auto-find planning documents
├── speckit/            # GitHub SpecKit adapter (538 lines)
│   └── format-adapter.ts
├── bmad/               # BMAD PRD/architecture adapter (188 lines)
│   └── format-adapter.ts
├── generic/            # Generic markdown fallback (198 lines)
│   └── format-adapter.ts
└── index.ts            # Auto-registers all adapters on import
```

## Where to Look

| Task             | Location                              | Notes                        |
| ---------------- | ------------------------------------- | ---------------------------- |
| Add new adapter  | Create `src/{name}/format-adapter.ts` | Implement FormatAdapter      |
| Modify detection | Adapter's `detect()` method           | Adjust confidence weights    |
| Add utility      | `base/utils.ts`                       | Shared across adapters       |
| Change registry  | `base/registry.ts`                    | Singleton, priority ordering |

## FormatAdapter Interface

```typescript
interface FormatAdapter {
  readonly metadata: AdapterMetadata; // name, version, formats, extensions

  // Confidence-based format detection (0-100)
  detect(content: string): DetectionResult;

  // External format → APS
  parse(
    content: string,
    context?: ParseContext,
    options?: ParseOptions
  ): Promise<ParseResult>;

  // APS → External format
  serialize(
    plan: APSPlan,
    options?: SerializeOptions
  ): Promise<SerializeResult>;

  // Fast validation without full conversion
  validate(
    content: string,
    options?: ValidateOptions
  ): Promise<ValidationResult>;
}
```

## Creating a New Adapter

1. Create directory: `src/{name}/`
2. Implement adapter:

```typescript
import { BaseFormatAdapter } from '../base/types.js';
import type {
  DetectionResult,
  ParseResult,
  SerializeResult,
} from '../base/types.js';

export class MyFormatAdapter extends BaseFormatAdapter {
  readonly metadata = {
    name: 'my-format',
    version: '1.0.0',
    formats: ['my-format'],
    extensions: ['.myf.md'],
  };

  detect(content: string): DetectionResult {
    let confidence = 0;

    // Check for format-specific markers
    if (content.includes('# My Format Header')) confidence += 30;
    if (/^---\nformat: my-format/m.test(content)) confidence += 40;

    return this.createDetection(confidence, {
      hasHeader: confidence > 0,
    });
  }

  async parse(content: string): Promise<ParseResult> {
    // Convert to APS...
    return { success: true, plan: apsPlan, warnings: [] };
  }

  async serialize(plan: APSPlan): Promise<SerializeResult> {
    // Convert from APS...
    return { success: true, content: markdown };
  }
}
```

3. Register in `index.ts`:

```typescript
import { MyFormatAdapter } from './my-format/format-adapter.js';
baseRegistry.register(new MyFormatAdapter());
```

## Detection Strategy

**Confidence Scoring** (0-100, threshold 50%):

| Indicator         | Weight | Example                          |
| ----------------- | ------ | -------------------------------- |
| Format header     | 30     | `# Specification`                |
| YAML front-matter | 30     | `---\nformat: speckit\n---`      |
| Section markers   | 15-20  | `## Intent`, `## Changes`        |
| ID patterns       | 20-25  | `REQ-001`, `TASK-001`            |
| Keywords          | 10-15  | `proposed_changes`, `provenance` |

**Priority Order**: SpecKit (100) > BMAD (90) > Generic (10)

Generic adapter has 30% threshold as fallback.

## AdapterRegistry

```typescript
// Get singleton instance
const registry = AdapterRegistry.getInstance();

// Auto-detect format (returns best match above threshold)
const detected = registry.detectAdapter(content, 0.5);
if (detected) {
  const { adapter, detection } = detected;
  const result = await adapter.parse(content);
}

// Get specific adapter
const speckit = registry.getAdapterForFormat('speckit');
```

## Utility Functions

```typescript
import {
  createDetection, // Build DetectionResult
  createError, // Build AdapterError
  createWarning, // Build AdapterWarning
  calculateConfidence, // Normalise to 0-100
  mergeParseResults, // Combine multiple results
} from './base/utils.js';
```

## Scripts

```bash
nx test adapters                          # All adapter tests
nx test adapters --testNamePattern="BMAD" # BMAD tests only
```

## Anti-Patterns (This Package)

- Never hardcode confidence thresholds - use constants
- Never skip detection step before parsing
- Always return structured errors (AdapterError), never throw
- Always preserve unknown fields during round-trip

## Testing

- 114+ SpecKit tests, 86+ BMAD tests, 32+ Generic tests
- Fixture-based with real document samples
- Test detection, parsing, serialisation, and round-trip fidelity
- Test files in `__tests__/fixtures/`
