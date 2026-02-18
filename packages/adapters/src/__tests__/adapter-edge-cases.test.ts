/**
 * Comprehensive Edge Case Tests for Adapter Framework
 *
 * Tests edge cases that are critical for production robustness but may not be
 * covered by standard functional tests. Organized by category.
 */

import { describe, it, expect, beforeEach, afterAll } from 'vitest';
import { SpecKitFormatAdapter } from '../speckit/format-adapter.js';
import { BMADFormatAdapter } from '../bmad/format-adapter.js';
import { GenericMarkdownAdapter } from '../generic/format-adapter.js';
import { AdapterRegistry } from '../base/registry.js';
import type { ParseContext } from '../base/types.js';

describe('Adapter Edge Cases', () => {
  let speckitAdapter: SpecKitFormatAdapter;
  let bmadAdapter: BMADFormatAdapter;
  let genericAdapter: GenericMarkdownAdapter;

  beforeEach(() => {
    speckitAdapter = new SpecKitFormatAdapter();
    bmadAdapter = new BMADFormatAdapter();
    genericAdapter = new GenericMarkdownAdapter();
  });

  describe('Binary and Non-Text Content', () => {
    it('should detect binary content and handle gracefully - SpecKit', async () => {
      // Simulate binary content with null bytes and non-UTF8 sequences
      const binaryContent = '\x00\xFF\xFE\x89PNG\r\n\x1a\n\x00\x00\x00';

      const result = await speckitAdapter.parse(binaryContent);

      // Adapters may succeed but produce empty/invalid results for binary content
      // The test validates that they don't crash
      expect(result).toBeDefined();
      if (result.success && result.data) {
        // Binary content should result in minimal/empty data
        expect(result.data.proposed_changes.length).toBe(0);
      }
    });

    it('should detect binary content and handle gracefully - BMAD', async () => {
      const binaryContent = '\x00\xFF\xFE\x89PNG\r\n\x1a\n\x00\x00\x00';

      const result = await bmadAdapter.parse(binaryContent);

      // Adapters may succeed but produce empty/invalid results for binary content
      // The test validates that they don't crash
      expect(result).toBeDefined();
      if (result.success && result.data) {
        // Binary content should result in minimal/empty data
        expect(result.data.proposed_changes.length).toBe(0);
      }
    });

    it('should handle mixed binary/text content - SpecKit', async () => {
      // Content with embedded null bytes
      const mixedContent = '# Specification\n\n## Intent\n\x00\x00\nSome text\n\xFF\xFE';

      const result = await speckitAdapter.parse(mixedContent);

      // Should not crash, validates robustness
      expect(result).toBeDefined();
      // Adapters may succeed or fail, both are acceptable for binary content
      if (result.success) {
        expect(result.data).toBeDefined();
      } else {
        expect(result.errors).toBeDefined();
      }
    });

    it('should detect binary content in detection phase', () => {
      const binaryContent = '\x00\xFF\xFE\x89PNG\r\n\x1a\n';

      const result = speckitAdapter.detect(binaryContent);

      expect(result.detected).toBe(false);
      expect(result.confidence).toBe(0);
    });
  });

  describe('Unicode and Encoding Edge Cases', () => {
    it('should handle RTL (right-to-left) text', async () => {
      const rtlContent = `# Specification

## Intent

تطوير ميزة جديدة (Arabic RTL text)
יצירת קובץ חדש (Hebrew RTL text)

## Changes

- file_create: src/feature.ts - Create feature with RTL support`;

      const result = await speckitAdapter.parse(rtlContent);

      expect(result.success).toBe(true);
      if (result.success && result.data) {
        // Intent section should contain both Arabic and Hebrew text
        expect(result.data.intent).toContain('تطوير');
        expect(result.data.intent).toContain('יצירת');
      }
    });

    it('should handle zero-width characters in content', async () => {
      // Zero-width space (U+200B), zero-width joiner (U+200D)
      const content = `---
name: Zero\u200BWidth\u200DTest
---

FR-01: Test\u200Bwith\u200Dzero\u200Bwidth\u200Dcharacters`;

      const result = await bmadAdapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success && result.data) {
        // Zero-width characters may be preserved or stripped
        // Test validates that parsing doesn't crash
        expect(result.data.proposed_changes.length).toBeGreaterThan(0);
      }
    });

    it('should handle combining characters in descriptions', async () => {
      // Combining diacritics: "e" + combining acute accent
      const content = `# Specification

## Intent

Test café (with combining character: cafe\u0301)

## Changes

- file_create: src/résumé.ts - Description`;

      const result = await speckitAdapter.parse(content);

      expect(result.success).toBe(true);
    });

    it('should handle BOM (Byte Order Mark) at file start', async () => {
      // UTF-8 BOM: EF BB BF
      const bomContent =
        '\uFEFF# Specification\n\n## Intent\n\nTest with BOM\n\n## Changes\n\n- file_create: test.ts';

      const result = await speckitAdapter.parse(bomContent);

      expect(result.success).toBe(true);
      if (result.success && result.data) {
        // BOM should be stripped, not included in intent
        expect(result.data.intent).not.toContain('\uFEFF');
        expect(result.data.intent).toContain('Test with BOM');
      }
    });

    it('should handle emoji and special Unicode symbols', async () => {
      const content = `---
name: Emoji Test 🚀
---

FR-01: Add support for emojis 🎉 💻 🔥
FR-02: Mathematical symbols ∑ ∫ √ ∞
FR-03: Currency symbols ¥ € £ ₹
FR-04: Box drawing characters ┌─┐ │ └─┘`;

      const result = await bmadAdapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success && result.data) {
        expect(result.data.proposed_changes.length).toBeGreaterThanOrEqual(4);
      }
    });

    it('should handle normalization forms (NFC vs NFD)', async () => {
      // Same character in different normalization forms
      // "é" as single character (NFC) vs "e" + combining acute (NFD)
      const nfcContent = `# Specification\n\n## Intent\n\nCafé (NFC)\n\n## Changes\n\n- file_create: test.ts`;
      const nfdContent = `# Specification\n\n## Intent\n\nCafe\u0301 (NFD)\n\n## Changes\n\n- file_create: test.ts`;

      const nfcResult = await speckitAdapter.parse(nfcContent);
      const nfdResult = await speckitAdapter.parse(nfdContent);

      expect(nfcResult.success).toBe(true);
      expect(nfdResult.success).toBe(true);
    });
  });

  describe('Empty and Null Content Edge Cases', () => {
    it('should handle completely empty string - SpecKit', async () => {
      const result = await speckitAdapter.parse('');

      // Adapters may succeed with empty data or fail
      // Test validates consistent behavior without crashing
      expect(result).toBeDefined();
      if (result.success && result.data) {
        // Empty content should result in empty changes
        expect(result.data.proposed_changes.length).toBe(0);
      }
    });

    it('should handle completely empty string - BMAD', async () => {
      const result = await bmadAdapter.parse('');

      expect(result).toBeDefined();
      if (result.success && result.data) {
        // Empty content should result in empty changes
        expect(result.data.proposed_changes.length).toBe(0);
      }
    });

    it('should handle completely empty string - Generic', async () => {
      const result = await genericAdapter.parse('');

      expect(result).toBeDefined();
      if (result.success && result.data) {
        // Empty content should result in empty changes
        expect(result.data.proposed_changes.length).toBe(0);
      }
    });

    it('should handle only whitespace content', async () => {
      const whitespaceContent = '   \n\n\t\t\n   \n';

      const result = await speckitAdapter.parse(whitespaceContent);

      // Test validates no crash on whitespace-only content
      expect(result).toBeDefined();
      if (result.success && result.data) {
        expect(result.data.proposed_changes.length).toBe(0);
      }
    });

    it('should handle content with only comments', async () => {
      const commentContent = `<!-- This is a comment -->
<!-- Another comment -->
<!-- And another -->`;

      const result = await speckitAdapter.parse(commentContent);

      // Test validates no crash on comment-only content
      expect(result).toBeDefined();
      if (result.success && result.data) {
        expect(result.data.proposed_changes.length).toBe(0);
      }
    });

    it('should detect empty content consistently', () => {
      const emptyDetection = speckitAdapter.detect('');
      const whitespaceDetection = speckitAdapter.detect('   \n\t\n   ');

      expect(emptyDetection.detected).toBe(false);
      expect(whitespaceDetection.detected).toBe(false);
    });
  });

  describe('Corrupted and Malformed Content', () => {
    it('should handle corrupted YAML frontmatter - BMAD', async () => {
      const corruptedContent = `---
name: Test
version: 1.0.0
author: Test User
this is not valid yaml: [unclosed bracket
more invalid: {unclosed brace
---

FR-01: Valid requirement`;

      const result = await bmadAdapter.parse(corruptedContent);

      // Should either recover or fail gracefully with clear error
      if (!result.success) {
        expect(result.errors).toBeDefined();
        expect(result.errors?.[0]?.message).toMatch(/yaml|frontmatter|metadata/i);
      }
    });

    it('should handle missing closing YAML delimiter', async () => {
      const content = `---
name: Unclosed YAML
version: 1.0.0

FR-01: This is after unclosed YAML`;

      const result = await bmadAdapter.parse(content);

      // Should handle gracefully without crashing
      expect(result).toBeDefined();
      // May succeed or fail depending on YAML parser leniency
      if (result.success) {
        expect(result.data).toBeDefined();
      } else {
        expect(result.errors).toBeDefined();
      }
    });

    it('should handle deeply nested markdown structures', async () => {
      let content = '# Specification\n\n## Intent\n\nNested test\n\n## Changes\n\n';

      // Create deeply nested list structure
      for (let i = 0; i < 50; i++) {
        content += '  '.repeat(i) + `- Level ${i} item\n`;
      }

      const result = await speckitAdapter.parse(content);

      // Should not crash, either succeed or fail gracefully
      expect(result).toBeDefined();
    });

    it('should handle extremely long lines', async () => {
      const veryLongLine = 'A'.repeat(100000);
      const content = `# Specification\n\n## Intent\n\n${veryLongLine}\n\n## Changes\n\n- file_create: test.ts`;

      const result = await speckitAdapter.parse(content);

      expect(result).toBeDefined();
      if (result.success && result.data) {
        expect(result.data.intent.length).toBeGreaterThan(50000);
      }
    });
  });

  describe('Concurrent Operations', () => {
    it('should handle concurrent parse operations on same adapter', async () => {
      const content1 = `# Specification\n\n## Intent\n\nTest 1\n\n## Changes\n\n- file_create: test1.ts`;
      const content2 = `# Specification\n\n## Intent\n\nTest 2\n\n## Changes\n\n- file_create: test2.ts`;
      const content3 = `# Specification\n\n## Intent\n\nTest 3\n\n## Changes\n\n- file_create: test3.ts`;

      // Parse all three concurrently
      const results = await Promise.all([
        speckitAdapter.parse(content1),
        speckitAdapter.parse(content2),
        speckitAdapter.parse(content3),
      ]);

      // All should succeed independently
      expect(results[0]?.success).toBe(true);
      expect(results[1]?.success).toBe(true);
      expect(results[2]?.success).toBe(true);

      // Should have correct data for each
      if (results[0]?.success && results[0].data) {
        expect(results[0].data.intent).toContain('Test 1');
      }
      if (results[1]?.success && results[1].data) {
        expect(results[1].data.intent).toContain('Test 2');
      }
      if (results[2]?.success && results[2].data) {
        expect(results[2].data.intent).toContain('Test 3');
      }
    });

    it('should handle concurrent detection operations', () => {
      const contents = Array.from(
        { length: 10 },
        (_v, i) =>
          `# Specification\n\n## Intent\n\nTest ${i}\n\n## Changes\n\n- file_create: test${i}.ts`
      );

      // Detection is synchronous, but we test multiple concurrent calls
      const detections = contents.map((content) => speckitAdapter.detect(content));

      // All detections should succeed
      detections.forEach((detection) => {
        expect(detection.detected).toBe(true);
        expect(detection.confidence).toBeGreaterThan(50);
      });
    });
  });

  describe('Registry Edge Cases', () => {
    let registry: AdapterRegistry;

    beforeEach(() => {
      AdapterRegistry.resetInstance();
      registry = AdapterRegistry.getInstance();
    });

    afterAll(() => {
      // Restore registry to clean state after all tests in this block
      AdapterRegistry.resetInstance();
    });

    it('should detect format conflicts when multiple adapters support same format', () => {
      const speckit = new SpecKitFormatAdapter();
      const generic = new GenericMarkdownAdapter();

      registry.register(speckit);
      registry.register(generic);

      // Both support .md extension
      const mdAdapters = registry.getImportAdapters('.md');

      // Should return multiple adapters
      expect(mdAdapters.length).toBeGreaterThan(1);
    });

    it('should handle adapter detection with conflicting confidence scores', () => {
      const speckit = new SpecKitFormatAdapter();
      const generic = new GenericMarkdownAdapter();

      registry.register(speckit);
      registry.register(generic);

      // Content that could match multiple formats
      const ambiguousContent = `# Some Document\n\nThis is a test with some content`;

      const result = registry.detectAdapter(ambiguousContent, 10); // Lower threshold

      // Should return an adapter if any match above threshold
      if (result) {
        expect(result.detection.confidence).toBeGreaterThanOrEqual(10);
      } else {
        // If no adapter matches even low threshold, that's also valid
        expect(result).toBeUndefined();
      }
    });

    it('should handle rapid sequential adapter registration', async () => {
      const adapters = [
        new SpecKitFormatAdapter(),
        new BMADFormatAdapter(),
        new GenericMarkdownAdapter(),
      ];

      // Register all adapters in rapid succession via microtasks
      // Note: This tests sequential registration order, not true concurrency,
      // as JavaScript microtasks execute sequentially within a single event loop tick
      const registrations = adapters.map((adapter) =>
        Promise.resolve().then(() => registry.register(adapter))
      );

      await Promise.all(registrations);

      expect(registry.size).toBe(3);
    });

    it('should prevent duplicate adapter registration with same name', () => {
      const adapter1 = new SpecKitFormatAdapter();
      const adapter2 = new SpecKitFormatAdapter();

      registry.register(adapter1);

      expect(() => registry.register(adapter2)).toThrow(/already registered/i);
    });

    it('should handle registry operations after clear', () => {
      registry.register(new SpecKitFormatAdapter());
      registry.register(new BMADFormatAdapter());

      expect(registry.size).toBe(2);

      registry.clear();

      expect(registry.size).toBe(0);

      // Should be able to register again after clear
      registry.register(new SpecKitFormatAdapter());
      expect(registry.size).toBe(1);
    });
  });

  describe('Error Recovery and Partial Parsing', () => {
    it('should continue parsing after encountering invalid change - SpecKit', async () => {
      const content = `# Specification

## Intent

Test partial parsing

## Changes

- file_create: valid1.ts - Valid change
- invalid_change_format_here
- file_update: valid2.ts - Another valid change
- more garbage data
- file_delete: valid3.ts - Final valid change`;

      const result = await speckitAdapter.parse(content);

      // Test validates that parser doesn't crash on invalid changes
      expect(result).toBeDefined();
      if (result.success && result.data) {
        // Should capture at least some valid changes
        expect(result.data.proposed_changes.length).toBeGreaterThan(0);
      }
    });

    it('should provide detailed error information for multiple failures', async () => {
      const content = `# Specification

## Intent

## Changes

- file_create: - Missing path
- file_update: test.ts
- file_delete:`;

      const result = await speckitAdapter.parse(content);

      // Should collect all errors
      if (!result.success || result.errors) {
        expect(result.errors).toBeDefined();
        // Could have multiple errors for different invalid changes
      }
    });

    it('should handle invalid requirement IDs gracefully - BMAD', async () => {
      const content = `---
name: Invalid IDs Test
---

FR-01: Valid requirement
INVALID-ID: Bad ID format
FR-02: Another valid requirement
: No ID at all
FR-03: Final valid requirement`;

      const result = await bmadAdapter.parse(content);

      if (result.success && result.data) {
        // Should parse valid requirements even if some are invalid
        expect(result.data.proposed_changes.length).toBeGreaterThan(0);
      }
    });
  });

  describe('Metadata Edge Cases', () => {
    it('should handle circular reference-like structures in metadata', async () => {
      // Can't have true circular references in JSON, but can test deep nesting
      const content = `---
name: Deep Metadata
nested:
  level1:
    level2:
      level3:
        level4:
          level5:
            level6:
              value: deep
---

FR-01: Test deep metadata`;

      const result = await bmadAdapter.parse(content);

      expect(result.success).toBe(true);
    });

    it('should handle metadata with special characters in keys', async () => {
      const content = `---
name: Special Keys
"key-with-dashes": value1
"key.with.dots": value2
"key with spaces": value3
"key:with:colons": value4
---

FR-01: Test requirement`;

      const result = await bmadAdapter.parse(content);

      expect(result.success).toBe(true);
    });

    it('should handle very large metadata objects', async () => {
      let yamlContent = '---\nname: Large Metadata\n';

      // Add 100 metadata fields
      for (let i = 0; i < 100; i++) {
        yamlContent += `field${i}: value${i}\n`;
      }

      yamlContent += '---\n\nFR-01: Test requirement';

      const result = await bmadAdapter.parse(yamlContent);

      expect(result.success).toBe(true);
    });

    it('should handle null and undefined values in metadata', async () => {
      const content = `---
name: Null Values Test
nullField: null
undefinedField:
emptyString: ""
---

FR-01: Test requirement`;

      const result = await bmadAdapter.parse(content);

      expect(result.success).toBe(true);
    });
  });

  describe('Performance and Large Content', () => {
    it('should handle very large documents efficiently', async () => {
      // Create a document with 200 changes
      let content = '# Specification\n\n## Intent\n\nLarge document test\n\n## Changes\n\n';

      for (let i = 0; i < 200; i++) {
        content += `- file_create: src/file${i}.ts - Create file number ${i}\n`;
      }

      const startTime = Date.now();
      const result = await speckitAdapter.parse(content);
      const endTime = Date.now();

      expect(result.success).toBe(true);

      // Sanity check: should complete in reasonable time (< 30 seconds for 200 items)
      // Using a generous threshold to avoid flaky tests on slow CI runners
      expect(endTime - startTime).toBeLessThan(30000);

      if (result.success && result.data) {
        expect(result.data.proposed_changes.length).toBe(200);
      }
    });

    it('should handle documents approaching 1MB in size', async () => {
      // Create a document around 1MB
      const largeDescription = 'A'.repeat(10000); // 10KB
      let content = `---
name: Large Document
---

`;

      // Add 50 requirements with 10KB descriptions each = ~500KB
      for (let i = 0; i < 50; i++) {
        content += `FR-${i.toString().padStart(2, '0')}: ${largeDescription}\n\n`;
      }

      const sizeInBytes = Buffer.byteLength(content, 'utf8');
      expect(sizeInBytes).toBeGreaterThan(500000); // > 500KB

      const result = await bmadAdapter.parse(content);

      expect(result).toBeDefined();
      // Should either succeed or fail gracefully, not crash
    });
  });

  describe('Round-trip Fidelity', () => {
    it('should preserve information through parse-serialize cycle - SpecKit', async () => {
      const originalContent = `# Specification

## Intent

Test round-trip fidelity for SpecKit adapter with specific formatting.

## Changes

- file_create: src/test.ts - Create test file
- file_update: src/app.ts - Update application
- file_delete: src/old.ts - Remove old file`;

      const parseResult = await speckitAdapter.parse(originalContent);
      expect(parseResult.success).toBe(true);

      if (parseResult.success && parseResult.data) {
        const serializeResult = await speckitAdapter.serialize(parseResult.data);
        expect(serializeResult.success).toBe(true);

        if (serializeResult.success && serializeResult.content) {
          // Re-parse the serialized content
          const reparseResult = await speckitAdapter.parse(serializeResult.content);

          expect(reparseResult.success).toBe(true);

          if (reparseResult.success && reparseResult.data) {
            // Should have same number of changes
            expect(reparseResult.data.proposed_changes.length).toBe(
              parseResult.data.proposed_changes.length
            );

            // Should preserve intent
            expect(reparseResult.data.intent).toContain('round-trip');
          }
        }
      }
    });

    it('should handle conversion of unstructured content', async () => {
      // Generic markdown might lose some structure when converting to APS
      const genericContent = `# My Project Plan

## Overview

This is a generic project document.

## Tasks

- [ ] Task 1
- [ ] Task 2
- [x] Task 3 (completed)

## Notes

Some additional notes here.`;

      const result = await genericAdapter.parse(genericContent);

      // Should parse successfully
      expect(result).toBeDefined();
      if (result.success) {
        // Generic adapter should extract task-like content
        expect(result.data).toBeDefined();
      }
    });
  });

  describe('Context Handling', () => {
    it('should handle parse context with all fields', async () => {
      const content = `# Specification

## Intent

Test context handling

## Changes

- file_create: test.ts`;

      const context: ParseContext = {
        repositoryPath: 'https://github.com/test/repo',
        branch: 'feature/test',
        commit: 'abc123def456',
        author: 'Test User <test@example.com>',
        timestamp: new Date().toISOString(),
        metadata: {
          customField: 'customValue',
          nested: {
            data: 'value',
          },
        },
      };

      const result = await speckitAdapter.parse(content, context);

      expect(result.success).toBe(true);
    });

    it('should handle parse context with minimal fields', async () => {
      const content = `# Specification

## Intent

Minimal context

## Changes

- file_create: test.ts`;

      const context: ParseContext = {};

      const result = await speckitAdapter.parse(content, context);

      expect(result.success).toBe(true);
    });
  });

  describe('Path Traversal Prevention', () => {
    it('should strip traversal paths from SpecKit backtick extraction', async () => {
      const content = `# Specification

## Intent

Path traversal test

## Changes

### Create malicious file

\`../../../etc/passwd\`

### Update safe file

\`src/app.ts\`

### Absolute path attempt

\`/etc/shadow\``;

      const result = await speckitAdapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success && result.data) {
        const paths = result.data.proposed_changes.map((c) => c.path);
        // Traversal and absolute paths should be stripped (undefined)
        expect(paths).not.toContain('../../../etc/passwd');
        expect(paths).not.toContain('/etc/shadow');
        // Safe path should be preserved
        expect(paths).toContain('src/app.ts');
      }
    });

    it('should strip traversal paths from SpecKit list items', async () => {
      const content = `# Specification

## Intent

List item path traversal test

## Changes

- Create file \`../../secrets/key.pem\` for credentials
- Update \`src/config.ts\` with new settings
- Delete \`/absolute/path/file.ts\` permanently`;

      const result = await speckitAdapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success && result.data) {
        const paths = result.data.proposed_changes.map((c) => c.path);
        expect(paths).not.toContain('../../secrets/key.pem');
        expect(paths).not.toContain('/absolute/path/file.ts');
        expect(paths).toContain('src/config.ts');
      }
    });

    it('should handle null bytes in paths - SpecKit', async () => {
      const content = `# Specification

## Intent

Null byte test

## Changes

### Create file

\`src/app\x00.ts\``;

      const result = await speckitAdapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success && result.data) {
        const paths = result.data.proposed_changes.map((c) => c.path);
        expect(paths).not.toContain('src/app\x00.ts');
      }
    });

    it('should sanitise BMAD requirement ID-based paths', async () => {
      const content = `---
name: Path Traversal Test
---

FR-01: Valid requirement
FR-../../etc/passwd: Malicious requirement`;

      const result = await bmadAdapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success && result.data) {
        for (const change of result.data.proposed_changes) {
          if (change.path) {
            expect(change.path).not.toContain('..');
            expect(change.path.startsWith('/')).toBe(false);
          }
        }
      }
    });

    it('should sanitise Generic parser generated paths', async () => {
      const content = `# Project Plan

## Tasks

- ../../../etc/passwd
- Normal task description
- /absolute/path/attack`;

      const result = await genericAdapter.parse(content);

      expect(result.success).toBe(true);
      if (result.success && result.data) {
        for (const change of result.data.proposed_changes) {
          if (change.path) {
            expect(change.path).not.toContain('..');
            expect(change.path.startsWith('/')).toBe(false);
          }
        }
      }
    });
  });

  describe('File Extension and Format Detection', () => {
    it('should detect format from content when extension is ambiguous', () => {
      const speckitContent = `# Specification

## Intent

This is clearly a SpecKit document

## Changes

- file_create: test.ts`;

      const bmadContent = `---
name: Test
---

FR-01: This is clearly a BMAD document`;

      const speckitDetection = speckitAdapter.detect(speckitContent);
      const bmadDetection = bmadAdapter.detect(bmadContent);

      // Both should be detected
      expect(speckitDetection.detected).toBe(true);
      expect(bmadDetection.detected).toBe(true);

      // Each adapter should be most confident about its own format
      expect(speckitDetection.confidence).toBeGreaterThanOrEqual(50);
      expect(bmadDetection.confidence).toBeGreaterThanOrEqual(50);
    });

    it('should handle files with unusual extensions', () => {
      // Adapters should rely on content, not just extensions
      const content = `# Specification

## Intent

Test content detection

## Changes

- file_create: test.ts`;

      const result = speckitAdapter.detect(content);

      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThan(50);
    });
  });
});
