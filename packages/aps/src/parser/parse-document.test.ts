/**
 * Tests for parse-document module
 */

import { describe, it, expect } from 'vitest';
import { promises as fs } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseDocument } from './parse-document.js';
import { ParseError } from '../types/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FIXTURES_DIR = join(__dirname, '__fixtures__');

async function loadFixture(filename: string): Promise<string> {
  return fs.readFile(join(FIXTURES_DIR, filename), 'utf-8');
}

describe('parseDocument', () => {
  describe('simple plans', () => {
    it('should parse canonical Work Items section as tasks', async () => {
      const content = `# Canonical Module

## Work Items

### CANON-001: Canonical work item

**Intent:** Parse canonical work item sections
**Expected Outcome:** Work item is exposed as a task`;

      const doc = await parseDocument(content, 'canonical-module.aps.md');

      expect(doc.tasks).toHaveLength(1);
      expect(doc.tasks[0]).toMatchObject({
        id: 'CANON-001',
        title: 'Canonical work item',
        intent: 'Parse canonical work item sections',
        expectedOutcome: 'Work item is exposed as a task',
      });
    });

    it('should parse a simple plan with multiple tasks', async () => {
      const content = await loadFixture('simple-plan.aps.md');
      const doc = await parseDocument(content, 'simple-plan.aps.md');

      expect(doc.title).toBe('Simple Feature Plan');
      expect(doc.metadata).toEqual({
        scope: 'TEST',
        owner: '@test',
        priority: 'high',
      });
      expect(doc.tasks).toHaveLength(2);

      // First task with all fields
      expect(doc.tasks[0]).toMatchObject({
        id: 'TEST-001',
        title: 'First task',
        intent: 'This is a simple task to test parsing',
        expectedOutcome: 'Task should be parsed correctly',
        confidence: 'high',
        scopes: ['TEST'],
        tags: ['example', 'simple'],
        dependencies: ['TEST-000'],
        inputs: ['Input one', 'Input two'],
      });

      // Second task with minimal fields
      expect(doc.tasks[1]).toMatchObject({
        id: 'TEST-002',
        title: 'Second task',
        intent: 'Another task without all fields',
        confidence: 'medium',
        scopes: ['TEST', 'API'],
        tags: ['minimal'],
      });
    });

    it('should parse a minimal task with only required fields', async () => {
      const content = await loadFixture('minimal-task.aps.md');
      const doc = await parseDocument(content, 'minimal-task.aps.md');

      expect(doc.title).toBe('Minimal Task');
      expect(doc.tasks).toHaveLength(1);
      expect(doc.tasks[0]).toMatchObject({
        id: 'MIN-001',
        title: 'Minimal task with only required fields',
        intent: 'This task only has the required Intent field',
        confidence: 'medium', // default value
      });
    });

    it('should parse real example file (feature-auth.aps.md)', async () => {
      const content = await fs.readFile(
        join(__dirname, '../../examples/feature-auth.aps.md'),
        'utf-8'
      );
      const doc = await parseDocument(content, 'feature-auth.aps.md');

      expect(doc.title).toBe('Feature: User Authentication');
      expect(doc.metadata).toEqual({
        scope: 'AUTH',
        owner: '@alice',
        priority: 'high',
      });
      expect(doc.tasks).toHaveLength(8);

      // Spot check a few tasks
      expect(doc.tasks[0].id).toBe('AUTH-001');
      expect(doc.tasks[0].title).toBe('Create user database model');
      expect(doc.tasks[0].scopes).toEqual(['AUTH', 'DB']);

      expect(doc.tasks[7].id).toBe('AUTH-008');
      expect(doc.tasks[7].dependencies).toEqual(['AUTH-003', 'AUTH-005', 'AUTH-007']);
    });

    it('should parse hyphenated Non-scope field', async () => {
      const content = await loadFixture('non-scope-hyphenated.aps.md');
      const doc = await parseDocument(content, 'non-scope-hyphenated.aps.md');

      expect(doc.title).toBe('Test Non-scope Field');
      expect(doc.tasks).toHaveLength(1);
      expect(doc.tasks[0]).toMatchObject({
        id: 'TEST-001',
        title: 'Test hyphenated Non-scope field',
        intent: 'Test that Non-scope field with hyphen is parsed correctly',
        nonScope: ['database', 'external APIs', 'authentication'],
      });
    });
  });

  describe('error handling', () => {
    it('should throw ParseError for invalid task ID format', async () => {
      const content = await loadFixture('invalid-task-id.aps.md');

      await expect(parseDocument(content, 'invalid-task-id.aps.md')).rejects.toThrow(ParseError);
      await expect(parseDocument(content, 'invalid-task-id.aps.md')).rejects.toThrow(
        /Invalid task heading format/
      );
    });

    it('should throw ParseError for non-zero-padded task ID', async () => {
      const content = await loadFixture('invalid-task-id-not-padded.aps.md');

      await expect(parseDocument(content, 'invalid-task-id-not-padded.aps.md')).rejects.toThrow(
        ParseError
      );
      await expect(parseDocument(content, 'invalid-task-id-not-padded.aps.md')).rejects.toThrow(
        /Invalid task heading format/
      );
    });

    it('should throw ParseError for document without H1 title', async () => {
      const content = '## This is H2\n\nNo H1 title';

      await expect(parseDocument(content, 'no-title.md')).rejects.toThrow(ParseError);
      await expect(parseDocument(content, 'no-title.md')).rejects.toThrow(
        /Document must have an H1 title/
      );
    });

    it('should throw ParseError for task without Intent field', async () => {
      const content = `# Plan

## Tasks

### TEST-001: Task without intent

**Expected Outcome:** This should fail`;

      await expect(parseDocument(content, 'no-intent.md')).rejects.toThrow(ParseError);
      await expect(parseDocument(content, 'no-intent.md')).rejects.toThrow(
        /missing required field: Intent/
      );
    });
  });

  describe('confidence levels', () => {
    it('should parse all confidence levels correctly', async () => {
      const content = `# Confidence Test

## Tasks

### LOW-001: Low confidence task

**Intent:** Low confidence task
**Confidence:** low

### MED-001: Medium confidence task

**Intent:** Medium confidence task
**Confidence:** medium

### HIGH-001: High confidence task

**Intent:** High confidence task
**Confidence:** high
`;

      const doc = await parseDocument(content);

      expect(doc.tasks[0].confidence).toBe('low');
      expect(doc.tasks[1].confidence).toBe('medium');
      expect(doc.tasks[2].confidence).toBe('high');
    });

    it('should default to medium confidence when not specified', async () => {
      const content = `# Plan

## Tasks

### TEST-001: Task without confidence

**Intent:** No confidence specified`;

      const doc = await parseDocument(content);

      expect(doc.tasks[0].confidence).toBe('medium');
    });
  });

  describe('metadata parsing', () => {
    it('should parse module metadata from line after H1', async () => {
      const content = `# My Module

**Scope:** AUTH **Owner:** @alice **Priority:** high

## Tasks

### AUTH-001: Task

**Intent:** Do something`;

      const doc = await parseDocument(content);

      expect(doc.metadata).toEqual({
        scope: 'AUTH',
        owner: '@alice',
        priority: 'high',
      });
    });

    it('should handle missing optional metadata fields', async () => {
      const content = `# My Module

**Scope:** TEST

## Tasks

### TEST-001: Task

**Intent:** Do something`;

      const doc = await parseDocument(content);

      expect(doc.metadata).toEqual({
        scope: 'TEST',
      });
    });
  });

  describe('ID field parsing', () => {
    it('should parse ID field as alias for Scope', async () => {
      const content = `# My Module

**ID:** AUTH **Owner:** @alice

## Tasks

### AUTH-001: Task

**Intent:** Do something`;

      const doc = await parseDocument(content);

      expect(doc.metadata).toEqual({
        scope: 'AUTH',
        owner: '@alice',
      });
    });

    it('should parse Scope field (legacy) the same as ID', async () => {
      const content = `# My Module

**Scope:** AUTH **Owner:** @alice

## Tasks

### AUTH-001: Task

**Intent:** Do something`;

      const doc = await parseDocument(content);

      expect(doc.metadata).toEqual({
        scope: 'AUTH',
        owner: '@alice',
      });
    });
  });

  describe('status normalization', () => {
    it('should normalize legacy Draft to Proposed', async () => {
      const content = `# My Module

**Scope:** TEST **Status:** Draft

## Tasks

### TEST-001: Task

**Intent:** Do something`;

      const doc = await parseDocument(content);
      expect(doc.metadata?.status).toBe('Proposed');
    });

    it('should normalize legacy Complete to Done', async () => {
      const content = `# My Module

**Scope:** TEST **Status:** Complete

## Tasks

### TEST-001: Task

**Intent:** Do something`;

      const doc = await parseDocument(content);
      expect(doc.metadata?.status).toBe('Done');
    });

    it('should keep current spec values as-is', async () => {
      const content = `# My Module

**Scope:** TEST **Status:** Proposed

## Tasks

### TEST-001: Task

**Intent:** Do something`;

      const doc = await parseDocument(content);
      expect(doc.metadata?.status).toBe('Proposed');
    });

    it('should accept all valid status values', async () => {
      for (const [input, expected] of [
        ['Proposed', 'Proposed'],
        ['Ready', 'Ready'],
        ['In Progress', 'In Progress'],
        ['Done', 'Done'],
        ['Blocked', 'Blocked'],
      ] as const) {
        const content = `# Module

**Scope:** TEST **Status:** ${input}

## Tasks

### TEST-001: Task

**Intent:** Do something`;

        const doc = await parseDocument(content);
        expect(doc.metadata?.status).toBe(expected);
      }
    });
  });

  describe('packages parsing', () => {
    it('should parse comma-separated Packages field', async () => {
      const content = `# My Module

**Scope:** TEST **Packages:** @app/core, @app/utils

## Tasks

### TEST-001: Task

**Intent:** Do something`;

      const doc = await parseDocument(content);
      expect(doc.metadata?.packages).toEqual(['@app/core', '@app/utils']);
    });

    it('should handle Packages: (none) as empty array', async () => {
      const content = `# My Module

**Scope:** TEST **Packages:** (none)

## Tasks

### TEST-001: Task

**Intent:** Do something`;

      const doc = await parseDocument(content);
      expect(doc.metadata?.packages).toEqual([]);
    });

    it('should handle empty Packages value as empty array', async () => {
      const content = `# My Module

**Scope:** TEST **Packages:**

## Tasks

### TEST-001: Task

**Intent:** Do something`;

      const doc = await parseDocument(content);
      expect(doc.metadata?.packages).toEqual([]);
    });

    it('should filter empty entries from Packages', async () => {
      const content = `# My Module

**Scope:** TEST **Packages:** @app/core,, @app/utils

## Tasks

### TEST-001: Task

**Intent:** Do something`;

      const doc = await parseDocument(content);
      expect(doc.metadata?.packages).toEqual(['@app/core', '@app/utils']);
    });
  });

  describe('task packages parsing', () => {
    it('should parse Packages field on tasks', async () => {
      const content = `# Plan

## Tasks

### TEST-001: Task with packages

**Intent:** Do something
**Packages:** @app/core, @app/utils`;

      const doc = await parseDocument(content);
      expect(doc.tasks[0].packages).toEqual(['@app/core', '@app/utils']);
    });

    it('should handle Packages: (none) on tasks as empty array', async () => {
      const content = `# Plan

## Tasks

### TEST-001: Task with no packages

**Intent:** Do something
**Packages:** (none)`;

      const doc = await parseDocument(content);
      expect(doc.tasks[0].packages).toEqual([]);
    });
  });

  describe('source tracking', () => {
    it('should include source path in parsed document', async () => {
      const content = `# Plan

## Tasks

### TEST-001: Task

**Intent:** Test`;

      const doc = await parseDocument(content, '/path/to/plan.aps.md');

      expect(doc.sourcePath).toBe('/path/to/plan.aps.md');
    });

    it('should include source path and line number in tasks', async () => {
      const content = `# Plan

## Tasks

### TEST-001: First task

**Intent:** First

### TEST-002: Second task

**Intent:** Second`;

      const doc = await parseDocument(content, 'plan.aps.md');

      expect(doc.tasks[0].sourcePath).toBe('plan.aps.md');
      expect(doc.tasks[0].sourceLineNumber).toBeGreaterThan(0);

      expect(doc.tasks[1].sourcePath).toBe('plan.aps.md');
      expect(doc.tasks[1].sourceLineNumber).toBeGreaterThan(doc.tasks[0].sourceLineNumber!);
    });
  });

  describe('inputs parsing', () => {
    it('should parse inline Inputs text as single-item array', async () => {
      const content = `# Plan

## Tasks

### TEST-001: Task with inline inputs

**Intent:** Do something
**Inputs:** Database credentials required
`;

      const doc = await parseDocument(content);

      expect(doc.tasks[0].inputs).toEqual(['Database credentials required']);
    });

    it('should parse Inputs list', async () => {
      const content = `# Plan

## Tasks

### TEST-001: Task with input list

**Intent:** Do something
**Inputs:**

- First input
- Second input
`;

      const doc = await parseDocument(content);

      expect(doc.tasks[0].inputs).toEqual(['First input', 'Second input']);
    });

    it('should prefer list over inline text when both present', async () => {
      // This is an edge case - if there's inline text but also a list follows
      const content = `# Plan

## Tasks

### TEST-001: Task with both

**Intent:** Do something
**Inputs:** Inline text here

- List item one
- List item two
`;

      const doc = await parseDocument(content);

      // List should take precedence
      expect(doc.tasks[0].inputs).toEqual(['List item one', 'List item two']);
    });
  });

  describe('validation field parsing', () => {
    it('should parse validation command with inline code (backticks)', async () => {
      const content = `# Plan

## Tasks

### TEST-001: Task with validation command

**Intent:** Do something
**Validation:** \`pnpm test\`
`;

      const doc = await parseDocument(content);

      expect(doc.tasks[0].validation).toBe('pnpm test');
    });

    it('should parse validation command with plain text', async () => {
      const content = `# Plan

## Tasks

### TEST-001: Task with plain validation

**Intent:** Do something
**Validation:** npm run test
`;

      const doc = await parseDocument(content);

      expect(doc.tasks[0].validation).toBe('npm run test');
    });

    it('should parse Test field as alias for Validation', async () => {
      const content = `# Plan

## Tasks

### TEST-001: Task with Test field

**Intent:** Do something
**Test:** \`pnpm nx run test\`
`;

      const doc = await parseDocument(content);

      expect(doc.tasks[0].validation).toBe('pnpm nx run test');
    });

    it('should parse validation with mixed inline code and text', async () => {
      const content = `# Plan

## Tasks

### TEST-001: Task with mixed validation

**Intent:** Do something
**Validation:** Run \`pnpm test\` after changes
`;

      const doc = await parseDocument(content);

      expect(doc.tasks[0].validation).toBe('Run pnpm test after changes');
    });
  });
});
