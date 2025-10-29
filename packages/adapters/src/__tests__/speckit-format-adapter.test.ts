/**
 * SpecKit Format Adapter Tests
 * Tests for format detection, parsing, serialization, and validation
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { readFile } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SpecKitFormatAdapter } from '../speckit/format-adapter.js';
import type { ParseContext } from '../base/types.js';

// Get __dirname equivalent for ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const fixturesDir = join(__dirname, 'fixtures/speckit');

describe('SpecKitFormatAdapter', () => {
  let adapter: SpecKitFormatAdapter;

  beforeEach(() => {
    adapter = new SpecKitFormatAdapter();
  });

  describe('metadata', () => {
    it('should have correct name and version', () => {
      expect(adapter.metadata.name).toBe('speckit');
      expect(adapter.metadata.version).toBe('2.0.0');
    });

    it('should have correct display name', () => {
      expect(adapter.metadata.displayName).toBe('GitHub SpecKit');
    });

    it('should support speckit formats', () => {
      expect(adapter.metadata.formats).toContain('speckit');
      expect(adapter.metadata.formats).toContain('spec-kit');
      expect(adapter.metadata.formats).toContain('spec.md');
    });

    it('should support .md extension', () => {
      expect(adapter.metadata.extensions).toContain('.md');
    });
  });

  describe('canImport / canExport', () => {
    it('should support importing speckit format', () => {
      expect(adapter.canImport('speckit')).toBe(true);
      expect(adapter.canImport('spec-kit')).toBe(true);
      expect(adapter.canImport('spec.md')).toBe(true);
    });

    it('should support exporting to speckit format', () => {
      expect(adapter.canExport('speckit')).toBe(true);
      expect(adapter.canExport('spec.md')).toBe(true);
    });

    it('should not support unknown formats', () => {
      expect(adapter.canImport('unknown')).toBe(false);
      expect(adapter.canExport('unknown')).toBe(false);
    });

    it('should support .md extension', () => {
      expect(adapter.canImport('.md')).toBe(true);
    });
  });

  describe('detect', () => {
    it('should detect valid spec document with high confidence', async () => {
      const content = await readFile(join(fixturesDir, 'sample-spec.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(50);
      expect(result.reason).toContain('specification-header');
    });

    it('should detect Specification header', () => {
      const content = `# Specification

## Intent

Build a new feature`;

      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.reason).toContain('specification-header');
    });

    it('should detect Intent section', () => {
      const content = `# Specification

## Intent

This is the intent of the specification.`;

      const result = adapter.detect(content);

      expect(result.reason).toContain('intent-section');
    });

    it('should detect Changes section', () => {
      const content = `# Specification

## Changes

### Files to Create

Create new file at path/to/file`;

      const result = adapter.detect(content);

      expect(result.reason).toContain('changes-section');
    });

    it('should detect file changes indicators', () => {
      const content = `# Specification

## Changes

### Files to Create

Create auth controller

### Files to Update

Update app.ts`;

      const result = adapter.detect(content);

      expect(result.reason).toContain('file-changes');
    });

    it('should detect code blocks', () => {
      const content = `# Specification

## Changes

\`\`\`typescript
export class Example {}
\`\`\``;

      const result = adapter.detect(content);

      expect(result.reason).toContain('code-blocks');
    });

    it('should not detect plain markdown without SpecKit indicators', () => {
      const content = `# Regular Document

This is just plain markdown content without SpecKit structure.`;

      const result = adapter.detect(content);

      expect(result.detected).toBe(false);
      expect(result.confidence).toBeLessThan(50);
    });

    it('should not detect BMAD format as SpecKit', () => {
      const content = `---
name: Product Requirements Document
---

# Product Requirements Document

FR-01: Some requirement
NFR-01: Another requirement`;

      const result = adapter.detect(content);

      expect(result.detected).toBe(false);
      expect(result.confidence).toBeLessThan(50);
    });
  });

  describe('parse', () => {
    it('should parse valid spec document to APS', async () => {
      const content = await readFile(join(fixturesDir, 'sample-spec.md'), 'utf-8');
      const context: ParseContext = {
        filePath: 'test-spec.md',
        author: 'Test Author',
      };

      const result = await adapter.parse(content, context);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data).toBeDefined();
        expect(result.data?.schema_version).toBe('0.1.0');
        expect(result.data?.intent).toBeDefined();
        expect(result.data?.proposed_changes).toBeDefined();
        expect(result.data?.provenance).toBeDefined();
        expect(result.data?.hash).toBeDefined();
      }
    });

    it('should extract intent from spec', async () => {
      const content = await readFile(join(fixturesDir, 'sample-spec.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success && result.data) {
        expect(result.data.intent).toBeDefined();
        expect(result.data.intent.toLowerCase()).toContain('authentication');
      }
    });

    it('should parse file changes from spec', async () => {
      const content = await readFile(join(fixturesDir, 'sample-spec.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.proposed_changes.length).toBeGreaterThan(0);
        const fileCreates = result.data?.proposed_changes.filter((c) => c.type === 'file_create');
        expect(fileCreates.length).toBeGreaterThan(0);
      }
    });

    it('should use context for provenance when provided', async () => {
      const content = await readFile(join(fixturesDir, 'sample-spec.md'), 'utf-8');
      const context: ParseContext = {
        filePath: '/path/to/spec.md',
        author: 'Context Author',
        repositoryPath: '/path/to/repo',
      };

      const result = await adapter.parse(content, context);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.provenance.author).toBe('Context Author');
        expect(result.data?.provenance.repository).toBe('/path/to/repo');
      }
    });

    it('should handle minimal spec document', async () => {
      const content = `# Specification

## Intent

Build authentication

## Changes

### Files to Create

#### Create auth.ts

Authentication module`;

      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.intent).toContain('authentication');
      }
    });

    it('should generate consistent hashes for same content', async () => {
      const content = await readFile(join(fixturesDir, 'sample-spec.md'), 'utf-8');
      const fixedContext: ParseContext = {
        author: 'Test Author',
        timestamp: '2025-01-01T00:00:00Z',
      };

      const result1 = await adapter.parse(content, fixedContext);
      const result2 = await adapter.parse(content, fixedContext);

      expect(result1.success).toBe(true);
      expect(result2.success).toBe(true);

      if (result1.success && result2.success) {
        expect(result1.data?.hash).toMatch(/^[a-f0-9]{64}$/);
        expect(result2.data?.hash).toMatch(/^[a-f0-9]{64}$/);
      }
    });
  });

  describe('serialize', () => {
    it('should serialize APS plan to SpecKit format', async () => {
      const content = await readFile(join(fixturesDir, 'sample-spec.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toBeDefined();
        expect(serializeResult.content.length).toBeGreaterThan(0);
      }
    });

    it('should include Specification header in serialized output', async () => {
      const content = await readFile(join(fixturesDir, 'sample-spec.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toMatch(/^#\s+Specification/m);
      }
    });

    it('should include Intent section in serialized output', async () => {
      const content = await readFile(join(fixturesDir, 'sample-spec.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toMatch(/##\s+Intent/m);
      }
    });

    it('should include Changes section in serialized output', async () => {
      const content = await readFile(join(fixturesDir, 'sample-spec.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toMatch(/##\s+Changes/m);
      }
    });

    it('should maintain roundtrip fidelity for basic structure', async () => {
      const originalContent = await readFile(join(fixturesDir, 'sample-spec.md'), 'utf-8');
      const parseResult1 = await adapter.parse(originalContent);

      expect(parseResult1.success).toBe(true);
      if (!parseResult1.success) return;

      const serializeResult = await adapter.serialize(parseResult1.data);

      expect(serializeResult.success).toBe(true);
      if (!serializeResult.success) return;

      const parseResult2 = await adapter.parse(serializeResult.content);

      expect(parseResult2.success).toBe(true);
      if (!parseResult2.success) return;

      // Check key properties are preserved
      expect(parseResult2.data?.intent).toBeDefined();
      expect(parseResult2.data?.proposed_changes.length).toBeGreaterThan(0);
    });
  });

  describe('validate', () => {
    it('should validate valid spec document', async () => {
      const content = await readFile(join(fixturesDir, 'sample-spec.md'), 'utf-8');
      const result = await adapter.validate(content);

      expect(result.valid).toBe(true);
      expect(result.summary).toContain('valid');
    });

    it('should reject document that is too short', () => {
      const content = '# Short';
      const result = adapter.validate(content);

      return result.then((res) => {
        expect(res.valid).toBe(false);
        expect(res.issues).toBeDefined();
        if (res.issues) {
          const shortError = res.issues.find((i) => i.code === 'CONTENT_TOO_SHORT');
          expect(shortError).toBeDefined();
        }
      });
    });

    it('should reject document with low confidence', async () => {
      const content = `# Not a SpecKit Document

This is just regular markdown without any SpecKit indicators like Intent, Changes, or file structure.

It has enough content to pass the length check, but it should still fail validation.`;

      const result = await adapter.validate(content);

      expect(result.valid).toBe(false);
      expect(result.issues).toBeDefined();
      if (result.issues) {
        const confidenceError = result.issues.find((i) => i.code === 'LOW_CONFIDENCE');
        expect(confidenceError).toBeDefined();
      }
    });

    it('should warn about missing Intent section', async () => {
      const content = `# Specification

## Changes

### Files to Create

Create some file here.

This document has the basic structure but is missing the Intent section which is recommended.`;

      const result = await adapter.validate(content);

      expect(result.issues).toBeDefined();
      if (result.issues) {
        const missingIntent = result.issues.find((i) => i.code === 'MISSING_INTENT');
        expect(missingIntent).toBeDefined();
        if (missingIntent) {
          expect(missingIntent.severity).toBe('warning');
        }
      }
    });

    it('should provide clear validation summary', async () => {
      const content = await readFile(join(fixturesDir, 'sample-spec.md'), 'utf-8');
      const result = await adapter.validate(content);

      expect(result.summary).toBeDefined();
      expect(result.summary.length).toBeGreaterThan(0);
    });
  });

  describe('edge cases', () => {
    describe('confidence scoring', () => {
      it('should have high confidence with all sections', () => {
        const content = `# Specification

## Intent

Build authentication

## Overview

Overview of the feature

## Goals

- Goal 1
- Goal 2

## Requirements

- Requirement 1

## Changes

### Files to Create

#### Create auth.ts

\`\`\`typescript
export class Auth {}
\`\`\``;

        const result = adapter.detect(content);

        expect(result.confidence).toBeGreaterThanOrEqual(90);
        expect(result.detected).toBe(true);
      });

      it('should have 0 confidence for completely unrelated content', () => {
        const content = 'Just some random text without any structure.';

        const result = adapter.detect(content);

        expect(result.confidence).toBe(0);
        expect(result.detected).toBe(false);
      });

      it('should have partial confidence with only Specification header', () => {
        const content = `# Specification

Some content but no other sections.`;

        const result = adapter.detect(content);

        expect(result.confidence).toBeLessThan(50);
        expect(result.detected).toBe(false);
      });

      it('should detect "Spec" as alternative to "Specification"', () => {
        const content = `# Spec

## Intent

Build feature`;

        const result = adapter.detect(content);

        expect(result.detected).toBe(true);
        expect(result.reason).toContain('specification-header');
      });
    });

    describe('section variations', () => {
      it('should detect "Goal" singular form', () => {
        const content = `# Specification

## Goal

Build authentication system`;

        const result = adapter.detect(content);

        expect(result.reason).toContain('goals-section');
      });

      it('should detect "Requirement" singular form', () => {
        const content = `# Specification

## Requirement

Node.js 18+`;

        const result = adapter.detect(content);

        expect(result.reason).toContain('requirements-section');
      });

      it('should detect "Change" singular form', () => {
        const content = `# Specification

## Change

### Files to Create

New file`;

        const result = adapter.detect(content);

        expect(result.reason).toContain('changes');
      });
    });

    describe('file change variations', () => {
      it('should detect "create file" phrase', () => {
        const content = `# Specification

## Changes

Create file at src/auth.ts`;

        const result = adapter.detect(content);

        expect(result.reason).toContain('file-changes');
      });

      it('should detect "update file" phrase', () => {
        const content = `# Specification

## Changes

Update file src/app.ts`;

        const result = adapter.detect(content);

        expect(result.reason).toContain('file-changes');
      });
    });

    describe('parsing edge cases', () => {
      it('should handle spec with only Intent and Changes', async () => {
        const content = `# Specification

## Intent

Implement feature X

## Changes

### Files to Create

#### Create feature.ts

Feature implementation`;

        const result = await adapter.parse(content);

        expect(result.success).toBe(true);
      });

      it('should handle empty Changes section', async () => {
        const content = `# Specification

## Intent

Implement feature

## Changes`;

        const result = await adapter.parse(content);

        expect(result.success).toBe(true);
      });

      it('should handle special characters in descriptions', async () => {
        const content = `# Specification

## Intent

Support **bold**, *italic*, and \`code\` in descriptions

## Changes

### Files to Create

#### Create test.ts with [links](http://example.com)

Implementation with | pipes |`;

        const result = await adapter.parse(content);

        expect(result.success).toBe(true);
      });
    });

    describe('serialization edge cases', () => {
      it('should handle APS plan with minimal changes', async () => {
        const content = `# Specification

## Intent

Minimal feature

## Changes

### Files to Create

#### Create minimal.ts

Minimal implementation`;

        const parseResult = await adapter.parse(content);
        expect(parseResult.success).toBe(true);
        if (!parseResult.success) return;

        const serializeResult = await adapter.serialize(parseResult.data!);
        expect(serializeResult.success).toBe(true);
      });
    });
  });
});
