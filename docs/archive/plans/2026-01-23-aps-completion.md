<\!-- Archived: 2026-03-01 | Reason: Implementation complete —
aps-markdown-adapter module is archived -->

# APS Completion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to
> implement this plan task-by-task.

**Goal:** Complete APS ecosystem by updating stale documentation, creating the
APS Markdown adapter, and verifying CLI integration.

**Architecture:** The `@eddacraft/anvil-aps` parser library is already complete.
We need: (1) documentation cleanup, (2) an adapter in `packages/adapters` that
converts parsed APS documents to APSPlan execution schema, and (3) verification
that CLI commands work correctly.

**Tech Stack:** TypeScript, remark/unified (already in aps), Vitest for testing

---

## Task 1: Clean Up Stale TODO.md

**Files:**

- Modify: `packages/aps/TODO.md`

**Step 1: Update TODO.md to reflect actual completion**

Replace the entire content with accurate status:

```markdown
# APS Package Status

> **Full Plan**: `plans/index.aps.md` (module: aps-markdown-adapter)
>
> This package is **complete**. See the plan document for adapter work.

---

## Package Status: Complete ✅

The `@eddacraft/anvil-aps` library provides:

- **Parser** — `parseDocument()`, `parseIndex()`, `parseTask()`
- **Loader** — `loadPlan()` with multi-module support
- **Filter** — `filterPlan()` with scope/tag/owner/task filtering
- **Validator** — `validatePlanningDoc()` with issue reporting
- **State** — Task locking/unlocking via `.anvil/state.json`
- **Templates** — `generateIndexTemplate()`, `generateLeafTemplate()`

### Test Coverage

| Module    | Coverage  | Tests   |
| --------- | --------- | ------- |
| parser    | 89.5%     | 31      |
| loader    | 96%       | 18      |
| validator | 97.9%     | 27      |
| state     | 94.2%     | 35      |
| filter    | 89.6%     | 30      |
| templates | 100%      | 26      |
| **Total** | **93.1%** | **167** |

---

## Related Work

See `plans/modules/aps-markdown-adapter.aps.md` for adapter integration.

---

_Last updated: January 2026_
```

**Step 2: Commit**

```bash
git add packages/aps/TODO.md
git commit -m "docs(aps): update TODO.md to reflect completed status

The @eddacraft/anvil-aps library is complete with 167 tests and 93% coverage.
Remaining work tracked in plans/modules/aps-markdown-adapter.aps.md.

Authored-By: Aneki (joshuaboys)"
```

---

## Task 2: Create APS Markdown Adapter - Detection

**Files:**

- Create: `packages/adapters/src/aps-markdown/index.ts`
- Create: `packages/adapters/src/aps-markdown/adapter.ts`
- Create: `packages/adapters/src/aps-markdown/__tests__/adapter.test.ts`

**Step 1: Create adapter directory structure**

```bash
mkdir -p packages/adapters/src/aps-markdown/__tests__
```

**Step 2: Write the failing detection test**

Create `packages/adapters/src/aps-markdown/__tests__/adapter.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { APSMarkdownAdapter } from '../adapter.js';

describe('APSMarkdownAdapter', () => {
  const adapter = new APSMarkdownAdapter();

  describe('metadata', () => {
    it('has correct metadata', () => {
      expect(adapter.metadata.name).toBe('aps-markdown');
      expect(adapter.metadata.extensions).toContain('.aps.md');
      expect(adapter.metadata.formats).toContain('aps');
    });
  });

  describe('detect', () => {
    it('detects .aps.md content with Tasks section', () => {
      const content = `# Feature Plan

**Scope:** AUTH **Owner:** @alice

## Tasks

### AUTH-001: Implement login

**Intent:** Create login endpoint
`;
      const result = adapter.detect(content);
      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(80);
    });

    it('detects index file with Modules section', () => {
      const content = `# Project Plan

## Modules

### auth

- **Path:** [./modules/auth.aps.md](./modules/auth.aps.md)
- **Scope:** AUTH
`;
      const result = adapter.detect(content);
      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(70);
    });

    it('does not detect regular markdown', () => {
      const content = `# README

This is a regular readme file.

## Installation

