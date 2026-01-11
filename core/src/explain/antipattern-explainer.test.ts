import { describe, it, expect, beforeEach } from 'vitest';
import type { ExplanationContext } from './types.js';
import { clearTemplates, hasTemplate, renderExplanation } from './template-loader.js';
import {
  registerAntiPatternTemplates,
  getAntiPatternExplanation,
} from './antipattern-explainer.js';

const mockContext: ExplanationContext = {
  file: 'src/utils/helpers.ts',
  line: 42,
  code: 'function parse(data: any) {',
};

describe('antipattern-explainer', () => {
  beforeEach(() => {
    clearTemplates();
  });

  describe('registerAntiPatternTemplates', () => {
    it('registers templates for all built-in patterns', () => {
      registerAntiPatternTemplates();
      expect(hasTemplate('AP-001')).toBe(true);
      expect(hasTemplate('AP-002')).toBe(true);
      expect(hasTemplate('AP-003')).toBe(true);
      expect(hasTemplate('AP-004')).toBe(true);
      expect(hasTemplate('AP-005')).toBe(true);
      expect(hasTemplate('AP-006')).toBe(true);
      expect(hasTemplate('AP-007')).toBe(true);
    });
  });

  describe('getAntiPatternExplanation', () => {
    it('returns explanation for AP-001', () => {
      const explanation = getAntiPatternExplanation('AP-001', mockContext);
      expect(explanation).not.toBeNull();
      expect(explanation?.ruleId).toBe('AP-001');
      expect(explanation?.title).toContain('eslint-disable');
      expect(explanation?.whyItMatters.content).toContain('linting');
    });

    it('returns explanation for AP-003 with detailed content', () => {
      const explanation = getAntiPatternExplanation('AP-003', mockContext);
      expect(explanation).not.toBeNull();
      expect(explanation?.ruleId).toBe('AP-003');
      expect(explanation?.title).toContain('any');
      expect(explanation?.whyItMatters.content).toContain('type checking');
      expect(explanation?.howToAddress.content).toContain('unknown');
    });

    it('returns explanation for AP-004', () => {
      const explanation = getAntiPatternExplanation('AP-004', mockContext);
      expect(explanation).not.toBeNull();
      expect(explanation?.ruleId).toBe('AP-004');
      expect(explanation?.title).toContain('ts-ignore');
    });

    it('returns explanation for AP-006', () => {
      const explanation = getAntiPatternExplanation('AP-006', mockContext);
      expect(explanation).not.toBeNull();
      expect(explanation?.ruleId).toBe('AP-006');
      expect(explanation?.title).toContain('catch');
      expect(explanation?.howToAddress.content).toContain('log');
    });

    it('returns null for unknown pattern', () => {
      const explanation = getAntiPatternExplanation('AP-999', mockContext);
      expect(explanation).toBeNull();
    });

    it('includes suppression syntax in when-to-suppress', () => {
      const explanation = getAntiPatternExplanation('AP-003', mockContext);
      expect(explanation?.whenToSuppress.content).toContain('@anvil-ignore');
      expect(explanation?.whenToSuppress.content).toContain('AP-003');
    });

    it('includes file location in summary', () => {
      const explanation = getAntiPatternExplanation('AP-003', mockContext);
      expect(explanation?.summary).toContain('src/utils/helpers.ts:42');
    });
  });

  describe('template integration', () => {
    it('registered templates can be rendered', () => {
      registerAntiPatternTemplates();
      const explanation = renderExplanation('AP-003', mockContext);
      expect(explanation).not.toBeNull();
      expect(explanation?.ruleId).toBe('AP-003');
    });

    it('includes similar warnings count when provided', () => {
      const contextWithSimilar: ExplanationContext = {
        ...mockContext,
        similarCount: 5,
      };
      const explanation = getAntiPatternExplanation('AP-003', contextWithSimilar);
      expect(explanation?.related?.similarWarnings).toBe(5);
    });
  });
});
