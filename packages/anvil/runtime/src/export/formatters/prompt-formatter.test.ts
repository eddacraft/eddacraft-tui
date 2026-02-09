/**
 * Tests for prompt formatter
 */

import { describe, it, expect } from 'vitest';
import { PromptFormatter, formatAsPrompt, formatAsConcisePrompt } from './prompt-formatter.js';
import type { Constraints } from '../constraint-collector.js';

describe('PromptFormatter', () => {
  const sampleConstraints: Constraints = {
    boundaries: [
      {
        name: 'no-ui-to-domain',
        from: 'ui',
        to: 'domain',
        message: 'UI layer must not directly access domain',
        severity: 'error',
      },
      {
        name: 'no-domain-to-ui',
        from: 'domain',
        to: 'ui',
        message: 'Domain should not depend on UI',
        severity: 'warning',
      },
    ],
    layers: [
      {
        name: 'ui',
        patterns: ['src/ui/**'],
        dependsOn: ['application'],
        description: 'User interface layer',
      },
      {
        name: 'application',
        patterns: ['src/app/**'],
        dependsOn: ['domain'],
        description: 'Application services',
      },
      {
        name: 'domain',
        patterns: ['src/domain/**'],
        dependsOn: [],
        description: 'Domain models',
      },
    ],
    antiPatterns: [
      {
        id: 'AP-001',
        name: 'Broad eslint-disable',
        category: 'escape-hatch',
        explanation: 'Disabling all ESLint rules hides legitimate issues',
        suggestion: 'Disable specific rules instead',
        severity: 'warning',
        enabled: true,
      },
      {
        id: 'AP-003',
        name: 'Explicit any type',
        category: 'type-safety',
        explanation: 'Using any defeats TypeScript type checking',
        suggestion: 'Use unknown or define proper types',
        severity: 'warning',
        enabled: true,
      },
    ],
    conventions: [
      {
        category: 'spelling',
        description: 'Use UK English spelling',
        examples: ['organised (not organized)', 'behaviour (not behavior)'],
      },
      {
        category: 'imports',
        description: 'ESM imports require .js extensions',
        examples: ["import { foo } from './bar.js'"],
      },
    ],
    suppressions: [
      {
        patternId: 'AP-003',
        file: 'src/legacy.ts',
        scope: 'file',
        reason: 'Legacy code not yet migrated',
      },
      {
        patternId: 'AP-001',
        file: 'src/api.ts',
        scope: 'statement',
        reason: 'Third-party integration',
        expiresAt: '2025-06-01T00:00:00.000Z',
      },
    ],
    metadata: {
      collectedAt: '2024-01-15T10:30:00.000Z',
      workspaceRoot: '/test/workspace',
      hasBaseline: true,
    },
  };

  describe('format', () => {
    it('should format complete constraints', () => {
      const formatter = new PromptFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('**Architecture Boundaries**');
      expect(result).toContain('**Layer Definitions**');
      expect(result).toContain('**Forbidden Anti-patterns**');
      expect(result).toContain('**Project Conventions**');
      expect(result).toContain('**Active Suppressions**');
    });

    it('should include opening instruction', () => {
      const formatter = new PromptFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('architecture boundaries');
      expect(result).toContain('When generating or modifying code');
    });

    it('should use concise opening when configured', () => {
      const formatter = new PromptFormatter({ concise: true });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('When working on this codebase');
      expect(result).toContain('Flag violations');
      expect(result).not.toContain('When generating or modifying code:');
    });
  });

  describe('boundaries formatting', () => {
    it('should format boundaries in detail', () => {
      const formatter = new PromptFormatter({ concise: false });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('**Architecture Boundaries**');
      expect(result).toContain('no-ui-to-domain');
      expect(result).toContain('Layer "ui" must not depend on "domain"');
      expect(result).toContain('UI layer must not directly access domain');
      expect(result).toContain('Severity: error');
    });

    it('should format boundaries concisely', () => {
      const formatter = new PromptFormatter({ concise: true });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('Layer "ui" must not depend on "domain"');
      expect(result).not.toContain('Severity:');
      expect(result).not.toContain('no-ui-to-domain');
    });

    it('should exclude boundaries when configured', () => {
      const formatter = new PromptFormatter({ includeBoundaries: false });
      const result = formatter.format(sampleConstraints);

      expect(result).not.toContain('**Architecture Boundaries**');
      expect(result).not.toContain('no-ui-to-domain');
    });

    it('should skip boundaries section when empty', () => {
      const emptyConstraints: Constraints = {
        ...sampleConstraints,
        boundaries: [],
      };

      const formatter = new PromptFormatter();
      const result = formatter.format(emptyConstraints);

      expect(result).not.toContain('**Architecture Boundaries**');
    });
  });

  describe('layers formatting', () => {
    it('should format layers in detail', () => {
      const formatter = new PromptFormatter({ concise: false });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('**Layer Definitions**');
      expect(result).toContain('**ui**');
      expect(result).toContain('User interface layer');
      expect(result).toContain('Files: src/ui/**');
      expect(result).toContain('Allowed dependencies: application');
    });

    it('should format layers concisely', () => {
      const formatter = new PromptFormatter({ concise: true });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('**ui**: src/ui/**');
      expect(result).toContain('Can depend on: application');
      expect(result).not.toContain('User interface layer');
    });

    it('should handle layers with no dependencies', () => {
      const formatter = new PromptFormatter({ concise: false });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('**domain**');
      expect(result).toContain('No dependencies (leaf layer)');
    });

    it('should exclude layers when configured', () => {
      const formatter = new PromptFormatter({ includeLayers: false });
      const result = formatter.format(sampleConstraints);

      expect(result).not.toContain('**Layer Definitions**');
      expect(result).not.toContain('**ui**');
    });

    it('should skip layers section when empty', () => {
      const emptyConstraints: Constraints = {
        ...sampleConstraints,
        layers: [],
      };

      const formatter = new PromptFormatter();
      const result = formatter.format(emptyConstraints);

      expect(result).not.toContain('**Layer Definitions**');
    });
  });

  describe('anti-patterns formatting', () => {
    it('should format anti-patterns in detail', () => {
      const formatter = new PromptFormatter({ concise: false });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('**Forbidden Anti-patterns**');
      expect(result).toContain('Escape Hatch:');
      expect(result).toContain('**Broad eslint-disable** (AP-001)');
      expect(result).toContain('Problem: Disabling all ESLint rules');
      expect(result).toContain('Instead: Disable specific rules');
    });

    it('should format anti-patterns concisely', () => {
      const formatter = new PromptFormatter({ concise: true });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('Broad eslint-disable: Disable specific rules');
      expect(result).not.toContain('Problem:');
      expect(result).not.toContain('(AP-001)');
    });

    it('should group anti-patterns by category', () => {
      const formatter = new PromptFormatter({ concise: false });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('Escape Hatch:');
      expect(result).toContain('Type Safety:');
    });

    it('should exclude anti-patterns when configured', () => {
      const formatter = new PromptFormatter({ includeAntiPatterns: false });
      const result = formatter.format(sampleConstraints);

      expect(result).not.toContain('**Forbidden Anti-patterns**');
      expect(result).not.toContain('Broad eslint-disable');
    });

    it('should skip anti-patterns section when empty', () => {
      const emptyConstraints: Constraints = {
        ...sampleConstraints,
        antiPatterns: [],
      };

      const formatter = new PromptFormatter();
      const result = formatter.format(emptyConstraints);

      expect(result).not.toContain('**Forbidden Anti-patterns**');
    });
  });

  describe('conventions formatting', () => {
    it('should format conventions in detail', () => {
      const formatter = new PromptFormatter({ concise: false });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('**Project Conventions**');
      expect(result).toContain('**Spelling**: Use UK English spelling');
      expect(result).toContain('• organised (not organized)');
      expect(result).toContain('• behaviour (not behavior)');
    });

    it('should format conventions concisely', () => {
      const formatter = new PromptFormatter({ concise: true });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('**Spelling**: Use UK English spelling');
      expect(result).not.toContain('• organised');
    });

    it('should handle conventions without examples', () => {
      const constraintsNoExamples: Constraints = {
        ...sampleConstraints,
        conventions: [
          {
            category: 'test',
            description: 'Test convention',
          },
        ],
      };

      const formatter = new PromptFormatter({ concise: false });
      const result = formatter.format(constraintsNoExamples);

      expect(result).toContain('**Test**: Test convention');
      expect(result).not.toContain('•');
    });

    it('should exclude conventions when configured', () => {
      const formatter = new PromptFormatter({ includeConventions: false });
      const result = formatter.format(sampleConstraints);

      expect(result).not.toContain('**Project Conventions**');
      expect(result).not.toContain('Spelling');
    });

    it('should skip conventions section when empty', () => {
      const emptyConstraints: Constraints = {
        ...sampleConstraints,
        conventions: [],
      };

      const formatter = new PromptFormatter();
      const result = formatter.format(emptyConstraints);

      expect(result).not.toContain('**Project Conventions**');
    });
  });

  describe('minimal constraints', () => {
    it('should handle constraints with no data', () => {
      const minimalConstraints: Constraints = {
        boundaries: [],
        layers: [],
        antiPatterns: [],
        conventions: [],
        suppressions: [],
        metadata: {
          collectedAt: '2024-01-15T10:30:00.000Z',
          workspaceRoot: '/test',
          hasBaseline: false,
        },
      };

      const formatter = new PromptFormatter();
      const result = formatter.format(minimalConstraints);

      expect(result).toContain('architecture boundaries');
      expect(result).not.toContain('**Architecture Boundaries**');
      expect(result).not.toContain('**Layer Definitions**');
      expect(result).not.toContain('**Forbidden Anti-patterns**');
      expect(result).not.toContain('**Project Conventions**');
      expect(result).not.toContain('**Active Suppressions**');
    });
  });

  describe('formatAsPrompt', () => {
    it('should format with default options', () => {
      const result = formatAsPrompt(sampleConstraints);

      expect(result).toContain('**Architecture Boundaries**');
      expect(result).toContain('**Layer Definitions**');
      expect(result).toContain('When generating or modifying code');
    });
  });

  describe('formatAsConcisePrompt', () => {
    it('should format in concise mode', () => {
      const result = formatAsConcisePrompt(sampleConstraints);

      expect(result).toContain('When working on this codebase');
      expect(result).toContain('Layer "ui" must not depend on "domain"');
      expect(result).not.toContain('Severity:');
      expect(result).not.toContain('Problem:');
    });
  });

  describe('category name formatting', () => {
    it('should format kebab-case category names', () => {
      const formatter = new PromptFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('Escape Hatch');
      expect(result).toContain('Type Safety');
    });
  });

  describe('section separation', () => {
    it('should separate sections with double newlines', () => {
      const formatter = new PromptFormatter();
      const result = formatter.format(sampleConstraints);

      // Should have clear separation between sections
      expect(result).toMatch(/\n\n.*Architecture Boundaries/);
      expect(result).toMatch(/\n\n.*Layer Definitions/);
    });
  });

  describe('suppressions formatting', () => {
    it('should format suppressions in detail', () => {
      const formatter = new PromptFormatter({ concise: false });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('**Active Suppressions**');
      expect(result).toContain('AP-003:');
      expect(result).toContain('**src/legacy.ts**');
      expect(result).toContain('Legacy code not yet migrated');
    });

    it('should format suppressions concisely', () => {
      const formatter = new PromptFormatter({ concise: true });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('**Active Suppressions**');
      expect(result).toContain('AP-003: suppressed in src/legacy.ts');
    });

    it('should include expiry date in verbose mode', () => {
      const formatter = new PromptFormatter({ concise: false });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('Expires:');
    });

    it('should exclude suppressions when configured', () => {
      const formatter = new PromptFormatter({ includeSuppressions: false });
      const result = formatter.format(sampleConstraints);

      expect(result).not.toContain('**Active Suppressions**');
    });

    it('should skip suppressions section when empty', () => {
      const emptyConstraints: Constraints = {
        ...sampleConstraints,
        suppressions: [],
      };

      const formatter = new PromptFormatter();
      const result = formatter.format(emptyConstraints);

      expect(result).not.toContain('**Active Suppressions**');
    });
  });
});