Run npm install.
`;
      const result = adapter.detect(content);
      expect(result.detected).toBe(false);
    });

    it('does not detect SpecKit format', () => {
      const content = `# Feature: User Login

## User Story
As a user I want to login

## Acceptance Criteria
- Given valid credentials
- When I submit login form
- Then I am authenticated
`;
      const result = adapter.detect(content);
      expect(result.detected).toBe(false);
    });
  });
});
```

**Step 3: Run test to verify it fails**

```bash
pnpm --filter @eddacraft/anvil-adapters test -- --grep "APSMarkdownAdapter"
```

Expected: FAIL - cannot find module '../adapter.js'

**Step 4: Implement adapter with detection**

Create `packages/adapters/src/aps-markdown/adapter.ts`:

```typescript
/**
 * APS Markdown Format Adapter
 *
 * Converts APS markdown planning documents (.aps.md) to APSPlan execution schema.
 * This bridges the human-readable planning format with the deterministic execution layer.
 */

import type { APSPlan, ValidationResult } from '@eddacraft/anvil-core';
import {
  BaseFormatAdapter,
  type AdapterMetadata,
  type DetectionResult,
  type ParseResult,
  type SerializeResult,
  type ParseContext,
  type AdapterOptions,
} from '../base/types.js';

/**
 * Adapter for APS Markdown planning documents
 */
export class APSMarkdownAdapter extends BaseFormatAdapter {
  readonly metadata: AdapterMetadata = {
    name: 'aps-markdown',
    version: '1.0.0',
    displayName: 'APS Markdown',
    description: 'Anvil Planning Spec markdown documents (.aps.md)',
    extensions: ['.aps.md'],
    formats: ['aps', 'aps-markdown'],
  };

  /**
   * Detect if content is an APS markdown document
   *
   * Detection criteria:
   * - Has H1 title
   * - Has either "## Tasks" section (leaf spec) or "## Modules" section (index)
   * - Task headings follow SCOPE-NNN pattern
   * - Module entries have Path field with .aps.md links
   */
  detect(content: string): DetectionResult {
    const lines = content.split('\n');

    // Must have H1 title
    const hasH1 = lines.some((line) => /^#\s+.+/.test(line));
    if (!hasH1) {
      return { detected: false, confidence: 0, reason: 'No H1 title found' };
    }

    // Check for Tasks section with task headings
    const hasTasksSection = /^##\s+Tasks\s*$/im.test(content);
    const hasTaskHeading = /^###\s+[A-Z0-9]{1,10}-\d{3}:/m.test(content);
    const hasIntentField = /\*\*Intent:\*\*/i.test(content);

    if (hasTasksSection && hasTaskHeading && hasIntentField) {
      return {
        detected: true,
        confidence: 90,
        reason: 'APS leaf spec with Tasks section and valid task headings',
      };
    }

    // Check for Modules section (index file)
    const hasModulesSection = /^##\s+Modules\s*$/im.test(content);
    const hasPathField = /\*\*Path:\*\*.*\.aps\.md/i.test(content);
    const hasScopeField = /\*\*Scope:\*\*/i.test(content);

    if (hasModulesSection && hasPathField) {
      return {
        detected: true,
        confidence: 85,
        reason: 'APS index file with Modules section and .aps.md paths',
      };
    }

    // Partial detection - has some APS markers but not complete
    if (hasTasksSection || hasModulesSection || hasScopeField) {
      return {
        detected: false,
        confidence: 30,
        reason: 'Has some APS markers but missing required sections',
      };
    }

    return {
      detected: false,
      confidence: 0,
      reason: 'No APS markers found',
    };
  }

  async parse(
    _content: string,
    _context?: ParseContext,
    _options?: AdapterOptions
  ): Promise<ParseResult> {
    // TODO: Implement in Task 3
    return this.createParseError([
      { code: 'NOT_IMPLEMENTED', message: 'Parse not yet implemented' },
    ]);
  }

  async serialize(
    _plan: APSPlan,
    _options?: AdapterOptions
  ): Promise<SerializeResult> {
    // TODO: Implement in Task 5
    return this.createSerializeError([
      { code: 'NOT_IMPLEMENTED', message: 'Serialize not yet implemented' },
    ]);
  }

  async validate(
    _content: string,
    _options?: AdapterOptions
  ): Promise<ValidationResult> {
    // TODO: Implement in Task 4
    return { valid: false, errors: [], warnings: [] };
  }
}

/**
 * Factory function to create adapter instance
 */
export function createAPSMarkdownAdapter(
  options?: AdapterOptions
): APSMarkdownAdapter {
  return new APSMarkdownAdapter(options);
}
```

