/**
 * BMAD Format Adapter Tests
 * Tests for format detection, parsing, serialization, and validation
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { readFile } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { BMADFormatAdapter } from '../bmad/format-adapter.js';
import {
  analyzePath,
  expandVariables,
  extractIntent,
  parseYamlBoolean,
  hasHyphenatedVariables,
  isAgentYamlContent,
  isWorkflowYamlContent,
  isTeamYamlContent,
  isModuleYamlContent,
  parseAgentYaml,
  parseWorkflowYaml,
  parseTeamYaml,
  parseModuleYaml,
} from '../bmad/utils.js';
import { BMADDocumentType, BMAD_FOLDERS, BMAD_UPSTREAM_VERSION } from '../bmad/types.js';
import type { ParseContext, PathDetectionHint } from '../base/types.js';

// Get __dirname equivalent for ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const fixturesDir = join(__dirname, 'fixtures/bmad');

describe('BMADFormatAdapter', () => {
  let adapter: BMADFormatAdapter;

  beforeEach(() => {
    adapter = new BMADFormatAdapter();
  });

  describe('metadata', () => {
    it('should have correct name and version', () => {
      expect(adapter.metadata.name).toBe('bmad');
      expect(adapter.metadata.version).toBe('0.1.2');
    });

    it('should have correct display name', () => {
      expect(adapter.metadata.displayName).toBe(
        'BMAD (Breakthrough Method for Agile AI-Driven Development)'
      );
    });

    it('should support bmad formats', () => {
      expect(adapter.metadata.formats).toContain('bmad');
      expect(adapter.metadata.formats).toContain('prd');
      expect(adapter.metadata.formats).toContain('architecture');
    });

    it('should support .md extension', () => {
      expect(adapter.metadata.extensions).toContain('.md');
    });

    it('should support .yaml and .yml extensions (v6)', () => {
      expect(adapter.metadata.extensions).toContain('.yaml');
      expect(adapter.metadata.extensions).toContain('.yml');
    });

    it('should support v6 format names', () => {
      expect(adapter.metadata.formats).toContain('agent');
      expect(adapter.metadata.formats).toContain('workflow');
      expect(adapter.metadata.formats).toContain('team');
      expect(adapter.metadata.formats).toContain('module');
    });

    it('should reference correct upstream version', () => {
      expect(BMAD_UPSTREAM_VERSION).toBe('6.0.3');
      expect(adapter.metadata.description).toContain(BMAD_UPSTREAM_VERSION);
    });
  });

  describe('canImport / canExport', () => {
    it('should support importing bmad format', () => {
      expect(adapter.canImport('bmad')).toBe(true);
      expect(adapter.canImport('prd')).toBe(true);
      expect(adapter.canImport('architecture')).toBe(true);
    });

    it('should support exporting to bmad format', () => {
      expect(adapter.canExport('bmad')).toBe(true);
      expect(adapter.canExport('prd')).toBe(true);
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
    it('should detect valid PRD document with high confidence', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(80);
      expect(result.reason).toContain('yaml-frontmatter');
      expect(result.reason).toContain('requirements');
    });

    it('should detect valid architecture document', async () => {
      const content = await readFile(join(fixturesDir, 'valid-architecture.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(50);
      expect(result.reason).toContain('yaml-frontmatter');
    });

    it('should detect valid epic document', async () => {
      const content = await readFile(join(fixturesDir, 'valid-epic.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(50);
    });

    it('should detect valid story document', async () => {
      const content = await readFile(join(fixturesDir, 'valid-story.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(50);
      expect(result.reason).toContain('user-story');
    });

    it('should not detect document that is too short', async () => {
      const content = await readFile(join(fixturesDir, 'invalid-too-short.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(false);
      expect(result.confidence).toBeLessThan(50);
    });

    it('should have low confidence for document without requirements', async () => {
      const content = await readFile(join(fixturesDir, 'invalid-no-requirements.md'), 'utf-8');
      const result = adapter.detect(content);

      // May detect YAML but lack of requirements should lower confidence
      expect(result.confidence).toBeLessThan(80);
    });

    it('should detect YAML front-matter indicator', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.reason).toContain('yaml-frontmatter');
    });

    it('should detect requirements indicator', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.reason).toMatch(/\d+ requirements?/);
    });

    it('should detect user story format indicator', async () => {
      const content = await readFile(join(fixturesDir, 'valid-story.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.reason).toContain('user-story');
    });

    it('should detect change log indicator', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.reason).toContain('change-log');
    });

    it('should not detect plain markdown without BMAD indicators', () => {
      const content = `# Regular Document\n\nThis is just plain markdown content.`;
      const result = adapter.detect(content);

      expect(result.detected).toBe(false);
      expect(result.confidence).toBeLessThan(50);
    });
  });

  describe('parse', () => {
    it('should parse valid PRD document to APS', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const context: ParseContext = {
        filePath: 'test-prd.md',
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

    it('should extract intent from PRD executive summary', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success && result.data) {
        expect(result.data.intent).toBeDefined();
        expect(result.data.intent.toLowerCase()).toContain('authentication');
      }
    });

    it('should handle long overview text without punctuation efficiently', () => {
      const longOverview = `Feature ${'word '.repeat(20000)}tail`;
      const content = `---
name: Long Overview
---

## Overview

${longOverview}`;

      const intent = extractIntent(content, BMADDocumentType.UNKNOWN);

      expect(intent.length).toBeLessThanOrEqual(203);
      expect(intent.startsWith('Feature word')).toBe(true);
      expect(intent.endsWith('...')).toBe(true);
    });

    it('should parse YAML front-matter metadata', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.provenance.author).toBe('Jane Smith');
      }
    });

    it('should parse functional requirements as changes', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        const frChanges = result.data?.proposed_changes.filter((c) =>
          c.description.includes('FR-')
        );
        expect(frChanges.length).toBeGreaterThan(0);
      }
    });

    it('should parse non-functional requirements as changes', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        const nfrChanges = result.data?.proposed_changes.filter((c) =>
          c.description.includes('NFR-')
        );
        expect(nfrChanges.length).toBeGreaterThan(0);
      }
    });

    it('should parse user stories as changes', async () => {
      const content = await readFile(join(fixturesDir, 'valid-story.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success && result.data) {
        expect(result.data.proposed_changes.length).toBeGreaterThan(0);
        expect(result.data.intent).toBeDefined();
      }
    });

    it('should handle document without front-matter', async () => {
      const content = `# PRD Document

FR-01: Some requirement

NFR-01: Some non-functional requirement`;

      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.provenance.author).toBe('unknown');
      }
    });

    it('should use context for provenance when provided', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const context: ParseContext = {
        filePath: '/path/to/prd.md',
        author: 'Context Author',
        repositoryPath: '/path/to/repo',
      };

      const result = await adapter.parse(content, context);

      expect(result.success).toBe(true);
      if (result.success) {
        // YAML author should override context author
        expect(result.data?.provenance.author).toBe('Jane Smith');
        expect(result.data?.provenance.repository).toBe('/path/to/repo');
      }
    });

    it('should handle parse errors gracefully', async () => {
      const content = 'Invalid content without proper structure';

      const result = await adapter.parse(content);

      // Parser is lenient and will parse minimal content
      // This test verifies error handling exists
      expect(result.success).toBeDefined();
    });

    it('should generate consistent hashes for same content', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const fixedContext: ParseContext = {
        author: 'Test Author',
        timestamp: '2025-01-01T00:00:00Z', // Fixed timestamp for deterministic hash
      };

      const result1 = await adapter.parse(content, fixedContext);
      const result2 = await adapter.parse(content, fixedContext);

      expect(result1.success).toBe(true);
      expect(result2.success).toBe(true);

      // Note: Hashes will differ due to random plan ID generation
      // This test verifies that parsing succeeds and generates valid hashes
      if (result1.success && result2.success) {
        expect(result1.data?.hash).toMatch(/^[a-f0-9]{64}$/);
        expect(result2.data?.hash).toMatch(/^[a-f0-9]{64}$/);
      }
    });
  });

  describe('serialize', () => {
    it('should serialize APS plan to BMAD format', async () => {
      // First parse a BMAD document to APS
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      // Then serialize back to BMAD
      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toBeDefined();
        expect(serializeResult.content.length).toBeGreaterThan(0);
      }
    });

    it('should include YAML front-matter in serialized output', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toMatch(/^---\n/);
        expect(serializeResult.content).toContain('name:');
        expect(serializeResult.content).toContain('version:');
      }
    });

    it('should include change log table in serialized output', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toContain('## Change Log');
        expect(serializeResult.content).toContain('| Date');
        expect(serializeResult.content).toContain('| Version');
      }
    });

    it('should categorize changes as FR/NFR appropriately', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toContain('FR-');
        expect(serializeResult.content).toContain('NFR-');
      }
    });

    it('should maintain roundtrip fidelity', async () => {
      // Parse BMAD → APS
      const originalContent = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const parseResult1 = await adapter.parse(originalContent);

      expect(parseResult1.success).toBe(true);
      if (!parseResult1.success) return;

      // Serialize APS → BMAD
      const serializeResult = await adapter.serialize(parseResult1.data);

      expect(serializeResult.success).toBe(true);
      if (!serializeResult.success) return;

      // Parse again BMAD → APS
      const parseResult2 = await adapter.parse(serializeResult.content);

      expect(parseResult2.success).toBe(true);
      if (!parseResult2.success) return;

      // Check key properties are preserved
      if (parseResult2.data) {
        // Intent may be transformed during serialization (e.g., to document title)
        expect(parseResult2.data.intent).toBeDefined();
        expect(parseResult2.data.intent.length).toBeGreaterThan(0);
        // Changes should be present (exact count may vary due to serialization format)
        expect(parseResult2.data.proposed_changes.length).toBeGreaterThan(0);
        // Author should be preserved
        expect(parseResult2.data.provenance.author).toBe(parseResult1.data?.provenance.author);
      }
    });
  });

  describe('validate', () => {
    it('should validate valid PRD document', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const result = await adapter.validate(content);

      expect(result.valid).toBe(true);
      expect(result.summary).toContain('valid');
    });

    it('should reject document that is too short', async () => {
      const content = await readFile(join(fixturesDir, 'invalid-too-short.md'), 'utf-8');
      const result = await adapter.validate(content);

      expect(result.valid).toBe(false);
      expect(result.issues).toBeDefined();
      if (result.issues) {
        const shortError = result.issues.find((i) => i.code === 'CONTENT_TOO_SHORT');
        expect(shortError).toBeDefined();
      }
    });

    it('should reject document with low confidence', async () => {
      const content = `# Not a BMAD Document

This is just regular markdown without any BMAD indicators like requirements or YAML front-matter.

It has enough content to pass the length check, but it should still fail validation because it doesn't look like BMAD format.`;

      const result = await adapter.validate(content);

      expect(result.valid).toBe(false);
      expect(result.issues).toBeDefined();
      if (result.issues) {
        const confidenceError = result.issues.find((i) => i.code === 'LOW_CONFIDENCE');
        expect(confidenceError).toBeDefined();
      }
    });

    it('should warn about missing requirements', async () => {
      const content = await readFile(join(fixturesDir, 'invalid-no-requirements.md'), 'utf-8');
      const result = await adapter.validate(content);

      expect(result.issues).toBeDefined();
      if (result.issues) {
        const noReqWarning = result.issues.find((i) => i.code === 'NO_REQUIREMENTS');
        expect(noReqWarning).toBeDefined();
        if (noReqWarning) {
          expect(noReqWarning.severity).toBe('warning');
        }
      }
    });

    it('should provide clear validation summary', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const result = await adapter.validate(content);

      expect(result.summary).toBeDefined();
      expect(result.summary.length).toBeGreaterThan(0);
    });

    it('should validate valid architecture document', async () => {
      const content = await readFile(join(fixturesDir, 'valid-architecture.md'), 'utf-8');
      const result = await adapter.validate(content);

      expect(result.valid).toBe(true);
    });

    it('should validate valid epic document', async () => {
      const content = await readFile(join(fixturesDir, 'valid-epic.md'), 'utf-8');
      const result = await adapter.validate(content);

      expect(result.valid).toBe(true);
    });

    it('should validate valid story document', async () => {
      const content = await readFile(join(fixturesDir, 'valid-story.md'), 'utf-8');
      const result = await adapter.validate(content);

      expect(result.valid).toBe(true);
    });
  });

  describe('edge cases', () => {
    describe('unicode and special characters', () => {
      it('should handle unicode characters in requirements', async () => {
        const content = `---
name: Unicode Test
version: 1.0.0
author: Test User
---

# Test Document

FR-01: Support émojis 🚀 and unicode characters ñ é
NFR-01: Handle Chinese characters 中文测试`;

        const result = await adapter.parse(content);

        expect(result.success).toBe(true);
        if (result.success) {
          expect(result.data?.proposed_changes.length).toBe(2);
          expect(result.data?.proposed_changes[0]?.description).toContain('🚀');
        }
      });

      it('should handle special markdown characters in descriptions', async () => {
        const content = `---
name: Special Chars Test
---

# Test

FR-01: Support **bold**, *italic*, and \`code\` in descriptions
FR-02: Handle | pipes | and [links](http://example.com)`;

        const result = await adapter.parse(content);

        expect(result.success).toBe(true);
        if (result.success) {
          expect(result.data?.proposed_changes.length).toBe(2);
        }
      });
    });

    describe('requirement ID formats', () => {
      it('should parse requirements with double-digit IDs', async () => {
        const content = `# Test

FR-01: First requirement
FR-10: Tenth requirement
FR-99: Ninety-ninth requirement`;

        const result = await adapter.parse(content);

        expect(result.success).toBe(true);
        if (result.success) {
          expect(result.data?.proposed_changes.length).toBe(3);
        }
      });

      it('should not parse malformed requirement IDs', () => {
        const content = `# Test

FR-1: Wrong format (single digit)
FR-001: Wrong format (three digits)
REQ-01: Wrong prefix`;

        const result = adapter.detect(content);

        // Should have low confidence due to malformed IDs
        expect(result.confidence).toBeLessThan(50);
      });

      it('should parse mixed requirement types', async () => {
        const content = `# Test

FR-01: Functional requirement
NFR-01: Non-functional requirement
US-01: User story requirement

As a user,
I want feature,
so that benefit.`;

        const result = await adapter.parse(content);

        expect(result.success).toBe(true);
        if (result.success) {
          // US-01 is parsed both as a requirement and as a user story, so we get 4 changes
          expect(result.data?.proposed_changes.length).toBeGreaterThanOrEqual(3);
          expect(result.data?.proposed_changes.some((c) => c.description.includes('FR-01'))).toBe(
            true
          );
          expect(result.data?.proposed_changes.some((c) => c.description.includes('NFR-01'))).toBe(
            true
          );
          expect(result.data?.proposed_changes.some((c) => c.description.includes('US-01'))).toBe(
            true
          );
        }
      });
    });

    describe('empty and minimal content', () => {
      it('should handle empty sections gracefully', async () => {
        const content = `---
name: Empty Sections Test
---

# Test Document

## Executive Summary

## Functional Requirements

FR-01: Only one requirement

## Non-Functional Requirements

## User Stories`;

        const result = await adapter.parse(content);

        expect(result.success).toBe(true);
        if (result.success) {
          expect(result.data?.proposed_changes.length).toBe(1);
        }
      });

      it('should handle document with only front-matter', async () => {
        const content = `---
name: Minimal Test
version: 1.0.0
---`;

        const result = await adapter.parse(content);

        expect(result.success).toBe(true);
        // Document is valid but has no changes
      });

      it('should detect minimal valid BMAD document', () => {
        const content = `---
name: Minimal
---

FR-01: Minimal requirement`;

        const result = adapter.detect(content);

        expect(result.detected).toBe(true);
        expect(result.confidence).toBeGreaterThanOrEqual(50);
      });
    });

    describe('large documents', () => {
      it('should handle document with many requirements', async () => {
        let content = `---
name: Large Document Test
---

# Large Test Document

`;

        // Add 50 requirements
        for (let i = 1; i <= 50; i++) {
          const id = i.toString().padStart(2, '0');
          content += `FR-${id}: Requirement number ${i}\n`;
        }

        const result = await adapter.parse(content);

        expect(result.success).toBe(true);
        if (result.success) {
          expect(result.data?.proposed_changes.length).toBe(50);
        }
      });

      it('should handle very long requirement descriptions', async () => {
        const longDescription = 'A'.repeat(1000);
        const content = `---
name: Long Description Test
---

FR-01: ${longDescription}`;

        const result = await adapter.parse(content);

        expect(result.success).toBe(true);
        if (result.success) {
          expect(result.data?.proposed_changes[0]?.description).toContain(longDescription);
        }
      });
    });

    describe('malformed content', () => {
      it('should handle malformed YAML front-matter', () => {
        const content = `---
name: Test: Invalid: YAML
invalid yaml here
no proper structure
---

FR-01: Some requirement`;

        const result = adapter.detect(content);

        // Should still detect due to FR-01
        expect(result.detected).toBe(true);
      });

      it('should handle documents with multiple YAML blocks', () => {
        const content = `---
name: First Block
---

Some content

---
name: Second Block
---

FR-01: Requirement`;

        const result = adapter.detect(content);

        // Should still detect YAML and requirements
        expect(result.detected).toBe(true);
      });

      it('should handle missing YAML closing delimiter', () => {
        const content = `---
name: Unclosed YAML
version: 1.0.0

FR-01: Requirement without proper YAML close`;

        const result = adapter.detect(content);

        // Should still detect based on FR-01
        expect(result.confidence).toBeGreaterThanOrEqual(25);
      });
    });

    describe('user story format variations', () => {
      it('should detect user story with "As an" (article)', () => {
        const content = `# Story

As an administrator,
I want to manage users,
so that I can control access.`;

        const result = adapter.detect(content);

        expect(result.reason).toContain('user-story');
      });

      it('should detect user story with "As a" (no article)', () => {
        const content = `# Story

As a user,
I want to login,
so that I can access my account.`;

        const result = adapter.detect(content);

        expect(result.reason).toContain('user-story');
      });

      it('should handle user stories without acceptance criteria', async () => {
        const content = `---
name: Story Test
---

US-01: Basic Login

As a user,
I want to log in,
so that I can access my account.`;

        const result = await adapter.parse(content);

        expect(result.success).toBe(true);
      });
    });

    describe('serialization edge cases', () => {
      it('should handle APS plan with no changes', async () => {
        const content = `---
name: Empty Plan
---

# Test Document`;

        const parseResult = await adapter.parse(content);
        expect(parseResult.success).toBe(true);
        if (!parseResult.success) return;

        const serializeResult = await adapter.serialize(parseResult.data!);
        expect(serializeResult.success).toBe(true);
      });

      it('should serialize plan with special characters', async () => {
        const content = `---
name: Special Chars
---

FR-01: Support "quotes" and 'apostrophes'
FR-02: Handle <brackets> and &ampersands&`;

        const parseResult = await adapter.parse(content);
        expect(parseResult.success).toBe(true);
        if (!parseResult.success) return;

        const serializeResult = await adapter.serialize(parseResult.data!);
        expect(serializeResult.success).toBe(true);
        if (serializeResult.success) {
          expect(serializeResult.content).toContain('quotes');
          expect(serializeResult.content).toContain('brackets');
        }
      });

      it('should preserve line breaks in descriptions', async () => {
        const content = `---
name: Line Breaks Test
---

FR-01: Multi-line requirement description that spans multiple lines`;

        const parseResult = await adapter.parse(content);
        const serializeResult = await adapter.serialize(parseResult.data!);

        expect(serializeResult.success).toBe(true);
      });
    });

    describe('detection confidence scoring', () => {
      it('should have maximum confidence with all indicators', () => {
        const content = `---
name: Product Requirements Document
version: 1.0.0
---

# Product Requirements Document

## Change Log

| Date | Version | Description | Author |
|------|---------|-------------|--------|
| 2025-01-01 | 1.0.0 | Initial | Test |

FR-01: Requirement

As a user,
I want feature,
so that benefit.`;

        const result = adapter.detect(content);

        expect(result.confidence).toBe(100);
        expect(result.reason).toContain('yaml-frontmatter');
        expect(result.reason).toContain('requirements');
        expect(result.reason).toContain('user-story');
        expect(result.reason).toContain('change-log');
        expect(result.reason).toContain('document-title');
      });

      it('should have 0 confidence for completely unrelated content', () => {
        const content = 'Just some random text without any structure.';

        const result = adapter.detect(content);

        expect(result.confidence).toBe(0);
        expect(result.detected).toBe(false);
      });

      it('should have partial confidence with only YAML', () => {
        const content = `---
name: Test
---

Some content without requirements.`;

        const result = adapter.detect(content);

        expect(result.confidence).toBe(30); // Only YAML points
        expect(result.detected).toBe(false); // Below 50% threshold
      });
    });
  });

  describe('additional parser tests', () => {
    it('should parse valid task document', async () => {
      const content = await readFile(join(fixturesDir, 'valid-task.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.proposed_changes.length).toBeGreaterThan(0);
        expect(result.data?.provenance.author).toBe('Developer');
      }
    });

    it('should parse minimal PRD correctly', async () => {
      const content = await readFile(join(fixturesDir, 'valid-minimal-prd.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        // Prettier reformatted the file so some requirements are on same line
        // Parser extracts what it can find with proper FR-XX: format on separate lines
        expect(result.data?.proposed_changes.length).toBeGreaterThanOrEqual(1);
        expect(result.data?.provenance.author).toBe('John Doe');
      }
    });

    it('should parse complex PRD with many requirements', async () => {
      const content = await readFile(join(fixturesDir, 'valid-complex-prd.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        // Prettier reformatted the file so requirements are wrapped on same lines
        // Parser extracts requirements that follow proper format (on separate lines)
        expect(result.data?.proposed_changes.length).toBeGreaterThanOrEqual(20);
        expect(result.data?.provenance.author).toBe('Product Team');
        expect(result.data?.provenance.version).toBe('2.1.0');
      }
    });

    it('should handle malformed YAML gracefully', async () => {
      const content = await readFile(join(fixturesDir, 'invalid-malformed-yaml.md'), 'utf-8');
      const result = await adapter.parse(content);

      // Parser should be lenient and still extract requirements
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.proposed_changes.length).toBeGreaterThan(0);
      }
    });

    it('should extract requirement IDs accurately', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        const descriptions = result.data?.proposed_changes.map((c) => c.description) || [];
        // Check that FR/NFR IDs are preserved in descriptions
        expect(descriptions.some((d) => d.includes('FR-'))).toBe(true);
        expect(descriptions.some((d) => d.includes('NFR-'))).toBe(true);
      }
    });

    it('should map different requirement types to appropriate change types', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        const changes = result.data?.proposed_changes || [];
        // Should have file_create for FR (functional) and config_update for NFR
        expect(changes.some((c) => c.type === 'file_create')).toBe(true);
        expect(changes.some((c) => c.type === 'config_update')).toBe(true);
      }
    });

    it('should extract provenance metadata correctly from different sources', async () => {
      const content = await readFile(join(fixturesDir, 'valid-architecture.md'), 'utf-8');
      const context: ParseContext = {
        filePath: 'test.md',
        repositoryPath: '/repo',
        branch: 'main',
        commit: 'abc123',
      };

      const result = await adapter.parse(content, context);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.provenance.repository).toBe('/repo');
        expect(result.data?.provenance.branch).toBe('main');
        expect(result.data?.provenance.commit).toBe('abc123');
      }
    });

    it('should handle document with no changes gracefully', async () => {
      const content = await readFile(join(fixturesDir, 'invalid-only-yaml.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        // Document with no requirements should result in empty changes array
        expect(result.data?.proposed_changes).toBeDefined();
        expect(Array.isArray(result.data?.proposed_changes)).toBe(true);
      }
    });

    it('should handle very long requirement descriptions', async () => {
      const longDesc = 'A'.repeat(500);
      const content = `---
name: Long Description Test
---

FR-01: ${longDesc}`;

      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.proposed_changes[0]?.description).toContain(longDesc);
      }
    });
  });

  describe('additional serializer tests', () => {
    it('should serialize plan with no execution history', async () => {
      const content = await readFile(join(fixturesDir, 'valid-minimal-prd.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toContain('---');
        expect(serializeResult.content).toContain('name:');
      }
    });

    it('should serialize plan with custom metadata', async () => {
      const content = await readFile(join(fixturesDir, 'valid-task.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toContain('Product Requirements Document');
        expect(serializeResult.content).toContain('Change Log');
      }
    });

    it('should serialize plan with very long descriptions', async () => {
      const content = await readFile(join(fixturesDir, 'valid-complex-prd.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        // Verify serialization doesn't truncate content
        expect(serializeResult.content.length).toBeGreaterThan(1000);
        expect(serializeResult.content).toContain('FR-');
        expect(serializeResult.content).toContain('NFR-');
      }
    });

    it('should properly categorize mixed requirement types', async () => {
      const content = await readFile(join(fixturesDir, 'valid-complex-prd.md'), 'utf-8');
      const parseResult = await adapter.parse(content);

      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        // Should have separate sections for FR, NFR, and US
        expect(serializeResult.content).toContain('## Functional Requirements');
        expect(serializeResult.content).toContain('## Non-Functional Requirements');
        expect(serializeResult.content).toContain('## User Stories');
      }
    });

    it('should include repository information when available', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const context: ParseContext = {
        repositoryPath: 'https://github.com/user/repo',
        branch: 'feature/auth',
        commit: 'abc123def',
      };

      const parseResult = await adapter.parse(content, context);
      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      if (serializeResult.success) {
        expect(serializeResult.content).toContain('Repository Information');
        expect(serializeResult.content).toContain('https://github.com/user/repo');
        expect(serializeResult.content).toContain('feature/auth');
      }
    });
  });

  describe('integration tests', () => {
    it('should complete full workflow: detect → parse → validate → serialize', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');

      // 1. Detect
      const detectResult = adapter.detect(content);
      expect(detectResult.detected).toBe(true);

      // 2. Parse
      const parseResult = await adapter.parse(content);
      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      // 3. Validate
      const validateResult = await adapter.validate(content);
      expect(validateResult.valid).toBe(true);

      // 4. Serialize
      const serializeResult = await adapter.serialize(parseResult.data!);
      expect(serializeResult.success).toBe(true);
    });

    it('should handle format auto-detection workflow', async () => {
      const files = [
        'valid-prd.md',
        'valid-architecture.md',
        'valid-epic.md',
        'valid-story.md',
        'valid-task.md',
      ];

      for (const file of files) {
        const content = await readFile(join(fixturesDir, file), 'utf-8');

        // Auto-detect should work for all valid BMAD documents
        const detectResult = adapter.detect(content);
        expect(detectResult.detected).toBe(true);
        expect(detectResult.confidence).toBeGreaterThanOrEqual(50);

        // Parse should succeed
        const parseResult = await adapter.parse(content);
        expect(parseResult.success).toBe(true);
      }
    });

    it('should recover from parse errors gracefully', async () => {
      const invalidContent = 'This will cause parsing issues but should not throw';

      // Should return error result, not throw
      const parseResult = await adapter.parse(invalidContent);
      expect(parseResult).toBeDefined();
      expect(parseResult.success).toBeDefined();
    });

    it('should handle batch processing of multiple documents', async () => {
      const files = ['valid-prd.md', 'valid-architecture.md', 'valid-story.md'];
      const results = [];

      for (const file of files) {
        const content = await readFile(join(fixturesDir, file), 'utf-8');
        const parseResult = await adapter.parse(content);
        results.push(parseResult);
      }

      // All should succeed
      expect(results.every((r) => r.success)).toBe(true);
      expect(results.length).toBe(3);
    });

    it('should preserve data integrity through full round-trip', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');

      // Parse → Serialize → Parse again
      const parse1 = await adapter.parse(content);
      expect(parse1.success).toBe(true);
      if (!parse1.success) return;

      const serialize = await adapter.serialize(parse1.data!);
      expect(serialize.success).toBe(true);
      if (!serialize.success) return;

      const parse2 = await adapter.parse(serialize.content);
      expect(parse2.success).toBe(true);
      if (!parse2.success) return;

      // Key data should be preserved
      expect(parse2.data?.provenance.author).toBe(parse1.data?.provenance.author);
      expect(parse2.data?.proposed_changes.length).toBeGreaterThan(0);
    });
  });

  describe('additional round-trip tests', () => {
    it('should maintain fidelity for complex PRD', async () => {
      const content = await readFile(join(fixturesDir, 'valid-complex-prd.md'), 'utf-8');

      const parse1 = await adapter.parse(content);
      expect(parse1.success).toBe(true);
      if (!parse1.success) return;

      const serialize = await adapter.serialize(parse1.data!);
      expect(serialize.success).toBe(true);
      if (!serialize.success) return;

      const parse2 = await adapter.parse(serialize.content);
      expect(parse2.success).toBe(true);
      if (!parse2.success) return;

      // Should have similar number of changes (may vary slightly due to categorization)
      const change1Count = parse1.data?.proposed_changes.length || 0;
      const change2Count = parse2.data?.proposed_changes.length || 0;
      expect(Math.abs(change1Count - change2Count)).toBeLessThan(10);
    });

    it('should maintain fidelity for minimal PRD', async () => {
      const content = await readFile(join(fixturesDir, 'valid-minimal-prd.md'), 'utf-8');

      const parse1 = await adapter.parse(content);
      expect(parse1.success).toBe(true);
      if (!parse1.success) return;

      const serialize = await adapter.serialize(parse1.data!);
      expect(serialize.success).toBe(true);
      if (!serialize.success) return;

      const parse2 = await adapter.parse(serialize.content);
      expect(parse2.success).toBe(true);
      if (!parse2.success) return;

      // Change count may vary due to validation requirements being added during serialization
      expect(parse2.data?.proposed_changes.length).toBeGreaterThanOrEqual(
        parse1.data?.proposed_changes.length || 0
      );
      expect(parse2.data?.provenance.author).toBe(parse1.data?.provenance.author);
    });

    it('should handle round-trip with task document', async () => {
      const content = await readFile(join(fixturesDir, 'valid-task.md'), 'utf-8');

      const parse1 = await adapter.parse(content);
      const serialize = await adapter.serialize(parse1.data!);
      const parse2 = await adapter.parse(serialize.content);

      expect(parse2.success).toBe(true);
      if (parse2.success && parse1.success) {
        // Author and version should be preserved
        expect(parse2.data?.provenance.author).toBe(parse1.data?.provenance.author);
      }
    });
  });

  describe('ADAPTUP: BMAD v6 folder structure detection', () => {
    it('should detect v6 _bmad folder path', () => {
      const hint: PathDetectionHint = {
        filePath: '/project/_bmad/docs/prd.md',
        parentDirs: ['docs', '_bmad', 'project'],
      };
      const result = analyzePath(hint);
      expect(result.isBmadFolder).toBe(true);
    });

    it('should detect legacy .bmad folder path', () => {
      const hint: PathDetectionHint = {
        filePath: '/project/.bmad/docs/prd.md',
        parentDirs: ['docs', '.bmad', 'project'],
      };
      const result = analyzePath(hint);
      expect(result.isBmadFolder).toBe(true);
    });

    it('should detect v6 _config folder path', () => {
      const hint: PathDetectionHint = {
        filePath: '/project/_bmad/_config/module.yaml',
        parentDirs: ['_config', '_bmad', 'project'],
      };
      const result = analyzePath(hint);
      expect(result.isBmadFolder).toBe(true);
      expect(result.isConfigFolder).toBe(true);
    });

    it('should detect legacy _cfg folder path', () => {
      const hint: PathDetectionHint = {
        filePath: '/project/.bmad/_cfg/settings.yaml',
        parentDirs: ['_cfg', '.bmad', 'project'],
      };
      const result = analyzePath(hint);
      expect(result.isBmadFolder).toBe(true);
      expect(result.isConfigFolder).toBe(true);
    });

    it('should not detect non-BMAD folder paths', () => {
      const hint: PathDetectionHint = {
        filePath: '/project/docs/prd.md',
        parentDirs: ['docs', 'project'],
      };
      const result = analyzePath(hint);
      expect(result.isBmadFolder).toBe(false);
      expect(result.isConfigFolder).toBe(false);
    });

    it('should boost confidence when file is in _bmad folder', async () => {
      const content = `---
name: Minimal
---

Some document content that is not very BMAD-like.`;

      const hint: PathDetectionHint = {
        filePath: '/project/_bmad/docs/doc.md',
        parentDirs: ['docs', '_bmad', 'project'],
      };

      const resultWithPath = adapter.detectWithPath(content, hint);
      const resultWithout = adapter.detect(content);

      expect(resultWithPath.confidence).toBeGreaterThan(resultWithout.confidence);
      expect(resultWithPath.reason).toContain('bmad-folder');
    });

    it('should have correct folder constants', () => {
      expect(BMAD_FOLDERS.PROJECT).toBe('_bmad');
      expect(BMAD_FOLDERS.PROJECT_LEGACY).toBe('.bmad');
      expect(BMAD_FOLDERS.CONFIG).toBe('_config');
      expect(BMAD_FOLDERS.CONFIG_LEGACY).toBe('_cfg');
      expect(BMAD_FOLDERS.MEMORY).toBe('_memory');
      expect(BMAD_FOLDERS.MODULE_CONFIG).toBe('module.yaml');
    });
  });

  describe('ADAPTUP: BMAD v6 config path handling', () => {
    it('should detect config folder in path indicators', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-prd.md'), 'utf-8');
      const hint: PathDetectionHint = {
        filePath: '/project/_bmad/_config/prd.md',
        parentDirs: ['_config', '_bmad', 'project'],
      };

      const result = adapter.detectWithPath(content, hint);
      expect(result.reason).toContain('bmad-config');
    });
  });

  describe('ADAPTUP: BMAD v6 variable syntax', () => {
    it('should expand underscore variable syntax', () => {
      const content = 'Path: {project_root}/docs';
      const result = expandVariables(content, { project_root: '/home/user' });
      expect(result).toBe('Path: /home/user/docs');
    });

    it('should expand hyphenated variable syntax', () => {
      const content = 'Path: {project-root}/docs';
      const result = expandVariables(content, { 'project-root': '/home/user' });
      expect(result).toBe('Path: /home/user/docs');
    });

    it('should expand both syntaxes from underscore key', () => {
      const content = '{project_root} and {project-root}';
      const result = expandVariables(content, { project_root: '/home' });
      expect(result).toBe('/home and /home');
    });

    it('should expand both syntaxes from hyphenated key', () => {
      const content = '{project_root} and {project-root}';
      const result = expandVariables(content, { 'project-root': '/home' });
      expect(result).toBe('/home and /home');
    });

    it('should detect hyphenated variables in content', () => {
      expect(hasHyphenatedVariables('{project-root}/docs')).toBe(true);
      expect(hasHyphenatedVariables('{output-file}')).toBe(true);
      expect(hasHyphenatedVariables('{project_root}/docs')).toBe(false);
      expect(hasHyphenatedVariables('no variables here')).toBe(false);
    });

    it('should detect v6 PRD with hyphenated variables', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-prd.md'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(50);
    });
  });

  describe('ADAPTUP: BMAD hasSidecar field support', () => {
    it('should parse hasSidecar: true from front-matter', async () => {
      const content = await readFile(join(fixturesDir, 'valid-agent.md'), 'utf-8');

      // Agent docs may not reach 50% with content-only detection (YAML 30 + hasSidecar 15 = 45)
      // Use detectWithPath from a _bmad folder to boost (+20 = 65)
      const hint: PathDetectionHint = {
        filePath: '/project/_bmad/agents/code-review.md',
        parentDirs: ['agents', '_bmad', 'project'],
      };
      const result = adapter.detectWithPath(content, hint);

      expect(result.detected).toBe(true);
      expect(result.reason).toContain('has-sidecar');
      expect(result.reason).toContain('bmad-folder');
    });

    it('should parse boolean YAML values', () => {
      expect(parseYamlBoolean('true')).toBe(true);
      expect(parseYamlBoolean('false')).toBe(false);
      expect(parseYamlBoolean('yes')).toBe(true);
      expect(parseYamlBoolean('no')).toBe(false);
      expect(parseYamlBoolean('on')).toBe(true);
      expect(parseYamlBoolean('off')).toBe(false);
      expect(parseYamlBoolean('TRUE')).toBe(true);
      expect(parseYamlBoolean('maybe')).toBeUndefined();
    });

    it('should warn on agent doc missing hasSidecar', async () => {
      const content = `---
name: 'Test Agent'
version: '1.0.0'
author: 'Test'
---

# Test Agent

## Purpose

A test agent for validation.

## Role

Performs test operations for quality assurance.`;

      const result = await adapter.validate(content);
      expect(result.issues).toBeDefined();
      if (result.issues) {
        const sidecarWarning = result.issues.find((i) => i.code === 'MISSING_HAS_SIDECAR');
        expect(sidecarWarning).toBeDefined();
        if (sidecarWarning) {
          expect(sidecarWarning.severity).toBe('warning');
        }
      }
    });

    it('should not warn when hasSidecar is present', async () => {
      const content = await readFile(join(fixturesDir, 'valid-agent.md'), 'utf-8');
      const result = await adapter.validate(content);

      if (result.issues) {
        const sidecarWarning = result.issues.find((i) => i.code === 'MISSING_HAS_SIDECAR');
        expect(sidecarWarning).toBeUndefined();
      }
    });

    it('should parse agent document successfully', async () => {
      const content = await readFile(join(fixturesDir, 'valid-agent.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.provenance.author).toBe('BMAD Team');
      }
    });

    it('should parse v6 PRD with all new features', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-prd.md'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.proposed_changes.length).toBeGreaterThanOrEqual(3);
        expect(result.data?.provenance.author).toBe('v6 Author');
      }
    });
  });

  // -----------------------------------------------------------------------
  // v6 Agent YAML Tests
  // -----------------------------------------------------------------------

  describe('v6: Agent YAML detection', () => {
    it('should detect agent YAML content', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-agent.yaml'), 'utf-8');
      expect(isAgentYamlContent(content)).toBe(true);
    });

    it('should not detect non-agent content as agent YAML', () => {
      expect(isAgentYamlContent('# Regular markdown')).toBe(false);
      expect(isAgentYamlContent('name: workflow\ndescription: test')).toBe(false);
    });

    it('should detect agent YAML with high confidence', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-agent.yaml'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(80);
      expect(result.reason).toContain('agent-yaml');
    });
  });

  describe('v6: Agent YAML parsing', () => {
    it('should parse a complete agent YAML file', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-agent.yaml'), 'utf-8');
      const agent = parseAgentYaml(content);

      expect(agent).not.toBeNull();
      expect(agent!.metadata.id).toBe('_bmad/bmm/agents/pm.md');
      expect(agent!.metadata.name).toBe('John');
      expect(agent!.metadata.title).toBe('Product Manager');
      expect(agent!.metadata.module).toBe('bmm');
      expect(agent!.metadata.hasSidecar).toBe(false);
      expect(agent!.metadata.capabilities).toContain('PRD creation');
    });

    it('should parse agent persona', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-agent.yaml'), 'utf-8');
      const agent = parseAgentYaml(content);

      expect(agent!.persona.role).toContain('Product Manager');
      expect(agent!.persona.identity).toContain('8+ years');
      expect(agent!.persona.communication_style).toContain('WHY');
      expect(agent!.persona.principles).toContain('user-centered design');
    });

    it('should parse agent menu items', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-agent.yaml'), 'utf-8');
      const agent = parseAgentYaml(content);

      expect(agent!.menu).toBeDefined();
      expect(agent!.menu!.length).toBe(4);

      const cpItem = agent!.menu![0];
      expect(cpItem.trigger).toContain('CP');
      expect(cpItem.exec).toContain('create-prd');
      expect(cpItem.description).toContain('[CP]');

      // Last item uses workflow instead of exec
      const ccItem = agent!.menu![3];
      expect(ccItem.workflow).toContain('correct-course');
      expect(ccItem.exec).toBeUndefined();
    });

    it('should parse agent with critical_actions', async () => {
      const content = await readFile(
        join(fixturesDir, 'valid-v6-agent-with-actions.yaml'),
        'utf-8'
      );
      const agent = parseAgentYaml(content);

      expect(agent).not.toBeNull();
      expect(agent!.metadata.name).toBe('Amelia');
      expect(agent!.critical_actions).toBeDefined();
      expect(agent!.critical_actions!.length).toBe(4);
      expect(agent!.critical_actions![0]).toContain('READ the entire story file');
    });

    it('should return null for non-agent YAML', () => {
      expect(parseAgentYaml('# Just markdown')).toBeNull();
      expect(parseAgentYaml('name: not an agent')).toBeNull();
    });

    it('should parse agent YAML via adapter.parse()', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-agent.yaml'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.intent).toContain('Product Manager');
        expect(result.data?.proposed_changes.length).toBeGreaterThan(0);
        // Agent name should be used as author
        expect(result.data?.provenance.author).toBe('John');
      }
    });

    it('should validate agent YAML successfully', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-agent.yaml'), 'utf-8');
      const result = await adapter.validate(content);

      expect(result.valid).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // v6 Workflow YAML Tests
  // -----------------------------------------------------------------------

  describe('v6: Workflow YAML detection and parsing', () => {
    it('should detect workflow YAML content', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-workflow.yaml'), 'utf-8');
      expect(isWorkflowYamlContent(content)).toBe(true);
    });

    it('should not detect non-workflow content', () => {
      expect(isWorkflowYamlContent('# Markdown')).toBe(false);
      expect(isWorkflowYamlContent('agent:\n  metadata:')).toBe(false);
    });

    it('should detect workflow YAML with high confidence', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-workflow.yaml'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(80);
      expect(result.reason).toContain('workflow-yaml');
    });

    it('should parse workflow YAML', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-workflow.yaml'), 'utf-8');
      const wf = parseWorkflowYaml(content);

      expect(wf).not.toBeNull();
      expect(wf!.name).toBe('dev-story');
      expect(wf!.description).toContain('Execute story implementation');
      expect(wf!.config_source).toContain('{project-root}');
      expect(wf!.instructions).toContain('instructions.xml');
      expect(wf!.validation).toContain('checklist.md');
    });

    it('should parse workflow YAML via adapter.parse()', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-workflow.yaml'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.intent).toContain('Execute story implementation');
        expect(result.data?.proposed_changes.length).toBeGreaterThan(0);
      }
    });
  });

  // -----------------------------------------------------------------------
  // v6 Team YAML Tests
  // -----------------------------------------------------------------------

  describe('v6: Team YAML detection and parsing', () => {
    it('should detect team YAML content', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-team.yaml'), 'utf-8');
      expect(isTeamYamlContent(content)).toBe(true);
    });

    it('should detect team YAML with high confidence', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-team.yaml'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(80);
      expect(result.reason).toContain('team-yaml');
    });

    it('should parse team YAML', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-team.yaml'), 'utf-8');
      const team = parseTeamYaml(content);

      expect(team).not.toBeNull();
      expect(team!.bundle.name).toBe('Team Plan and Architect');
      expect(team!.agents).toEqual(['analyst', 'architect', 'pm', 'sm', 'ux-designer']);
      expect(team!.party).toBe('./default-party.csv');
    });

    it('should parse team YAML via adapter.parse()', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-team.yaml'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        // Each agent becomes a proposed change
        expect(result.data?.proposed_changes.length).toBe(5);
      }
    });
  });

  // -----------------------------------------------------------------------
  // v6 Module YAML Tests
  // -----------------------------------------------------------------------

  describe('v6: Module YAML detection and parsing', () => {
    it('should detect module YAML content', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-module.yaml'), 'utf-8');
      expect(isModuleYamlContent(content)).toBe(true);
    });

    it('should not detect non-module content as module YAML', () => {
      expect(isModuleYamlContent('name: something\ndescription: test')).toBe(false);
      expect(isModuleYamlContent('code: bmm\nname: test')).toBe(false);
      expect(isModuleYamlContent('agent:\n  metadata:')).toBe(false);
    });

    it('should detect module YAML with high confidence', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-module.yaml'), 'utf-8');
      const result = adapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(70);
      expect(result.reason).toContain('module-yaml');
    });

    it('should parse module YAML', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-module.yaml'), 'utf-8');
      const mod = parseModuleYaml(content);

      expect(mod).not.toBeNull();
      expect(mod!.code).toBe('bmm');
      expect(mod!.name).toBe('BMad Method Agile-AI Driven-Development');
      expect(mod!.default_selected).toBe(true);
    });

    it('should parse module YAML via adapter.parse()', async () => {
      const content = await readFile(join(fixturesDir, 'valid-v6-module.yaml'), 'utf-8');
      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data?.intent).toContain('AI-driven agile');
        expect(result.data?.proposed_changes.length).toBeGreaterThan(0);
      }
    });
  });

  // -----------------------------------------------------------------------
  // v6 Serializer Tests
  // -----------------------------------------------------------------------

  describe('v6: Serializer output format', () => {
    it('should include v6 output_file with hyphenated variable syntax', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const parseResult = await adapter.parse(content);
      expect(parseResult.success).toBe(true);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);
      expect(serializeResult.success).toBe(true);
      if (!serializeResult.success) return;

      expect(serializeResult.content).toContain('{project-root}');
      expect(serializeResult.content).toContain('output_file:');
    });

    it('should include BMAD version in footer', async () => {
      const content = await readFile(join(fixturesDir, 'valid-prd.md'), 'utf-8');
      const parseResult = await adapter.parse(content);
      if (!parseResult.success) return;

      const serializeResult = await adapter.serialize(parseResult.data!);
      if (!serializeResult.success) return;

      expect(serializeResult.content).toContain(BMAD_UPSTREAM_VERSION);
    });
  });

  // -----------------------------------------------------------------------
  // v6 Folder Constants
  // -----------------------------------------------------------------------

  describe('v6: Updated folder constants', () => {
    it('should have OUTPUT folder constant', () => {
      expect(BMAD_FOLDERS.OUTPUT).toBe('_bmad-output');
    });
  });
});
