import { describe, it, expect, beforeEach } from 'vitest';
import type { ExplanationContext } from './types.js';
import { clearTemplates, hasTemplate, renderExplanation } from './template-loader.js';
import {
  registerBoundaryTemplates,
  getBoundaryExplanation,
  isArchitectureRule,
} from './boundary-explainer.js';

const mockContext: ExplanationContext = {
  file: 'src/api/handler.ts',
  line: 15,
};

const boundaryContext: ExplanationContext = {
  file: 'src/api/handler.ts',
  line: 15,
  fromFile: 'src/api/handler.ts',
  toFile: 'src/db/queries.ts',
  fromLayer: 'presentation',
  toLayer: 'infrastructure',
};

describe('boundary-explainer', () => {
  beforeEach(() => {
    clearTemplates();
  });

  describe('registerBoundaryTemplates', () => {
    it('registers templates for all architecture rules', () => {
      registerBoundaryTemplates();
      expect(hasTemplate('ARCH-001')).toBe(true);
      expect(hasTemplate('ARCH-002')).toBe(true);
      expect(hasTemplate('ARCH-003')).toBe(true);
      expect(hasTemplate('ARCH-004')).toBe(true);
      expect(hasTemplate('BOUND-001')).toBe(true);
    });
  });

  describe('isArchitectureRule', () => {
    it('returns true for ARCH rules', () => {
      expect(isArchitectureRule('ARCH-001')).toBe(true);
      expect(isArchitectureRule('ARCH-002')).toBe(true);
      expect(isArchitectureRule('ARCH-003')).toBe(true);
      expect(isArchitectureRule('ARCH-004')).toBe(true);
    });

    it('returns true for BOUND rules', () => {
      expect(isArchitectureRule('BOUND-001')).toBe(true);
    });

    it('returns false for AP rules', () => {
      expect(isArchitectureRule('AP-003')).toBe(false);
    });

    it('returns false for unknown rules', () => {
      expect(isArchitectureRule('UNKNOWN-001')).toBe(false);
    });
  });

  describe('getBoundaryExplanation', () => {
    it('returns explanation for ARCH-001 (circular)', () => {
      const explanation = getBoundaryExplanation('ARCH-001', mockContext);
      expect(explanation).not.toBeNull();
      expect(explanation?.ruleId).toBe('ARCH-001');
      expect(explanation?.title).toContain('Circular');
      expect(explanation?.whyItMatters.content).toContain('cycle');
    });

    it('returns explanation for ARCH-002 (orphan)', () => {
      const explanation = getBoundaryExplanation('ARCH-002', mockContext);
      expect(explanation).not.toBeNull();
      expect(explanation?.ruleId).toBe('ARCH-002');
      expect(explanation?.title).toContain('Orphan');
    });

    it('returns explanation for ARCH-003 (layer violation)', () => {
      const explanation = getBoundaryExplanation('ARCH-003', boundaryContext);
      expect(explanation).not.toBeNull();
      expect(explanation?.ruleId).toBe('ARCH-003');
      expect(explanation?.title).toContain('Layer');
      expect(explanation?.whyItMatters.content).toContain('presentation');
      expect(explanation?.whyItMatters.content).toContain('infrastructure');
    });

    it('returns explanation for BOUND-001 (new boundary)', () => {
      const explanation = getBoundaryExplanation('BOUND-001', boundaryContext);
      expect(explanation).not.toBeNull();
      expect(explanation?.ruleId).toBe('BOUND-001');
      expect(explanation?.title).toContain('Boundary');
      expect(explanation?.whyItMatters.content).toContain('NEW');
    });

    it('returns null for unknown rule', () => {
      const explanation = getBoundaryExplanation('UNKNOWN-001', mockContext);
      expect(explanation).toBeNull();
    });

    it('includes layer context in ARCH-003 whyItMatters', () => {
      const explanation = getBoundaryExplanation('ARCH-003', boundaryContext);
      expect(explanation?.whyItMatters.content).toContain('presentation');
      expect(explanation?.whyItMatters.content).toContain('infrastructure');
    });

    it('formats summary with from/to files', () => {
      const explanation = getBoundaryExplanation('BOUND-001', boundaryContext);
      expect(explanation?.summary).toContain('src/api/handler.ts');
      expect(explanation?.summary).toContain('src/db/queries.ts');
      expect(explanation?.summary).toContain('→');
    });

    it('includes suppression syntax', () => {
      const explanation = getBoundaryExplanation('ARCH-003', mockContext);
      expect(explanation?.whenToSuppress.content).toContain('@anvil-ignore');
      expect(explanation?.whenToSuppress.content).toContain('ARCH-003');
    });
  });

  describe('template integration', () => {
    it('registered templates can be rendered', () => {
      registerBoundaryTemplates();
      const explanation = renderExplanation('ARCH-003', boundaryContext);
      expect(explanation).not.toBeNull();
      expect(explanation?.ruleId).toBe('ARCH-003');
    });

    it('includes similar warnings count when provided', () => {
      const contextWithSimilar: ExplanationContext = {
        ...boundaryContext,
        similarCount: 3,
      };
      const explanation = getBoundaryExplanation('ARCH-003', contextWithSimilar);
      expect(explanation?.related?.similarWarnings).toBe(3);
    });
  });
});