**Step 5: Create index export**

Create `packages/adapters/src/aps-markdown/index.ts`:

```typescript
export { APSMarkdownAdapter, createAPSMarkdownAdapter } from './adapter.js';
```

**Step 6: Run test to verify it passes**

```bash
pnpm --filter @eddacraft/anvil-adapters test -- --grep "APSMarkdownAdapter"
```

Expected: PASS (4 tests)

**Step 7: Commit**

```bash
git add packages/adapters/src/aps-markdown/
git commit -m "feat(adapters): add APSMarkdownAdapter with detection

Implements APSMD-002: Detect APS markdown format with confidence scoring.
- Detects leaf specs (Tasks section with SCOPE-NNN headings)
- Detects index files (Modules section with .aps.md paths)
- Returns confidence scores for format matching

Authored-By: Aneki (joshuaboys)"
```

---

## Task 3: Implement APS Markdown Parsing

**Files:**

- Modify: `packages/adapters/src/aps-markdown/adapter.ts`
- Modify: `packages/adapters/src/aps-markdown/__tests__/adapter.test.ts`
- Create: `packages/adapters/src/aps-markdown/__tests__/__fixtures__/`

**Step 1: Create test fixtures**

Create
`packages/adapters/src/aps-markdown/__tests__/__fixtures__/simple-leaf.aps.md`:

```markdown
# Authentication Feature

**Scope:** AUTH **Owner:** @alice **Priority:** high

## Tasks

### AUTH-001: Implement login endpoint

**Intent:** Create POST /auth/login endpoint with JWT response **Expected
Outcome:** Returns JWT token on success, 401 on failure **Validation:**
`pnpm test -- --grep "login"` **Confidence:** high **Tags:** security, api
**Files:** src/auth/login.ts, src/auth/jwt.ts

### AUTH-002: Add password reset

**Intent:** Implement password reset flow with email verification
**Confidence:** medium **Dependencies:** AUTH-001
```

**Step 2: Write failing parse test**

Add to `packages/adapters/src/aps-markdown/__tests__/adapter.test.ts`:

```typescript
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

describe('parse', () => {
  it('parses a leaf spec to APSPlan', async () => {
    const content = readFileSync(
      join(__dirname, '__fixtures__/simple-leaf.aps.md'),
      'utf-8'
    );

    const result = await adapter.parse(content, {
      repositoryPath: '/test/repo',
      author: 'test-user',
    });

    expect(result.success).toBe(true);
    expect(result.data).toBeDefined();
    expect(result.data!.intent).toContain('Authentication Feature');
    expect(result.data!.proposed_changes).toHaveLength(2);

    // Check first change
    const change1 = result.data!.proposed_changes[0];
    expect(change1.description).toContain('Implement login endpoint');
    expect(change1.metadata?.taskId).toBe('AUTH-001');
    expect(change1.metadata?.confidence).toBe('high');
  });

  it('maps task fields to change metadata', async () => {
    const content = readFileSync(
      join(__dirname, '__fixtures__/simple-leaf.aps.md'),
      'utf-8'
    );

    const result = await adapter.parse(content);

    expect(result.success).toBe(true);
    const change = result.data!.proposed_changes[0];
    expect(change.metadata?.validation).toBe('pnpm test -- --grep "login"');
    expect(change.metadata?.tags).toEqual(['security', 'api']);
    expect(change.metadata?.files).toEqual([
      'src/auth/login.ts',
      'src/auth/jwt.ts',
    ]);
  });

  it('preserves task dependencies in change metadata', async () => {
    const content = readFileSync(
      join(__dirname, '__fixtures__/simple-leaf.aps.md'),
      'utf-8'
    );

    const result = await adapter.parse(content);

    expect(result.success).toBe(true);
    const change2 = result.data!.proposed_changes[1];
    expect(change2.metadata?.taskId).toBe('AUTH-002');
    expect(change2.metadata?.dependencies).toEqual(['AUTH-001']);
  });
});
```

