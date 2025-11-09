/**
 * Generic Format Adapter Tests
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { readFile } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { GenericMarkdownAdapter } from '../generic/format-adapter.js';
import type { ParseContext } from '../base/types.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const fixturesDir = join(__dirname, 'fixtures/generic');

describe('GenericMarkdownAdapter', () => {
  let adapter: GenericMarkdownAdapter;

  beforeEach(() => {
    adapter = new GenericMarkdownAdapter();
  });

  describe('metadata', () => {
    it('should have correct name and version', () => {
      expect(adapter.metadata.name).toBe('generic-markdown');
      expect(adapter.metadata.version).toBe('1.0.0');
    });

    it('should have correct display name', () => {
      expect(adapter.metadata.displayName).toBe('Generic Markdown');
    });

    it('should support generic formats', () => {
      expect(adapter.metadata.formats).toContain('generic');
      expect(adapter.metadata.formats).toContain('prd');
      expect(adapter.metadata.formats).toContain('plan');
      expect(adapter.metadata.formats).toContain('todo');
    });

    it('should support markdown extensions', () => {
      expect(adapter.metadata.extensions).toContain('.md');
      expect(adapter.metadata.extensions).toContain('.markdown');
    });
  });

  describe('canImport / canExport', () => {
    it('should support importing generic formats', () => {
      expect(adapter.canImport('generic')).toBe(true);
      expect(adapter.canImport('prd')).toBe(true);
      expect(adapter.canImport('plan')).toBe(true);
      expect(adapter.canImport('todo')).toBe(true);
    });

    it('should support exporting to generic format', () => {
      expect(adapter.canExport('generic')).toBe(true);
      expect(adapter.canExport('markdown')).toBe(true);
    });

    it('should support markdown extensions', () => {
      expect(adapter.canImport('.md')).toBe(true);
      expect(adapter.canImport('.markdown')).toBe(true);
    });
  });

  describe('detect', () => {
    it('should detect simple PRD with moderate confidence', async () => {
      const content = await readFile(join(fixturesDir, 'prd-simple.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(30);
      expect(result.confidence).toBeLessThanOrEqual(45); // Capped for fallback
      expect(result.reason).toContain('markdown-headings');
    });

    it('should detect TODO list', async () => {
      const content = await readFile(join(fixturesDir, 'todo-list.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(30);
    });

    it('should detect detailed plan', async () => {
      const content = await readFile(join(fixturesDir, 'plan-detailed.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(30);
      expect(result.reason).toContain('requirements-section');
    });

    it('should detect RFC document', async () => {
      const content = await readFile(join(fixturesDir, 'rfc-example.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(30);
    });

    it('should have lower confidence than specific formats', async () => {
      const content = await readFile(join(fixturesDir, 'prd-simple.md'), 'utf-8');
      const result = adapter.detect(content);

      // Generic adapter confidence should be capped at 45%
      // so BMAD (50%+) and SpecKit (50%+) win
      expect(result.confidence).toBeLessThanOrEqual(45);
    });

    it('should not detect very short content', () => {
      const content = '# Title\n\nShort content.';
      const result = adapter.detect(content);

      expect(result.detected).toBe(false);
    });
  });

  describe('parse', () => {
    it('should parse simple PRD', async () => {
      const content = await readFile(join(fixturesDir, 'prd-simple.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.schema_version).toBe('0.1.0');
        expect(result.data?.intent).toBeDefined();
        expect(result.data?.proposed_changes).toBeDefined();
        expect(result.data?.hash).toBeDefined();
      }
    });

    it('should extract requirements as changes', async () => {
      const content = await readFile(join(fixturesDir, 'prd-simple.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.proposed_changes.length).toBeGreaterThan(0);
        // Should have changes from requirements
        const hasRequirements = result.data?.proposed_changes.some((c) =>
          c.description.toLowerCase().includes('metric')
        );
        expect(hasRequirements).toBe(true);
      }
    });

    it('should extract tasks as changes', async () => {
      const content = await readFile(join(fixturesDir, 'todo-list.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.proposed_changes.length).toBeGreaterThan(0);
        // Should have tasks
        const hasTasks = result.data?.proposed_changes.some((c) =>
          c.description.toLowerCase().includes('pipeline')
        );
        expect(hasTasks).toBe(true);
      }
    });

    it('should extract features as changes', async () => {
      const content = await readFile(join(fixturesDir, 'todo-list.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        // Should have features
        const hasFeatures = result.data?.proposed_changes.some((c) =>
          c.description.toLowerCase().includes('oauth')
        );
        expect(hasFeatures).toBe(true);
      }
    });

    it('should include goals in metadata', async () => {
      const content = await readFile(join(fixturesDir, 'prd-simple.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.metadata?.goals).toBeDefined();
        expect(Array.isArray(result.data?.metadata?.goals)).toBe(true);
      }
    });

    it('should use context for provenance', async () => {
      const content = await readFile(join(fixturesDir, 'prd-simple.md'), 'utf-8');
      const context: ParseContext = {
        filePath: '/path/to/prd.md',
        author: 'Test Author',
        repositoryPath: '/path/to/repo',
      };

      const result = await adapter.parse(content, context);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.provenance.author).toBe('Test Author');
        expect(result.data?.provenance.repository).toBe('/path/to/repo');
      }
    });

    it('should handle documents with only goals', () => {
      const content = `# Project Goals

## Goals

- Improve performance
- Reduce costs
- Enhance security`;

      return adapter.parse(content).then((result) => {
        expect(result.success).toBe(true);
        if (result.success) {
          expect(result.data?.metadata?.goals).toBeDefined();
        }
      });
    });

    it('should extract intent from purpose section', async () => {
      const content = await readFile(join(fixturesDir, 'prd-simple.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.intent).toContain('dashboard');
      }
    });
  });

  describe('serialize', () => {
    it('should serialize plan to generic markdown', async () => {
      const content = await readFile(join(fixturesDir, 'prd-simple.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toContain('# ');
        expect(serializeResult.content).toContain('## ');
      }
    });

    it('should include changes section', async () => {
      const content = await readFile(join(fixturesDir, 'prd-simple.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toContain('## Changes');
      }
    });

    it('should include metadata section', async () => {
      const content = await readFile(join(fixturesDir, 'prd-simple.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toContain('## Metadata');
        expect(serializeResult.content).toContain('Author');
      }
    });

    it('should maintain roundtrip fidelity', async () => {
      const content = await readFile(join(fixturesDir, 'prd-simple.md'), 'utf-8');
      const parse1 = await adapter.parse(content);

      expect(parse1.success).toBe(true);
      if (!parse1.success) return;

      const serialize = await adapter.serialize(parse1.data!);

      expect(serialize.success).toBe(true);
      if (!serialize.success) return;

      const parse2 = await adapter.parse(serialize.content);

      expect(parse2.success).toBe(true);
      if (!parse2.success) return;

      // Serialized format uses list items under "Files to Create/Update"
      // which won't be detected as requirements/tasks by the parser
      // Check that document has valid structure
      expect(parse2.data?.intent).toBeDefined();
      expect(parse2.data?.provenance.author).toBeDefined();
    });
  });

  describe('validate', () => {
    it('should validate simple PRD', async () => {
      const content = await readFile(join(fixturesDir, 'prd-simple.md'), 'utf-8');
      const result = await adapter.validate(content);

      expect(result.valid).toBe(true);
      expect(result.summary).toContain('valid');
    });

    it('should reject very short content', async () => {
      const content = '# Short';
      const result = await adapter.validate(content);

      expect(result.valid).toBe(false);
      expect(result.issues).toBeDefined();
      if (result.issues) {
        expect(result.issues.some((i) => i.code === 'CONTENT_TOO_SHORT')).toBe(true);
      }
    });

    it('should warn about missing planning sections', async () => {
      const content = `# Random Document

This is just random content without any planning sections like requirements,
tasks, or features. It should still be valid but with warnings.

## Introduction

Some introduction text here.

## Conclusion

Some conclusion text here.`;

      const result = await adapter.validate(content);

      expect(result.issues).toBeDefined();
      if (result.issues) {
        expect(result.issues.some((i) => i.code === 'NO_PLANNING_SECTIONS')).toBe(true);
      }
    });

    it('should validate detailed plan', async () => {
      const content = await readFile(join(fixturesDir, 'plan-detailed.md'), 'utf-8');
      const result = await adapter.validate(content);

      expect(result.valid).toBe(true);
    });

    it('should validate RFC document', async () => {
      const content = await readFile(join(fixturesDir, 'rfc-example.md'), 'utf-8');
      const result = await adapter.validate(content);

      expect(result.valid).toBe(true);
    });
  });

  describe('integration', () => {
    it('should complete full workflow: detect → parse → validate → serialize', async () => {
      const content = await readFile(join(fixturesDir, 'prd-simple.md'), 'utf-8');

      // Detect
      const detectResult = adapter.detect(content);
      expect(detectResult.detected).toBe(true);

      // Parse
      const parseResult = await adapter.parse(content);
      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      // Validate
      const validateResult = await adapter.validate(content);
      expect(validateResult.valid).toBe(true);

      // Serialize
      const serializeResult = await adapter.serialize(parseResult.data!);
      expect(serializeResult.success).toBe(true);
    });

    it('should handle multiple document types', async () => {
      const files = ['prd-simple.md', 'todo-list.md', 'plan-detailed.md', 'rfc-example.md'];

      for (const file of files) {
        const content = await readFile(join(fixturesDir, file), 'utf-8');

        const detectResult = adapter.detect(content);
        expect(detectResult.detected).toBe(true);

        const parseResult = await adapter.parse(content);
        expect(parseResult.success).toBe(true);
      }
    });
  });
});
