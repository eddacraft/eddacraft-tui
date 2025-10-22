# Next Steps: SpecKit Adapter Interface Migration

**Priority:** Medium **Estimated Effort:** 4-6 hours **Blocking:** No
(workaround available with explicit `--format` flag)

## Context

The SpecKit adapters are fully functional and tested (69/69 tests passing), but
use the legacy `BaseAdapter` interface. To enable format auto-detection in the
CLI, they need to implement the unified `FormatAdapter` interface.

## Implementation Plan

### Phase 1: Create Wrapper Adapter (2-3 hours)

**File:** `packages/adapters/src/speckit/format-adapter.ts`

```typescript
import type { FormatAdapter, FormatMetadata } from '../base/types.js';
import { SpecKitImportAdapter } from './import.js';
import { SpecKitExportAdapter } from './export.js';
import type { APSPlan } from '@anvil/core';

export class SpecKitFormatAdapter implements FormatAdapter {
  private importAdapter: SpecKitImportAdapter;
  private exportAdapter: SpecKitExportAdapter;

  metadata: FormatMetadata = {
    name: 'speckit',
    version: '2.0.0',
    description: 'GitHub SpecKit format adapter',
    author: 'Anvil Team',
    filePatterns: ['spec.md', 'plan.md', 'tasks.md'],
    confidence: {
      min: 50,
      high: 85,
    },
  };

  constructor() {
    this.importAdapter = new SpecKitImportAdapter();
    this.exportAdapter = new SpecKitExportAdapter();
  }

  /**
   * Detect if content is SpecKit format
   */
  detect(content: string): { confidence: number; indicators: string[] } {
    let confidence = 0;
    const indicators: string[] = [];

    // SpecKit-specific section headers
    const specKitMarkers = [
      { pattern: /^## Intent$/m, points: 30, name: 'Intent section' },
      {
        pattern: /^## User Scenarios & Testing$/m,
        points: 25,
        name: 'User Scenarios section',
      },
      {
        pattern: /^## Functional Requirements$/m,
        points: 20,
        name: 'Functional Requirements section',
      },
      {
        pattern: /^## Key Entities$/m,
        points: 15,
        name: 'Key Entities section',
      },
      {
        pattern: /^# (Specification|Implementation Plan|Tasks):/m,
        points: 20,
        name: 'SpecKit title format',
      },
      {
        pattern: /\*\*FR-\d+:/,
        points: 15,
        name: 'Functional requirement format',
      },
      { pattern: /\*\*Scenario:/, points: 10, name: 'Scenario format' },
      {
        pattern: /\*\*Acceptance Criteria:\*\*/m,
        points: 10,
        name: 'Acceptance criteria',
      },
      {
        pattern: /^### Priority \d+ \(P\d+\)/m,
        points: 10,
        name: 'Priority markers',
      },
    ];

    for (const marker of specKitMarkers) {
      if (marker.pattern.test(content)) {
        confidence += marker.points;
        indicators.push(marker.name);
      }
    }

    // Markdown file is expected
    if (content.startsWith('#')) {
      confidence += 5;
      indicators.push('Markdown format');
    }

    return {
      confidence: Math.min(confidence, 100),
      indicators,
    };
  }

  /**
   * Parse SpecKit content to APS
   */
  async parse(content: string): Promise<APSPlan> {
    const result = await this.importAdapter.convertToAPS({
      format: 'speckit',
      version: '2.0.0',
      content,
    });

    if (!result.success || !result.data) {
      const errors = 'errors' in result ? result.errors : [];
      throw new Error(
        `Failed to parse SpecKit: ${errors?.map((e) => e.message).join(', ')}`
      );
    }

    return result.data;
  }

  /**
   * Serialize APS to SpecKit format
   */
  async serialize(aps: APSPlan): Promise<string> {
    const result = await this.exportAdapter.convertFromAPS(aps);

    if (!result.success || !result.data) {
      const errors = 'errors' in result ? result.errors : [];
      throw new Error(
        `Failed to serialize to SpecKit: ${errors?.map((e) => e.message).join(', ')}`
      );
    }

    // Return spec.md content (main file)
    const content = result.data.content as { specContent: string };
    return content.specContent;
  }

  /**
   * Validate SpecKit content
   */
  async validate(content: string): Promise<{
    valid: boolean;
    errors?: Array<{ message: string; line?: number }>;
  }> {
    try {
      // Parse to APS first
      const aps = await this.parse(content);

      // Validate the APS
      const validationResult = await this.importAdapter.validateSpec(aps);

      return {
        valid: validationResult.valid,
        errors: validationResult.issues?.map((issue) => ({
          message: issue.message,
        })),
      };
    } catch (error) {
      return {
        valid: false,
        errors: [
          {
            message:
              error instanceof Error ? error.message : 'Validation failed',
          },
        ],
      };
    }
  }
}
```

### Phase 2: Add Tests (1-2 hours)

**File:** `packages/adapters/src/speckit/__tests__/format-adapter.test.ts`

```typescript
import { describe, it, expect } from 'vitest';
import { SpecKitFormatAdapter } from '../format-adapter.js';
import { readFileSync } from 'fs';
import { join } from 'path';

describe('SpecKitFormatAdapter', () => {
  let adapter: SpecKitFormatAdapter;

  beforeEach(() => {
    adapter = new SpecKitFormatAdapter();
  });

  describe('detect', () => {
    it('should detect SpecKit format with high confidence', () => {
      const content = `# Specification: Test Feature