**Step 3: Run test to verify it fails**

```bash
pnpm --filter @eddacraft/anvil-adapters test -- --grep "parse"
```

Expected: FAIL - parse returns NOT_IMPLEMENTED error

**Step 4: Implement parse method**

Update `packages/adapters/src/aps-markdown/adapter.ts`:

```typescript
import {
  parseDocument,
  type ParsedDocument,
  type Task,
} from '@eddacraft/anvil-aps';
import {
  generatePlanId,
  generateHash,
  APS_SCHEMA_VERSION,
  type APSPlan,
  type Change,
} from '@eddacraft/anvil-core';

// ... existing code ...

async parse(
  content: string,
  context?: ParseContext,
  _options?: AdapterOptions
): Promise<ParseResult> {
  try {
    // Parse using @eddacraft/anvil-aps parser
    const doc = await parseDocument(content, context?.repositoryPath);

    // Convert to APSPlan
    const plan = this.convertToAPSPlan(doc, context);

    return this.createParseSuccess(plan);
  } catch (error) {
    return this.createParseError([
      {
        code: 'PARSE_ERROR',
        message: error instanceof Error ? error.message : String(error),
      },
    ]);
  }
}

/**
 * Convert parsed APS document to APSPlan execution schema
 */
private convertToAPSPlan(doc: ParsedDocument, context?: ParseContext): APSPlan {
  const planId = context?.planId ?? generatePlanId();
  const timestamp = context?.timestamp ?? new Date().toISOString();

  // Build intent from document title and metadata
  const intent = doc.metadata?.scope
    ? `${doc.title} (Scope: ${doc.metadata.scope})`
    : doc.title;

  // Convert tasks to proposed_changes
  const proposed_changes = doc.tasks.map((task) => this.taskToChange(task));

  const planWithoutHash: Omit<APSPlan, 'hash'> = {
    schema_version: APS_SCHEMA_VERSION,
    id: planId,
    intent,
    proposed_changes,
    provenance: {
      timestamp,
      author: context?.author ?? process.env['USER'] ?? 'unknown',
      source: 'aps-markdown-adapter',
      version: this.metadata.version,
      repository: context?.repositoryPath ?? process.cwd(),
      branch: context?.branch ?? 'main',
      commit: context?.commit ?? '',
    },
    validations: {
      required_checks: ['lint', 'test'],
      skip_checks: [],
    },
    evidence: [],
    executions: [],
  };

  // Generate hash
  const hash = generateHash(planWithoutHash);

  return { ...planWithoutHash, hash } as APSPlan;
}

/**
 * Convert a single Task to a Change
 */
private taskToChange(task: Task): Change {
  // Determine change type based on task content
  const changeType = this.inferChangeType(task);

  return {
    type: changeType,
    path: task.files?.[0] ?? `task/${task.id}`,
    description: `${task.id}: ${task.title}\n\n${task.intent}`,
    rationale: task.expectedOutcome ?? task.intent,
    metadata: {
      taskId: task.id,
      confidence: task.confidence,
      validation: task.validation,
      tags: task.tags,
      files: task.files,
      scopes: task.scopes,
      dependencies: task.dependencies,
      risks: task.risks,
    },
  };
}

/**
 * Infer change type from task content
 */
private inferChangeType(task: Task): Change['type'] {
  const intent = task.intent.toLowerCase();
  const title = task.title.toLowerCase();

  if (intent.includes('create') || intent.includes('add') || title.includes('implement')) {
    return 'file_create';
  }
  if (intent.includes('update') || intent.includes('modify') || intent.includes('fix')) {
    return 'file_update';
  }
  if (intent.includes('delete') || intent.includes('remove')) {
    return 'file_delete';
  }
  if (intent.includes('config') || intent.includes('setting')) {
    return 'config_update';
  }

  // Default to script_execute for tasks without clear file operations
  return 'script_execute';
}
```

**Step 5: Add @eddacraft/anvil-aps dependency**

```bash
cd packages/adapters
pnpm add @eddacraft/anvil-aps@workspace:*
```

**Step 6: Run test to verify it passes**

```bash
pnpm --filter @eddacraft/anvil-adapters test -- --grep "parse"
```

