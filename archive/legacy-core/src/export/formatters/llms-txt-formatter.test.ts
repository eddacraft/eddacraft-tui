/**
 * Tests for llms.txt formatter
 */

import { describe, it, expect } from 'vitest';
import {
  LlmsTxtFormatter,
  formatAsLlmsTxt,
  formatAsLlmsTxtWithoutMetadata,
} from './llms-txt-formatter.js';
import type { Constraints } from '../constraint-collector.js';

describe('LlmsTxtFormatter', () => {
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
    metadata: {
      collectedAt: '2024-01-15T10:30:00.000Z',
      workspaceRoot: '/test/workspace',
      hasBaseline: true,
    },
  };

  describe('format', () => {
    it('should format complete constraints', () => {
      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('# Anvil Architecture Constraints');
      expect(result).toContain('## Boundary Rules');
      expect(result).toContain('## Layer Definitions');
      expect(result).toContain('## Anti-patterns (Blocked)');
      expect(result).toContain('## Conventions');
    });

    it('should include title', () => {
      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toMatch(/^# Anvil Architecture Constraints/);
    });

    it('should format metadata when included', () => {
      const formatter = new LlmsTxtFormatter({ includeMetadata: true });
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('**Generated:**');
      expect(result).toContain('**Workspace:**');
      expect(result).toContain('**Has Baseline:** Yes');
    });

    it('should exclude metadata when configured', () => {
      const formatter = new LlmsTxtFormatter({ includeMetadata: false });
      const result = formatter.format(sampleConstraints);

      expect(result).not.toContain('**Generated:**');
      expect(result).not.toContain('**Workspace:**');
    });
  });

  describe('boundary formatting', () => {
    it('should format boundary rules', () => {
      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('## Boundary Rules');
      expect(result).toContain('no-ui-to-domain');
      expect(result).toContain('From: `ui`');
      expect(result).toContain('To: `domain`');
      expect(result).toContain('UI layer must not directly access domain');
    });

    it('should include severity emojis for boundaries', () => {
      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('🚫'); // error
      expect(result).toContain('⚠️'); // warning
    });

    it('should exclude boundaries when configured', () => {
      const formatter = new LlmsTxtFormatter({ includeBoundaries: false });
      const result = formatter.format(sampleConstraints);

      expect(result).not.toContain('## Boundary Rules');
      expect(result).not.toContain('no-ui-to-domain');
    });

    it('should skip boundaries section when empty', () => {
      const emptyConstraints: Constraints = {
        ...sampleConstraints,
        boundaries: [],
      };

      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(emptyConstraints);

      expect(result).not.toContain('## Boundary Rules');
    });
  });

  describe('layer formatting', () => {
    it('should format layer definitions', () => {
      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('## Layer Definitions');
      expect(result).toContain('### ui');
      expect(result).toContain('User interface layer');
      expect(result).toContain('**Patterns:**');
      expect(result).toContain('`src/ui/**`');
      expect(result).toContain('**Can depend on:**');
      expect(result).toContain('application');
    });

    it('should handle layers with no dependencies', () => {
      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('### domain');
      expect(result).toContain('**Dependencies:** None (leaf layer)');
    });

    it('should exclude layers when configured', () => {
      const formatter = new LlmsTxtFormatter({ includeLayers: false });
      const result = formatter.format(sampleConstraints);

      expect(result).not.toContain('## Layer Definitions');
      expect(result).not.toContain('### ui');
    });

    it('should skip layers section when empty', () => {
      const emptyConstraints: Constraints = {
        ...sampleConstraints,
        layers: [],
      };

      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(emptyConstraints);

      expect(result).not.toContain('## Layer Definitions');
    });
  });

  describe('anti-pattern formatting', () => {
    it('should format anti-patterns grouped by category', () => {
      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('## Anti-patterns (Blocked)');
      expect(result).toContain('### Escape Hatch');
      expect(result).toContain('### Type Safety');
    });

    it('should format individual anti-patterns', () => {
      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('Broad eslint-disable');
      expect(result).toContain('`AP-001`');
      expect(result).toContain("**Why it's problematic:**");
      expect(result).toContain('Disabling all ESLint rules');
      expect(result).toContain('**What to do instead:**');
      expect(result).toContain('Disable specific rules');
    });

    it('should include severity emojis for anti-patterns', () => {
      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('⚠️'); // warning
    });

    it('should exclude anti-patterns when configured', () => {
      const formatter = new LlmsTxtFormatter({ includeAntiPatterns: false });
      const result = formatter.format(sampleConstraints);

      expect(result).not.toContain('## Anti-patterns (Blocked)');
      expect(result).not.toContain('AP-001');
    });

    it('should skip anti-patterns section when empty', () => {
      const emptyConstraints: Constraints = {
        ...sampleConstraints,
        antiPatterns: [],
      };

      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(emptyConstraints);

      expect(result).not.toContain('## Anti-patterns (Blocked)');
    });
  });

  describe('convention formatting', () => {
    it('should format conventions', () => {
      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('## Conventions');
      expect(result).toContain('### Spelling');
      expect(result).toContain('Use UK English spelling');
    });

    it('should format convention examples', () => {
      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('**Examples:**');
      expect(result).toContain('organised (not organized)');
      expect(result).toContain('behaviour (not behavior)');
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

      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(constraintsNoExamples);

      expect(result).toContain('Test convention');
      expect(result).not.toContain('**Examples:**');
    });

    it('should exclude conventions when configured', () => {
      const formatter = new LlmsTxtFormatter({ includeConventions: false });
      const result = formatter.format(sampleConstraints);

      expect(result).not.toContain('## Conventions');
      expect(result).not.toContain('Spelling');
    });

    it('should skip conventions section when empty', () => {
      const emptyConstraints: Constraints = {
        ...sampleConstraints,
        conventions: [],
      };

      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(emptyConstraints);

      expect(result).not.toContain('## Conventions');
    });
  });

  describe('minimal constraints', () => {
    it('should handle constraints with only title', () => {
      const minimalConstraints: Constraints = {
        boundaries: [],
        layers: [],
        antiPatterns: [],
        conventions: [],
        metadata: {
          collectedAt: '2024-01-15T10:30:00.000Z',
          workspaceRoot: '/test',
          hasBaseline: false,
        },
      };

      const formatter = new LlmsTxtFormatter({ includeMetadata: false });
      const result = formatter.format(minimalConstraints);

      expect(result).toContain('# Anvil Architecture Constraints');
      expect(result).not.toContain('## Boundary Rules');
      expect(result).not.toContain('## Anti-patterns');
      expect(result).not.toContain('## Conventions');
    });
  });

  describe('formatAsLlmsTxt', () => {
    it('should format with default options', () => {
      const result = formatAsLlmsTxt(sampleConstraints);

      expect(result).toContain('# Anvil Architecture Constraints');
      expect(result).toContain('**Generated:**');
      expect(result).toContain('## Boundary Rules');
    });
  });

  describe('formatAsLlmsTxtWithoutMetadata', () => {
    it('should format without metadata', () => {
      const result = formatAsLlmsTxtWithoutMetadata(sampleConstraints);

      expect(result).toContain('# Anvil Architecture Constraints');
      expect(result).not.toContain('**Generated:**');
      expect(result).not.toContain('**Workspace:**');
      expect(result).toContain('## Boundary Rules');
    });
  });

  describe('category name formatting', () => {
    it('should format kebab-case category names', () => {
      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toContain('Escape Hatch');
      expect(result).toContain('Type Safety');
    });
  });

  describe('severity emojis', () => {
    it('should use correct emoji for each severity', () => {
      const constraintsWithAllSeverities: Constraints = {
        boundaries: [
          {
            name: 'error-boundary',
            from: 'a',
            to: 'b',
            message: 'Error level',
            severity: 'error',
          },
          {
            name: 'warning-boundary',
            from: 'c',
            to: 'd',
            message: 'Warning level',
            severity: 'warning',
          },
          {
            name: 'info-boundary',
            from: 'e',
            to: 'f',
            message: 'Info level',
            severity: 'info',
          },
        ],
        layers: [],
        antiPatterns: [],
        conventions: [],
        metadata: {
          collectedAt: '2024-01-15T10:30:00.000Z',
          workspaceRoot: '/test',
          hasBaseline: true,
        },
      };

      const formatter = new LlmsTxtFormatter();
      const result = formatter.format(constraintsWithAllSeverities);

      expect(result).toContain('🚫'); // error
      expect(result).toContain('⚠️'); // warning
      expect(result).toContain('ℹ️'); // info
    });
  });
});
