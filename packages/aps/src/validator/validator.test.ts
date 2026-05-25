/**
 * Validator module tests
 */

import { describe, it, expect } from 'vitest';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { validatePlanningDoc, formatValidationIssues, type ValidationResult } from './index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const fixturesDir = join(__dirname, '__fixtures__');

describe('validatePlanningDoc', () => {
  describe('valid documents', () => {
    it('should validate a valid leaf spec with no errors', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'valid-leaf.aps.md'));

      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should validate canonical Work Items and Outcome aliases', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'canonical-work-items.aps.md'));

      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
      expect(result.warnings.some((w) => w.rule === 'missing-expected-outcome')).toBe(false);
    });

    it('should warn when Work Items has no task entries before the next section', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'empty-work-items.aps.md'));

      expect(result.valid).toBe(true);
      expect(
        result.warnings.some(
          (w) => w.rule === 'required-sections' && w.message.includes('no task entries')
        )
      ).toBe(true);
    });

    it('should validate a valid index file with linked modules', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'valid-index.aps.md'));

      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('required-sections rule', () => {
    it('should error when leaf spec is missing H1 title', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'missing-h1.aps.md'));

      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.rule === 'required-sections')).toBe(true);
      expect(result.errors.some((e) => e.message.includes('H1 title'))).toBe(true);
    });

    it('should error when leaf spec is missing ## Tasks section', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'missing-tasks-section.aps.md'));

      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.rule === 'required-sections')).toBe(true);
      expect(result.errors.some((e) => e.message.includes('## Tasks'))).toBe(true);
    });

    it('should error when index file is missing ## Modules section', async () => {
      // This file doesn't have ## Modules, so it's treated as a leaf spec
      // and will error because it's missing ## Tasks
      const result = await validatePlanningDoc(join(fixturesDir, 'missing-modules-section.aps.md'));

      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.rule === 'required-sections')).toBe(true);
    });
  });

  describe('task-format rule', () => {
    it('should error when task heading has invalid format', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'invalid-task-id.aps.md'));

      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.rule === 'task-format')).toBe(true);
    });

    it('should error on lowercase task IDs', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'invalid-task-id.aps.md'));

      const formatErrors = result.errors.filter((e) => e.rule === 'task-format');
      expect(formatErrors.some((e) => e.message.includes('test-001'))).toBe(true);
    });

    it('should error on task IDs with non-3-digit numbers', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'invalid-task-id.aps.md'));

      const formatErrors = result.errors.filter((e) => e.rule === 'task-format');
      expect(formatErrors.some((e) => e.message.includes('T-1'))).toBe(true);
    });

    it('should error on task IDs with scope longer than 10 characters', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'invalid-task-id.aps.md'));

      const formatErrors = result.errors.filter((e) => e.rule === 'task-format');
      expect(formatErrors.some((e) => e.message.includes('VERYLONGSCOPE123-001'))).toBe(true);
    });
  });

  describe('task-intent rule', () => {
    it('should error when task is missing Intent field', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'missing-intent.aps.md'));

      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.rule === 'task-intent')).toBe(true);
      expect(result.errors.some((e) => e.message.includes('Intent'))).toBe(true);
    });
  });

  describe('missing-confidence rule (warning)', () => {
    it('should warn when task is missing Confidence field', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'missing-confidence.aps.md'));

      // Should be valid (warnings don't invalidate)
      expect(result.valid).toBe(true);
      expect(result.warnings.some((w) => w.rule === 'missing-confidence')).toBe(true);
      expect(result.warnings.some((w) => w.message.includes('Confidence'))).toBe(true);
    });
  });

  describe('broken-links rule', () => {
    it('should error when module link points to non-existent file', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'broken-links.aps.md'));

      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.rule === 'broken-links')).toBe(true);
      expect(result.errors.some((e) => e.message.includes('nonexistent'))).toBe(true);
    });
  });

  describe('path-containment rule', () => {
    it('should error when module link escapes the project directory', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'path-containment.aps.md'));

      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.rule === 'path-containment')).toBe(true);
      expect(result.errors.some((e) => e.message.includes('escapes project directory'))).toBe(true);
    });

    it('should be skippable via skipRules', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'path-containment.aps.md'), {
        skipRules: ['path-containment'],
      });

      expect(result.errors.some((e) => e.rule === 'path-containment')).toBe(false);
    });
  });

  describe('duplicate-ids rule', () => {
    it('should error when same task ID appears in multiple modules', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'duplicate-ids-index.aps.md'));

      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.rule === 'duplicate-ids')).toBe(true);
      expect(result.errors.some((e) => e.message.includes('DUP-001'))).toBe(true);
    });
  });

  describe('circular-dependencies rule', () => {
    it('should error when modules have circular dependencies', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'circular-deps-index.aps.md'));

      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.rule === 'circular-dependencies')).toBe(true);
      expect(result.errors.some((e) => e.message.includes('Circular dependency'))).toBe(true);
    });
  });

  describe('scope-mismatch rule (warning)', () => {
    it('should warn when task ID scope prefix does not match module scope', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'scope-mismatch.aps.md'));

      // Should be valid (warnings don't invalidate)
      expect(result.valid).toBe(true);
      expect(result.warnings.some((w) => w.rule === 'scope-mismatch')).toBe(true);
      expect(result.warnings.some((w) => w.message.includes('AUTH-001'))).toBe(true);
      expect(result.warnings.some((w) => w.message.includes('TEST'))).toBe(true);
    });

    it('should not warn when task ID scope prefix matches module scope', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'scope-mismatch.aps.md'));

      // TEST-001 should not generate a scope-mismatch warning
      const testScopeWarnings = result.warnings.filter(
        (w) => w.rule === 'scope-mismatch' && w.message.includes('TEST-001')
      );
      expect(testScopeWarnings).toHaveLength(0);
    });
  });

  describe('skipRules option', () => {
    it('should skip specified rules', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'missing-intent.aps.md'), {
        skipRules: ['task-intent', 'missing-confidence'],
      });

      expect(result.errors.some((e) => e.rule === 'task-intent')).toBe(false);
      expect(result.warnings.some((w) => w.rule === 'missing-confidence')).toBe(false);
    });
  });

  describe('recursive option', () => {
    it('should not validate linked modules when recursive is false', async () => {
      const result = await validatePlanningDoc(join(fixturesDir, 'broken-links.aps.md'), {
        recursive: false,
      });

      // When not recursive, broken links are not checked
      expect(result.errors.some((e) => e.rule === 'broken-links')).toBe(false);
    });
  });

  describe('file-readable rule', () => {
    it('should error when file does not exist', async () => {
      const result = await validatePlanningDoc('/nonexistent/path/file.aps.md');

      expect(result.valid).toBe(false);
      expect(result.errors.some((e) => e.rule === 'file-readable')).toBe(true);
    });
  });
});