Expected: PASS

**Step 7: Commit**

```bash
git add packages/adapters/
git commit -m "feat(adapters): implement APSMarkdownAdapter parse method

Implements APSMD-003: Convert parsed Tasks to APSPlan proposed_changes.
- Uses @eddacraft/anvil-aps parser for markdown processing
- Maps task fields to Change metadata
- Infers change type from task intent
- Preserves dependencies and validation commands

Authored-By: Aneki (joshuaboys)"
```

---

## Task 4: Register Adapter in Registry

**Files:**

- Modify: `packages/adapters/src/index.ts`

**Step 1: Write failing registration test**

Add to test file:

```typescript
import { registry } from '../../index.js';

describe('registry integration', () => {
  it('is registered in format registry', () => {
    const detected = registry.detect(`# Plan

## Tasks

### TEST-001: Test task

**Intent:** Test intent
`);

    const apsAdapter = detected.find(
      (d) => d.adapter.metadata.name === 'aps-markdown'
    );
    expect(apsAdapter).toBeDefined();
    expect(apsAdapter!.result.detected).toBe(true);
  });
});
```

**Step 2: Run test to verify it fails**

```bash
pnpm --filter @eddacraft/anvil-adapters test -- --grep "registry integration"
```

Expected: FAIL - aps-markdown not found in registry

**Step 3: Register adapter**

Update `packages/adapters/src/index.ts`:

```typescript
// ... existing imports ...

// Export APS Markdown adapter
export * from './aps-markdown/index.js';

// ... existing auto-register code ...

import { APSMarkdownAdapter } from './aps-markdown/index.js';

// Register adapters in priority order
// APS Markdown has highest priority for .aps.md files
baseRegistry.register(new APSMarkdownAdapter());
baseRegistry.register(new BMADFormatAdapter());
baseRegistry.register(new SpecKitFormatAdapter());
baseRegistry.register(new GenericMarkdownAdapter());
```

**Step 4: Run test to verify it passes**

```bash
pnpm --filter @eddacraft/anvil-adapters test -- --grep "registry integration"
```

Expected: PASS

**Step 5: Commit**

```bash
git add packages/adapters/src/index.ts
git commit -m "feat(adapters): register APSMarkdownAdapter in format registry

Implements APSMD-006: Make APS markdown adapter discoverable.
- Registered with highest priority for .aps.md files
- Auto-detection includes APS markdown format

Authored-By: Aneki (joshuaboys)"
```

---

## Task 5: Verify CLI Integration

**Files:**

- None (verification only)

**Step 1: Build packages**

```bash
pnpm build
```

**Step 2: Test CLI plan commands with APS plans**

```bash
# Validate the main plan
pnpm anvil plan validate plans/index.aps.md

# Load a specific module
pnpm anvil plan load plans/modules/aps-markdown-adapter.aps.md

# Load with filters
pnpm anvil plan load plans/index.aps.md --scope APSMD --json
```

**Step 3: Verify output**

Expected:

- `validate` shows valid plan or specific issues
- `load` shows modules and tasks
- `--json` outputs proper context bundle

**Step 4: Document any issues found**

If issues found, create follow-up tasks in
`plans/modules/aps-markdown-adapter.aps.md`.

**Step 5: Final commit (if all passes)**

```bash
git add .
git commit -m "test(aps): verify CLI integration with APS plans

- anvil plan validate works with plans/index.aps.md
- anvil plan load works with module filtering
- JSON context bundle output verified

Authored-By: Aneki (joshuaboys)"
```

---

## Summary

| Task | Description                         | Status |
| ---- | ----------------------------------- | ------ |
| 1    | Clean up stale TODO.md              | Ready  |
| 2    | Create APSMarkdownAdapter detection | Ready  |
| 3    | Implement parse method              | Ready  |
| 4    | Register in format registry         | Ready  |
| 5    | Verify CLI integration              | Ready  |

**Total: 5 tasks, ~15 steps**

---

## Notes

- Task 3 may need refinement based on actual Change schema in
  @eddacraft/anvil-core
- Serialization (APSMD-005) deferred - parse is the primary use case
- Multi-module support (APSMD-004) handled by @eddacraft/anvil-aps loader