## Intent

This is a test specification.

## User Scenarios & Testing

### Priority 1 (P1) - Critical

**Scenario: User does something**
`;

      const result = adapter.detect(content);

      expect(result.confidence).toBeGreaterThan(70);
      expect(result.indicators).toContain('Intent section');
      expect(result.indicators).toContain('User Scenarios section');
    });

    it('should have low confidence for non-SpecKit content', () => {
      const content = 'Just some random text without SpecKit markers';

      const result = adapter.detect(content);

      expect(result.confidence).toBeLessThan(50);
    });
  });

  describe('parse', () => {
    it('should parse valid SpecKit content to APS', async () => {
      const content = readFileSync(
        join(
          __dirname,
          '../../../../cli/src/__tests__/fixtures/speckit/spec.md'
        ),
        'utf-8'
      );

      const aps = await adapter.parse(content);

      expect(aps).toBeDefined();
      expect(aps.intent).toBeDefined();
      expect(aps.proposed_changes).toBeInstanceOf(Array);
    });
  });

  describe('serialize', () => {
    it('should serialize APS to SpecKit format', async () => {
      const sampleAPS = {
        id: 'test-plan',
        hash: '0'.repeat(64),
        intent: 'Test plan for serialization',
        schema_version: '0.1.0' as const,
        proposed_changes: [],
        provenance: {
          timestamp: new Date().toISOString(),
          source: 'test' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const markdown = await adapter.serialize(sampleAPS);

      expect(markdown).toContain('# Specification');
      expect(markdown).toContain('## Intent');
      expect(markdown).toContain('Test plan for serialization');
    });
  });

  describe('validate', () => {
    it('should validate correct SpecKit content', async () => {
      const content = `# Specification: Valid Test

## Intent

This is a valid specification with sufficient detail for testing purposes.
It has enough content to pass validation rules.
`;

      const result = await adapter.validate(content);

      expect(result.valid).toBe(true);
      expect(result.errors).toBeUndefined();
    });

    it('should reject invalid SpecKit content', async () => {
      const content = `# Specification: Invalid

## Intent

Short
`;

      const result = await adapter.validate(content);

      expect(result.valid).toBe(false);
      expect(result.errors).toBeDefined();
    });
  });
});
```

### Phase 3: Enable Auto-Registration (30 minutes)

**File:** `packages/adapters/src/index.ts`

```typescript
// Import and register SpecKit format adapter
import { SpecKitFormatAdapter } from './speckit/format-adapter.js';

// Register adapters
baseRegistry.register(new SpecKitFormatAdapter());
```

**File:** `packages/adapters/src/speckit/index.ts`

```typescript
// Add new export
export { SpecKitFormatAdapter } from './format-adapter.js';
```

### Phase 4: Integration Testing (1 hour)

**File:** `cli/src/__tests__/cli-speckit-e2e.test.ts`

```typescript
import { describe, it, expect } from 'vitest';
import { execSync } from 'child_process';
import { readFileSync } from 'fs';

describe('CLI SpecKit End-to-End', () => {
  it('should auto-detect and validate SpecKit spec.md', () => {
    const output = execSync(
      'node dist/index.js validate src/__tests__/fixtures/speckit/spec.md',
      { cwd: 'cli', encoding: 'utf-8' }
    );

    expect(output).toContain('✓ Detected format: speckit');
    expect(output).toContain('✓ Plan is valid');
  });

  it('should export APS to SpecKit', () => {
    // Create temp APS file
    // Run export command
    // Verify spec.md, plan.md, tasks.md created
  });
});
```

## Acceptance Criteria

- [ ] `SpecKitFormatAdapter` implements all `FormatAdapter` methods
- [ ] Detection confidence > 70% for valid SpecKit documents
- [ ] Detection confidence < 50% for non-SpecKit documents
- [ ] All existing SpecKit tests still pass (69/69)
- [ ] New format-adapter tests pass (10+ new tests)
- [ ] CLI auto-detects SpecKit format without `--format` flag
- [ ] Export command works without explicit `--from` flag
- [ ] All integration tests pass

## Verification Steps

1. Run adapter tests: `npx nx test adapters`
2. Run CLI tests: `pnpm test`
3. Manual CLI test:
   ```bash
   cd cli
   node dist/index.js validate src/__tests__/fixtures/speckit/spec.md
   # Should show: "✓ Detected format: speckit (85% confidence)"
   ```
4. Manual export test:
   ```bash
   node dist/index.js export src/__tests__/fixtures/speckit/spec.md --to aps
   # Should auto-detect SpecKit format
   ```

## Rollback Plan

If issues arise, the TODO comments in `packages/adapters/src/index.ts` can be
left commented out, and users can continue using explicit `--format speckit`
flag.

## Follow-up Work (Future)

After completing this migration:

1. **BMAD Adapter** - Implement same pattern for BMAD format
2. **Evidence Injection** - Add evidence to SpecKit export in special sections
3. **Custom Templates** - Allow users to customize SpecKit output format
4. **Confidence Tuning** - Adjust detection thresholds based on user feedback
5. **Multi-file Support** - Handle spec.md + plan.md + tasks.md as single
   logical document

## Resources

- FormatAdapter interface: `packages/adapters/src/base/types.ts`
- SpecKit adapter tests: `packages/adapters/src/__tests__/speckit-*.test.ts`
- CLI integration: `cli/src/services/plan-loader.ts`
- Registry implementation: `packages/adapters/src/base/registry.ts`
