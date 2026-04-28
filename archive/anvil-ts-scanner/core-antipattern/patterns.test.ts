import { describe, expect, it } from 'vitest';

import {
  PATTERNS,
  getDefaultPatterns,
  getEnabledPatterns,
  getPattern,
  getPatternIds,
  getPatternsByCategory,
  getPatternsByFamily,
  isValidPatternId,
} from './patterns.js';

// Post-ANVFMT-015 snapshot — the compiled `.anvil` registry is the sole
// source: AP-001..AP-007, GS-001, RL-001..RL-006, DD-001..DD-004. Legacy
// HTML/CSS TS patterns were retired in ANVFMT-014/015 (see D-002).
const EXPECTED_PATTERN_COUNT = 18;

describe('Pattern Catalogue', () => {
  describe('PATTERNS', () => {
    it('should contain the full registry catalogue', () => {
      expect(PATTERNS).toHaveLength(EXPECTED_PATTERN_COUNT);
    });

    it('should have unique IDs', () => {
      const ids = PATTERNS.map((p) => p.id);
      expect(new Set(ids).size).toBe(ids.length);
    });

    it('should follow <PREFIX>-<NNN> ID format', () => {
      for (const pattern of PATTERNS) {
        expect(pattern.id).toMatch(/^[A-Z]{2,5}-\d{3}$/);
      }
    });

    it('should have all required fields with valid values', () => {
      const validCategories = [
        'escape-hatch',
        'error-handling',
        'code-quality',
        'type-safety',
        'type-evasion',
        'accountability',
        'deferred-debt',
      ];
      const validSeverities = ['error', 'warning', 'info'];
      const validConfidences = ['high', 'medium', 'low'];

      for (const pattern of PATTERNS) {
        expect(typeof pattern.name).toBe('string');
        expect(pattern.name.length).toBeGreaterThan(0);
        expect(validCategories).toContain(pattern.category);
        expect(validSeverities).toContain(pattern.severity);
        expect(validConfidences).toContain(pattern.confidence);
        expect(pattern.detection).toHaveProperty('type');
        expect(typeof pattern.title).toBe('string');
        expect(pattern.title.length).toBeGreaterThan(0);
        expect(typeof pattern.explanation).toBe('string');
        expect(pattern.explanation.length).toBeGreaterThan(0);
        expect(typeof pattern.suggestion).toBe('string');
        expect(pattern.suggestion.length).toBeGreaterThan(0);
      }
    });

    it('should have nudge text on every pattern', () => {
      for (const pattern of PATTERNS) {
        expect(pattern.nudge, `${pattern.id} missing nudge`).toBeDefined();
        expect(typeof pattern.nudge).toBe('string');
        expect(pattern.nudge!.length).toBeGreaterThan(0);
      }
    });

    it('should have family provenance on every pattern', () => {
      for (const pattern of PATTERNS) {
        expect(pattern.family, `${pattern.id} missing family`).toBeDefined();
        expect(pattern.definitionRef, `${pattern.id} missing definitionRef`).toBeDefined();
        expect(pattern.spectrumPosition, `${pattern.id} missing spectrumPosition`).toBeDefined();
      }
    });
  });

  describe('getPattern', () => {
    it('should return pattern by ID', () => {
      const pattern = getPattern('AP-001');
      expect(pattern).toBeDefined();
      expect(pattern?.title).toBe('Broad eslint-disable added');
    });

    it('should return undefined for unknown ID', () => {
      expect(getPattern('AP-999')).toBeUndefined();
    });

    it('should return undefined for invalid ID format', () => {
      expect(getPattern('invalid')).toBeUndefined();
    });

    it('should resolve the new GS-001 non-null assertion rule', () => {
      const pattern = getPattern('GS-001');
      expect(pattern).toBeDefined();
      expect(pattern?.family).toBe('guardrail-suppression');
    });
  });

  describe('getPatternsByCategory', () => {
    it('should return escape-hatch patterns (guardrail-suppression family)', () => {
      const patterns = getPatternsByCategory('escape-hatch');
      // AP-001, AP-002, AP-004, AP-005, GS-001 — sorted by id
      expect(patterns.map((p) => p.id)).toEqual(['AP-001', 'AP-002', 'AP-004', 'AP-005', 'GS-001']);
    });

    it('should return type-evasion patterns', () => {
      const patterns = getPatternsByCategory('type-evasion');
      expect(patterns.map((p) => p.id)).toEqual(['AP-003']);
    });

    it('should return error-handling patterns (error-visibility family)', () => {
      const patterns = getPatternsByCategory('error-handling');
      expect(patterns.map((p) => p.id)).toEqual(['AP-006', 'AP-007']);
    });

    it('should return accountability patterns (responsibility-laundering family)', () => {
      const patterns = getPatternsByCategory('accountability');
      expect(patterns.map((p) => p.id)).toEqual([
        'RL-001',
        'RL-002',
        'RL-003',
        'RL-004',
        'RL-005',
        'RL-006',
      ]);
    });

    it('should return deferred-debt patterns', () => {
      const patterns = getPatternsByCategory('deferred-debt');
      expect(patterns.map((p) => p.id)).toEqual(['DD-001', 'DD-002', 'DD-003', 'DD-004']);
    });
  });

  describe('getPatternsByFamily', () => {
    it('should return all guardrail-suppression rules', () => {
      const patterns = getPatternsByFamily('guardrail-suppression');
      expect(patterns.map((p) => p.id)).toEqual(['AP-001', 'AP-002', 'AP-004', 'AP-005', 'GS-001']);
    });

    it('should return [] for non-existent family', () => {
      expect(getPatternsByFamily('nonexistent')).toEqual([]);
    });
  });

  describe('getEnabledPatterns', () => {
    it('should return all enabled patterns', () => {
      const patterns = getEnabledPatterns();
      expect(patterns.every((p) => p.enabled)).toBe(true);
      expect(patterns).toHaveLength(EXPECTED_PATTERN_COUNT);
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
      expect(ids).toContain('GS-001');
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
    it('should return all pattern IDs in catalogue order', () => {
      const ids = getPatternIds();
      // Registry patterns are byte-sorted at compile time.
      expect(ids).toEqual([
        'AP-001',
        'AP-002',
        'AP-003',
        'AP-004',
        'AP-005',
        'AP-006',
        'AP-007',
        'DD-001',
        'DD-002',
        'DD-003',
        'DD-004',
        'GS-001',
        'RL-001',
        'RL-002',
        'RL-003',
        'RL-004',
        'RL-005',
        'RL-006',
      ]);
    });
  });

  describe('isValidPatternId', () => {
    it('should return true for valid IDs', () => {
      expect(isValidPatternId('AP-001')).toBe(true);
      expect(isValidPatternId('AP-007')).toBe(true);
      expect(isValidPatternId('GS-001')).toBe(true);
      expect(isValidPatternId('RL-003')).toBe(true);
      expect(isValidPatternId('DD-002')).toBe(true);
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