describe('formatValidationIssues', () => {
  it('should format issues for display', async () => {
    const result = await validatePlanningDoc(join(fixturesDir, 'missing-intent.aps.md'));
    const formatted = formatValidationIssues(result);

    expect(formatted).toContain('[ERROR]');
    expect(formatted).toContain('Intent');
    expect(formatted).toContain('error(s)');
    expect(formatted).toContain('warning(s)');
  });

  it('should return "No issues found" for valid documents', async () => {
    const result = await validatePlanningDoc(join(fixturesDir, 'valid-leaf.aps.md'));
    // Valid leaf spec might still have warnings (e.g., missing confidence)
    // so we test the "no issues" case with a mock
    expect(result.valid).toBe(true);

    // Test the "no issues" case with a mock result
    const mockResult: ValidationResult = {
      valid: true,
      issues: [],
      errors: [],
      warnings: [],
    };
    const mockFormatted = formatValidationIssues(mockResult);
    expect(mockFormatted).toBe('No issues found.');
  });

  it('should include context in formatted output', async () => {
    const result = await validatePlanningDoc(join(fixturesDir, 'scope-mismatch.aps.md'));
    const output = formatValidationIssues(result);

    expect(output).toContain('Module scope:');
  });

  it('should include line numbers when available', async () => {
    const result = await validatePlanningDoc(join(fixturesDir, 'missing-intent.aps.md'));
    const formatted = formatValidationIssues(result);

    // Should contain file path with line number
    expect(formatted).toMatch(/\.aps\.md:\d+/);
  });
});

describe('ValidationResult structure', () => {
  it('should separate errors and warnings', async () => {
    const result = await validatePlanningDoc(join(fixturesDir, 'scope-mismatch.aps.md'));

    expect(result.issues.length).toBe(result.errors.length + result.warnings.length);
    expect(result.errors.every((e) => e.severity === 'error')).toBe(true);
    expect(result.warnings.every((w) => w.severity === 'warning')).toBe(true);
  });

  it('should mark as valid when only warnings exist', async () => {
    const result = await validatePlanningDoc(join(fixturesDir, 'missing-confidence.aps.md'));

    expect(result.valid).toBe(true);
    expect(result.warnings.length).toBeGreaterThan(0);
    expect(result.errors).toHaveLength(0);
  });

  it('should mark as invalid when errors exist', async () => {
    const result = await validatePlanningDoc(join(fixturesDir, 'missing-intent.aps.md'));

    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
  });

  it('should include path and lineNumber in issues where applicable', async () => {
    const result = await validatePlanningDoc(join(fixturesDir, 'missing-intent.aps.md'));

    const issueWithLocation = result.issues.find((i) => i.path && i.lineNumber);
    expect(issueWithLocation).toBeDefined();
    expect(issueWithLocation?.path).toContain('missing-intent.aps.md');
    expect(typeof issueWithLocation?.lineNumber).toBe('number');
  });
});
