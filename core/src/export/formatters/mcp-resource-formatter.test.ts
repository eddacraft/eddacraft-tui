/**
 * Tests for MCP resource formatter
 */

import { describe, it, expect } from 'vitest';
import {
  McpResourceFormatter,
  formatAsMcpResource,
  formatAsMcpResourceJson,
  type McpResource,
} from './mcp-resource-formatter.js';
import type { Constraints } from '../constraint-collector.js';

describe('McpResourceFormatter', () => {
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
    it('should format complete constraints as MCP resource', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result).toHaveProperty('uri');
      expect(result).toHaveProperty('name');
      expect(result).toHaveProperty('description');
      expect(result).toHaveProperty('mimeType');
      expect(result).toHaveProperty('contents');
    });

    it('should set correct URI', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result.uri).toBe('anvil://constraints');
    });

    it('should allow custom base URI', () => {
      const formatter = new McpResourceFormatter({ baseUri: 'custom://anvil/constraints' });
      const result = formatter.format(sampleConstraints);

      expect(result.uri).toBe('custom://anvil/constraints');
    });

    it('should set correct name and description', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result.name).toBe('Anvil Architecture Constraints');
      expect(result.description).toContain('Architecture rules');
    });

    it('should set JSON MIME type', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result.mimeType).toBe('application/json');
    });
  });

  describe('contents metadata', () => {
    it('should include metadata', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result.contents.metadata).toBeDefined();
      expect(result.contents.metadata.workspaceRoot).toBe('/test/workspace');
      expect(result.contents.metadata.hasBaseline).toBe(true);
      expect(result.contents.metadata.version).toBe('1.0.0');
    });

    it('should include generation timestamp', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result.contents.metadata.generatedAt).toBeDefined();
      const timestamp = new Date(result.contents.metadata.generatedAt);
      expect(timestamp.getTime()).toBeLessThanOrEqual(Date.now());
    });
  });

  describe('boundaries formatting', () => {
    it('should format boundaries', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result.contents.boundaries).toBeDefined();
      expect(result.contents.boundaries).toHaveLength(2);
    });

    it('should include boundary IDs', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result.contents.boundaries![0].id).toBe('boundary-1');
      expect(result.contents.boundaries![1].id).toBe('boundary-2');
    });

    it('should include boundary fields', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      const boundary = result.contents.boundaries![0];
      expect(boundary.name).toBe('no-ui-to-domain');
      expect(boundary.from).toBe('ui');
      expect(boundary.to).toBe('domain');
      expect(boundary.message).toBe('UI layer must not directly access domain');
      expect(boundary.severity).toBe('error');
    });

    it('should exclude boundaries when configured', () => {
      const formatter = new McpResourceFormatter({ includeBoundaries: false });
      const result = formatter.format(sampleConstraints);

      expect(result.contents.boundaries).toBeUndefined();
    });

    it('should exclude boundaries when empty', () => {
      const emptyConstraints: Constraints = {
        ...sampleConstraints,
        boundaries: [],
      };

      const formatter = new McpResourceFormatter();
      const result = formatter.format(emptyConstraints);

      expect(result.contents.boundaries).toBeUndefined();
    });
  });

  describe('layers formatting', () => {
    it('should format layers', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result.contents.layers).toBeDefined();
      expect(result.contents.layers).toHaveLength(2);
    });

    it('should include layer fields', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      const layer = result.contents.layers![0];
      expect(layer.name).toBe('ui');
      expect(layer.patterns).toEqual(['src/ui/**']);
      expect(layer.dependsOn).toEqual(['application']);
      expect(layer.description).toBe('User interface layer');
    });

    it('should handle layers without dependencies', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      const layer = result.contents.layers![1];
      expect(layer.name).toBe('domain');
      expect(layer.dependsOn).toEqual([]);
    });

    it('should exclude layers when configured', () => {
      const formatter = new McpResourceFormatter({ includeLayers: false });
      const result = formatter.format(sampleConstraints);

      expect(result.contents.layers).toBeUndefined();
    });

    it('should exclude layers when empty', () => {
      const emptyConstraints: Constraints = {
        ...sampleConstraints,
        layers: [],
      };

      const formatter = new McpResourceFormatter();
      const result = formatter.format(emptyConstraints);

      expect(result.contents.layers).toBeUndefined();
    });
  });

  describe('anti-patterns formatting', () => {
    it('should format anti-patterns', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result.contents.antiPatterns).toBeDefined();
      expect(result.contents.antiPatterns).toHaveLength(2);
    });

    it('should include anti-pattern fields', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      const pattern = result.contents.antiPatterns![0];
      expect(pattern.id).toBe('AP-001');
      expect(pattern.name).toBe('Broad eslint-disable');
      expect(pattern.category).toBe('escape-hatch');
      expect(pattern.explanation).toContain('Disabling all ESLint rules');
      expect(pattern.suggestion).toContain('Disable specific rules');
      expect(pattern.severity).toBe('warning');
      expect(pattern.enabled).toBe(true);
    });

    it('should exclude anti-patterns when configured', () => {
      const formatter = new McpResourceFormatter({ includeAntiPatterns: false });
      const result = formatter.format(sampleConstraints);

      expect(result.contents.antiPatterns).toBeUndefined();
    });

    it('should exclude anti-patterns when empty', () => {
      const emptyConstraints: Constraints = {
        ...sampleConstraints,
        antiPatterns: [],
      };

      const formatter = new McpResourceFormatter();
      const result = formatter.format(emptyConstraints);

      expect(result.contents.antiPatterns).toBeUndefined();
    });
  });

  describe('conventions formatting', () => {
    it('should format conventions', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      expect(result.contents.conventions).toBeDefined();
      expect(result.contents.conventions).toHaveLength(2);
    });

    it('should include convention fields', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.format(sampleConstraints);

      const convention = result.contents.conventions![0];
      expect(convention.category).toBe('spelling');
      expect(convention.description).toBe('Use UK English spelling');
      expect(convention.examples).toEqual([
        'organised (not organized)',
        'behaviour (not behavior)',
      ]);
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

      const formatter = new McpResourceFormatter();
      const result = formatter.format(constraintsNoExamples);

      const convention = result.contents.conventions![0];
      expect(convention.examples).toBeUndefined();
    });

    it('should exclude conventions when configured', () => {
      const formatter = new McpResourceFormatter({ includeConventions: false });
      const result = formatter.format(sampleConstraints);

      expect(result.contents.conventions).toBeUndefined();
    });

    it('should exclude conventions when empty', () => {
      const emptyConstraints: Constraints = {
        ...sampleConstraints,
        conventions: [],
      };

      const formatter = new McpResourceFormatter();
      const result = formatter.format(emptyConstraints);

      expect(result.contents.conventions).toBeUndefined();
    });
  });

  describe('formatAsJson', () => {
    it('should format as pretty JSON by default', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.formatAsJson(sampleConstraints);

      expect(result).toContain('\n');
      expect(result).toContain('  ');
      expect(() => JSON.parse(result)).not.toThrow();
    });

    it('should format as compact JSON when not pretty', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.formatAsJson(sampleConstraints, false);

      expect(result).not.toContain('\n  ');
      expect(() => JSON.parse(result)).not.toThrow();
    });

    it('should produce valid JSON', () => {
      const formatter = new McpResourceFormatter();
      const result = formatter.formatAsJson(sampleConstraints);

      const parsed = JSON.parse(result) as McpResource;
      expect(parsed.uri).toBe('anvil://constraints');
      expect(parsed.contents.metadata).toBeDefined();
    });
  });

  describe('minimal constraints', () => {
    it('should handle constraints with no data', () => {
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

      const formatter = new McpResourceFormatter();
      const result = formatter.format(minimalConstraints);

      expect(result.uri).toBe('anvil://constraints');
      expect(result.contents.metadata).toBeDefined();
      expect(result.contents.boundaries).toBeUndefined();
      expect(result.contents.layers).toBeUndefined();
      expect(result.contents.antiPatterns).toBeUndefined();
      expect(result.contents.conventions).toBeUndefined();
    });
  });

  describe('formatAsMcpResource', () => {
    it('should format with default options', () => {
      const result = formatAsMcpResource(sampleConstraints);

      expect(result.uri).toBe('anvil://constraints');
      expect(result.contents.boundaries).toBeDefined();
      expect(result.contents.layers).toBeDefined();
      expect(result.contents.antiPatterns).toBeDefined();
      expect(result.contents.conventions).toBeDefined();
    });
  });

  describe('formatAsMcpResourceJson', () => {
    it('should format as pretty JSON by default', () => {
      const result = formatAsMcpResourceJson(sampleConstraints);

      expect(result).toContain('\n');
      expect(() => JSON.parse(result)).not.toThrow();
    });

    it('should format as compact JSON when requested', () => {
      const result = formatAsMcpResourceJson(sampleConstraints, false);

      expect(result).not.toContain('\n  ');
      expect(() => JSON.parse(result)).not.toThrow();
    });
  });
});
