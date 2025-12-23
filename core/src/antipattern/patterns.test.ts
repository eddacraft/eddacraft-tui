import { describe, expect, it } from 'vitest';

import {
  PATTERNS,
  getDefaultPatterns,
  getEnabledPatterns,
  getPattern,
  getPatternIds,
  getPatternsByCategory,
  isValidPatternId,
} from './patterns.js';

describe('Pattern Catalogue', () => {
  describe('PATTERNS', () => {
    it('should contain 7 patterns', () => {
      expect(PATTERNS).toHaveLength(7);
    });

    it('should have unique IDs', () => {
      const ids = PATTERNS.map((p) => p.id);
      expect(new Set(ids).size).toBe(ids.length);
    });

    it('should follow AP-XXX ID format', () => {
      for (const pattern of PATTERNS) {
        expect(pattern.id).toMatch(/^AP-\d{3}$/);
      }
    });

    it('should have all required fields', () => {
      for (const pattern of PATTERNS) {
        expect(pattern.name).toBeTruthy();
        expect(pattern.category).toBeTruthy();
        expect(pattern.severity).toBeTruthy();
        expect(pattern.confidence).toBeTruthy();
        expect(pattern.detection).toBeTruthy();
        expect(pattern.title).toBeTruthy();
        expect(pattern.explanation).toBeTruthy();
        expect(pattern.suggestion).toBeTruthy();
      }
    });
  });

  describe('getPattern', () => {
    it('should return pattern by ID', () => {
      const pattern = getPattern('AP-001');
      expect(pattern).toBeDefined();
      expect(pattern?.name).toBe('Broad eslint-disable');
    });

    it('should return undefined for unknown ID', () => {
      expect(getPattern('AP-999')).toBeUndefined();
    });

    it('should return undefined for invalid ID format', () => {
      expect(getPattern('invalid')).toBeUndefined();
    });
  });

  describe('getPatternsByCategory', () => {
    it('should return escape-hatch patterns', () => {
      const patterns = getPatternsByCategory('escape-hatch');
      expect(patterns).toHaveLength(2);
      expect(patterns.map((p) => p.id)).toEqual(['AP-001', 'AP-002']);
    });

    it('should return type-safety patterns', () => {
      const patterns = getPatternsByCategory('type-safety');
      expect(patterns).toHaveLength(3);
      expect(patterns.map((p) => p.id)).toEqual(['AP-003', 'AP-004', 'AP-005']);
    });

    it('should return error-handling patterns', () => {
      const patterns = getPatternsByCategory('error-handling');
      expect(patterns).toHaveLength(1);
      expect(patterns.map((p) => p.id)).toEqual(['AP-006']);
    });

    it('should return code-quality patterns', () => {
      const patterns = getPatternsByCategory('code-quality');
      expect(patterns).toHaveLength(1);
      expect(patterns.map((p) => p.id)).toEqual(['AP-007']);
    });
  });

  describe('getEnabledPatterns', () => {
    it('should return all enabled patterns', () => {
      const patterns = getEnabledPatterns();
      expect(patterns.every((p) => p.enabled)).toBe(true);
      expect(patterns).toHaveLength(7);
    });
  });

  describe('getDefaultPatterns', () => {
    it('should exclude opt-in patterns', () => {
      const patterns = getDefaultPatterns();
      expect(patterns.every((p) => !p.optIn)).toBe(true);
    });

    it('should include non-opt-in patterns', () => {
      const patterns = getDefaultPatterns();
      const ids = patterns.map((p) => p.id);
      expect(ids).toContain('AP-001');
      expect(ids).toContain('AP-003');
      expect(ids).toContain('AP-004');
      expect(ids).toContain('AP-006');
    });

    it('should exclude opt-in patterns', () => {
      const patterns = getDefaultPatterns();
      const ids = patterns.map((p) => p.id);
      expect(ids).not.toContain('AP-002');
      expect(ids).not.toContain('AP-005');
      expect(ids).not.toContain('AP-007');
    });
  });

  describe('getPatternIds', () => {
    it('should return all pattern IDs', () => {
      const ids = getPatternIds();
      expect(ids).toEqual(['AP-001', 'AP-002', 'AP-003', 'AP-004', 'AP-005', 'AP-006', 'AP-007']);
    });
  });

  describe('isValidPatternId', () => {
    it('should return true for valid IDs', () => {
      expect(isValidPatternId('AP-001')).toBe(true);
      expect(isValidPatternId('AP-007')).toBe(true);
    });

    it('should return false for invalid IDs', () => {
      expect(isValidPatternId('AP-999')).toBe(false);
      expect(isValidPatternId('invalid')).toBe(false);
      expect(isValidPatternId('')).toBe(false);
    });
  });

  describe('Pattern regex detection', () => {
    describe('AP-001: Broad eslint-disable', () => {
      const pattern = getPattern('AP-001')!;
      const regex = new RegExp((pattern.detection as { pattern: string }).pattern);

      it('should match /* eslint-disable */', () => {
        expect(regex.test('/* eslint-disable */')).toBe(true);
      });

      it('should match // eslint-disable at end of line', () => {
        expect(regex.test('// eslint-disable')).toBe(true);
      });

      it('should NOT match eslint-disable-next-line', () => {
        expect(regex.test('// eslint-disable-next-line')).toBe(false);
      });

      it('should NOT match eslint-disable-line', () => {
        expect(regex.test('// eslint-disable-line')).toBe(false);
      });
    });

    describe('AP-002: Rule-specific eslint-disable', () => {
      const pattern = getPattern('AP-002')!;
      const regex = new RegExp((pattern.detection as { pattern: string }).pattern);

      it('should match eslint-disable with rule', () => {
        expect(regex.test('eslint-disable @typescript-eslint/no-explicit-any')).toBe(true);
      });

      it('should match eslint-disable-next-line with rule', () => {
        expect(regex.test('eslint-disable-next-line no-console')).toBe(true);
      });

      it('should match eslint-disable-line with rule', () => {
        expect(regex.test('eslint-disable-line no-unused-vars')).toBe(true);
      });
    });

    describe('AP-003: Explicit any type', () => {
      const pattern = getPattern('AP-003')!;
      const regex = new RegExp((pattern.detection as { pattern: string }).pattern);

      it('should match : any', () => {
        expect(regex.test('const x: any = 1')).toBe(true);
      });

      it('should match as any', () => {
        expect(regex.test('foo as any')).toBe(true);
      });

      it('should match <any>', () => {
        expect(regex.test('Array<any>')).toBe(true);
      });

      it('should NOT match "any" in strings', () => {
        expect(regex.test('"any value"')).toBe(false);
      });

      it('should NOT match words containing "any"', () => {
        expect(regex.test('company')).toBe(false);
        expect(regex.test('many')).toBe(false);
      });
    });

    describe('AP-004: @ts-ignore', () => {
      const pattern = getPattern('AP-004')!;
      const regex = new RegExp((pattern.detection as { pattern: string }).pattern);

      it('should match @ts-ignore', () => {
        expect(regex.test('// @ts-ignore')).toBe(true);
      });
    });

    describe('AP-005: @ts-expect-error', () => {
      const pattern = getPattern('AP-005')!;
      const regex = new RegExp((pattern.detection as { pattern: string }).pattern);

      it('should match @ts-expect-error', () => {
        expect(regex.test('// @ts-expect-error')).toBe(true);
      });
    });

    describe('AP-006: Empty catch', () => {
      const pattern = getPattern('AP-006')!;
      const regex = new RegExp((pattern.detection as { pattern: string }).pattern);

      it('should match empty catch block', () => {
        expect(regex.test('catch (e) {}')).toBe(true);
      });

      it('should match catch with only whitespace', () => {
        expect(regex.test('catch (e) {   }')).toBe(true);
      });

      it('should match catch with only comment', () => {
        expect(regex.test('catch (e) { // ignore }')).toBe(true);
      });
    });

    describe('AP-007: Console in production', () => {
      const pattern = getPattern('AP-007')!;
      const regex = new RegExp((pattern.detection as { pattern: string }).pattern);

      it('should match console.log', () => {
        expect(regex.test('console.log("test")')).toBe(true);
      });

      it('should match console.warn', () => {
        expect(regex.test('console.warn("warning")')).toBe(true);
      });

      it('should match console.info', () => {
        expect(regex.test('console.info("info")')).toBe(true);
      });

      it('should match console.debug', () => {
        expect(regex.test('console.debug("debug")')).toBe(true);
      });

      it('should NOT match console.error', () => {
        expect(regex.test('console.error("error")')).toBe(false);
      });
    });
  });
});
