import { describe, it, expect, beforeEach } from 'vitest';
import type { Warning } from '../antipattern/types.js';
import {
  resetExplainService,
  initExplainService,
  explainWarning,
  explainById,
  explainByRule,
  listWarnings,
  isExplainable,
  getExplainableRules,
} from './explain-service.js';

function createMockWarning(overrides: Partial<Warning> = {}): Warning {
  return {
    id: 'AP-003',
    category: 'anti-pattern',
    severity: 'warning',
    confidence: 'high',
    title: 'Explicit any type usage',
    message: 'Using any type',
    explanation: 'This defeats TypeScript',
    suggestion: 'Use unknown instead',
    location: {
      file: 'src/utils/helpers.ts',
      line: 42,
    },
    ...overrides,
  };
}

describe('ExplainService', () => {
  beforeEach(() => {
    resetExplainService();
  });

  describe('initExplainService', () => {
    it('initialises templates', () => {
      initExplainService();
      expect(isExplainable('AP-003')).toBe(true);
      expect(isExplainable('ARCH-001')).toBe(true);
    });

    it('is idempotent', () => {
      initExplainService();
      initExplainService();
      expect(isExplainable('AP-003')).toBe(true);
    });
  });

  describe('explainWarning', () => {
    it('returns explanation for anti-pattern warning', () => {
      const warning = createMockWarning();
      const explanation = explainWarning(warning);

      expect(explanation.ruleId).toBe('AP-003');
      expect(explanation.title).toContain('any');
      expect(explanation.whyItMatters.title).toBe('WHY THIS WARNING EXISTS');
      expect(explanation.howToAddress.title).toBe('HOW TO ADDRESS');
      expect(explanation.whenToSuppress.title).toBe('WHEN TO SUPPRESS');
    });

    it('returns explanation for architecture warning', () => {
      const warning = createMockWarning({
        id: 'ARCH-001',
        category: 'architecture',
        title: 'Circular dependency',
      });
      const explanation = explainWarning(warning);

      expect(explanation.ruleId).toBe('ARCH-001');
      expect(explanation.title).toContain('Circular');
    });

    it('includes similar warnings count when all warnings provided', () => {
      const warnings: Warning[] = [
        createMockWarning({ location: { file: 'src/a.ts', line: 1 } }),
        createMockWarning({ location: { file: 'src/a.ts', line: 10 } }),
        createMockWarning({ location: { file: 'src/b.ts', line: 5 } }),
      ];

      const explanation = explainWarning(warnings[0], warnings);
      expect(explanation.related?.similarWarnings).toBe(1);
    });

    it('returns generic explanation for unknown rule', () => {
      const warning = createMockWarning({ id: 'UNKNOWN-001' as string });
      const explanation = explainWarning(warning);

      expect(explanation.ruleId).toBe('UNKNOWN-001');
      expect(explanation.whyItMatters.content).toContain('potential issue');
    });
  });

  describe('explainById', () => {
    it('finds and explains warning by ID', () => {
      const warnings: Warning[] = [
        createMockWarning({ id: 'AP-001', location: { file: 'a.ts', line: 1 } }),
        createMockWarning({ id: 'AP-003', location: { file: 'b.ts', line: 10 } }),
      ];

      const explanation = explainById('AP-003-b.ts:10', warnings);
      expect(explanation).not.toBeNull();
      expect(explanation?.ruleId).toBe('AP-003');
    });

    it('returns null for non-existent warning ID', () => {
      const warnings: Warning[] = [createMockWarning()];
      expect(explainById('AP-007-x.ts:1', warnings)).toBeNull();
    });

    it('returns null for invalid warning ID format', () => {
      const warnings: Warning[] = [createMockWarning()];
      expect(explainById('invalid', warnings)).toBeNull();
    });
  });

  describe('explainByRule', () => {
    it('returns explanation for known rule', () => {
      const explanation = explainByRule('AP-003', { file: 'test.ts', line: 5 });
      expect(explanation).not.toBeNull();
      expect(explanation?.ruleId).toBe('AP-003');
    });

    it('returns explanation for architecture rule', () => {
      const explanation = explainByRule('ARCH-001');
      expect(explanation).not.toBeNull();
      expect(explanation?.ruleId).toBe('ARCH-001');
    });

    it('returns null for unknown rule', () => {
      expect(explainByRule('UNKNOWN-999')).toBeNull();
    });

    it('uses default context when not provided', () => {
      const explanation = explainByRule('AP-003');
      expect(explanation).not.toBeNull();
    });
  });

  describe('listWarnings', () => {
    it('returns list with warning IDs', () => {
      const warnings: Warning[] = [
        createMockWarning({ id: 'AP-003', location: { file: 'a.ts', line: 1 } }),
        createMockWarning({ id: 'AP-001', location: { file: 'b.ts', line: 10 } }),
      ];

      const list = listWarnings(warnings);
      expect(list).toHaveLength(2);
      expect(list[0].warningId).toBe('AP-003-a.ts:1');
      expect(list[0].ruleId).toBe('AP-003');
      expect(list[1].warningId).toBe('AP-001-b.ts:10');
    });

    it('returns empty array for no warnings', () => {
      expect(listWarnings([])).toEqual([]);
    });
  });

  describe('isExplainable', () => {
    it('returns true for anti-pattern rules', () => {
      expect(isExplainable('AP-001')).toBe(true);
      expect(isExplainable('AP-003')).toBe(true);
      expect(isExplainable('AP-006')).toBe(true);
    });

    it('returns true for architecture rules', () => {
      expect(isExplainable('ARCH-001')).toBe(true);
      expect(isExplainable('ARCH-003')).toBe(true);
      expect(isExplainable('BOUND-001')).toBe(true);
    });

    it('returns false for unknown rules', () => {
      expect(isExplainable('UNKNOWN-001')).toBe(false);
    });
  });

  describe('getExplainableRules', () => {
    it('returns all known rule IDs', () => {
      const rules = getExplainableRules();
      expect(rules).toContain('AP-001');
      expect(rules).toContain('AP-003');
      expect(rules).toContain('ARCH-001');
      expect(rules).toContain('BOUND-001');
      expect(rules.length).toBeGreaterThanOrEqual(12);
    });
  });
});
