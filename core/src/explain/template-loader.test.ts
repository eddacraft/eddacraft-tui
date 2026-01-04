import { describe, it, expect, beforeEach } from 'vitest';
import type { ExplanationTemplate, ExplanationContext } from './types.js';
import {
  registerTemplate,
  getTemplate,
  hasTemplate,
  getRegisteredRuleIds,
  renderExplanation,
  clearTemplates,
  createGenericExplanation,
} from './template-loader.js';

const mockTemplate: ExplanationTemplate = {
  ruleId: 'AP-003',
  render: (context: ExplanationContext) => ({
    ruleId: 'AP-003',
    title: 'Explicit any type',
    summary: `Explicit any type at ${context.file}:${context.line}`,
    whyItMatters: {
      title: 'WHY THIS WARNING EXISTS',
      content: 'The any type disables TypeScript type checking.',
    },
    howToAddress: {
      title: 'HOW TO ADDRESS',
      content: 'Use a proper type or unknown instead.',
    },
    whenToSuppress: {
      title: 'WHEN TO SUPPRESS',
      content: 'Only for third-party library limitations.',
    },
  }),
};

const mockContext: ExplanationContext = {
  file: 'src/utils/helpers.ts',
  line: 42,
  code: 'function parse(data: any) {',
};

describe('template-loader', () => {
  beforeEach(() => {
    clearTemplates();
  });

  describe('registerTemplate', () => {
    it('registers a template', () => {
      registerTemplate(mockTemplate);
      expect(hasTemplate('AP-003')).toBe(true);
    });

    it('overwrites existing template with same ruleId', () => {
      registerTemplate(mockTemplate);
      const newTemplate: ExplanationTemplate = {
        ruleId: 'AP-003',
        render: () => ({
          ruleId: 'AP-003',
          title: 'Updated',
          summary: 'Updated summary',
          whyItMatters: { title: 'WHY', content: 'Updated' },
          howToAddress: { title: 'HOW', content: 'Updated' },
          whenToSuppress: { title: 'WHEN', content: 'Updated' },
        }),
      };
      registerTemplate(newTemplate);
      const result = renderExplanation('AP-003', mockContext);
      expect(result?.title).toBe('Updated');
    });
  });

  describe('getTemplate', () => {
    it('returns registered template', () => {
      registerTemplate(mockTemplate);
      const template = getTemplate('AP-003');
      expect(template).toBeDefined();
      expect(template?.ruleId).toBe('AP-003');
    });

    it('returns undefined for unregistered ruleId', () => {
      expect(getTemplate('AP-999')).toBeUndefined();
    });
  });

  describe('hasTemplate', () => {
    it('returns true for registered template', () => {
      registerTemplate(mockTemplate);
      expect(hasTemplate('AP-003')).toBe(true);
    });

    it('returns false for unregistered template', () => {
      expect(hasTemplate('AP-999')).toBe(false);
    });
  });

  describe('getRegisteredRuleIds', () => {
    it('returns empty array when no templates registered', () => {
      expect(getRegisteredRuleIds()).toEqual([]);
    });

    it('returns all registered rule IDs', () => {
      registerTemplate(mockTemplate);
      registerTemplate({ ...mockTemplate, ruleId: 'AP-001' });
      const ids = getRegisteredRuleIds();
      expect(ids).toHaveLength(2);
      expect(ids).toContain('AP-003');
      expect(ids).toContain('AP-001');
    });
  });

  describe('renderExplanation', () => {
    it('renders explanation using registered template', () => {
      registerTemplate(mockTemplate);
      const explanation = renderExplanation('AP-003', mockContext);
      expect(explanation).toBeDefined();
      expect(explanation?.ruleId).toBe('AP-003');
      expect(explanation?.summary).toContain('src/utils/helpers.ts:42');
    });

    it('returns null for unregistered ruleId', () => {
      expect(renderExplanation('AP-999', mockContext)).toBeNull();
    });
  });

  describe('clearTemplates', () => {
    it('removes all registered templates', () => {
      registerTemplate(mockTemplate);
      expect(hasTemplate('AP-003')).toBe(true);
      clearTemplates();
      expect(hasTemplate('AP-003')).toBe(false);
    });
  });

  describe('createGenericExplanation', () => {
    it('creates a generic explanation when no template exists', () => {
      const explanation = createGenericExplanation('AP-999', 'Unknown Warning', mockContext);
      expect(explanation.ruleId).toBe('AP-999');
      expect(explanation.title).toBe('Unknown Warning');
      expect(explanation.summary).toContain('src/utils/helpers.ts:42');
      expect(explanation.whyItMatters.title).toBe('WHY THIS WARNING EXISTS');
      expect(explanation.whenToSuppress.content).toContain('@anvil-ignore AP-999');
    });
  });
});
