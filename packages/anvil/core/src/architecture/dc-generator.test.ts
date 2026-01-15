import { describe, it, expect } from 'vitest';
import { generateDCConfig } from './dc-generator.js';
import type { ArchitectureDefinition } from './definition-schema.js';

function createDefinition(overrides: Partial<ArchitectureDefinition> = {}): ArchitectureDefinition {
  return {
    schema_version: '0.1.0',
    template: 'custom',
    layers: {},
    rules: [],
    options: {
      detect_orphans: true,
      detect_circular: true,
      default_severity: 'error',
      exclude_patterns: ['**/*.test.ts'],
    },
    ...overrides,
  };
}

describe('dc-generator', () => {
  describe('globToRegex', () => {
    it('should escape dots before ** conversion', () => {
      const definition = createDefinition({
        layers: {
          domain: { patterns: ['src/domain/**/*.ts'], depends_on: [] },
          infra: { patterns: ['src/infra/**/*.ts'], depends_on: ['domain'] },
        },
      });

      const config = generateDCConfig(definition);

      expect(config).toContain('src\\\\/domain\\\\/(.*\\\\/)?[^\\\\/]*\\\\.ts$');
      expect(config).not.toContain('^src');
      expect(config).not.toContain('\\\\.*\\\\/');
    });

    it('should handle single * correctly', () => {
      const definition = createDefinition({
        layers: {
          utils: { patterns: ['src/*.ts'], depends_on: [] },
          other: { patterns: ['other/**'], depends_on: ['utils'] },
        },
      });

      const config = generateDCConfig(definition);

      expect(config).toContain('src\\\\/[^\\\\/]*\\\\.ts$');
    });

    it('should handle patterns with dots in names', () => {
      const definition = createDefinition({
        layers: {
          config: { patterns: ['src/config.service.ts'], depends_on: [] },
          other: { patterns: ['other/**'], depends_on: ['config'] },
        },
      });

      const config = generateDCConfig(definition);

      expect(config).toContain('src\\\\/config\\\\.service\\\\.ts$');
    });

    it('should escape regex metacharacters in patterns', () => {
      const definition = createDefinition({
        layers: {
          utils: { patterns: ['src/utils+helpers/**'], depends_on: [] },
          other: { patterns: ['other/**'], depends_on: ['utils'] },
        },
      });

      const config = generateDCConfig(definition);

      expect(config).toContain('utils\\\\+helpers');
    });

    it('should match root-level files with **/ pattern', () => {
      const definition = createDefinition({
        layers: {
          all: { patterns: ['**/*.ts'], depends_on: [] },
          other: { patterns: ['other/**'], depends_on: ['all'] },
        },
      });

      const config = generateDCConfig(definition);

      expect(config).toContain('(.*\\\\/)?[^\\\\/]*\\\\.ts$');
    });

    it('should anchor patterns at end only (not start) for DC path flexibility', () => {
      const definition = createDefinition({
        layers: {
          domain: { patterns: ['src/domain/**'], depends_on: [] },
          other: { patterns: ['other/**'], depends_on: ['domain'] },
        },
      });

      const config = generateDCConfig(definition);

      expect(config).toContain('src\\\\/domain\\\\/.*$');
      expect(config).not.toContain('^src\\\\/domain');
    });
  });

  describe('deduplicateRules', () => {
    it('should let user rules override auto-generated rules', () => {
      const definition = createDefinition({
        layers: {
          domain: { patterns: ['src/domain/**'], depends_on: [] },
        },
        rules: [
          {
            name: 'no-circular',
            from: 'domain',
            to: 'domain',
            severity: 'warn',
            allowed: false,
            message: 'Custom circular rule',
          },
        ],
      });

      const config = generateDCConfig(definition);
      const parsed = extractForbiddenRules(config);

      const circularRules = parsed.filter((r) => r.name === 'no-circular');
      expect(circularRules).toHaveLength(1);
      expect(circularRules[0].severity).toBe('warn');
      expect(circularRules[0].comment).toBe('Custom circular rule');
    });

    it('should preserve auto-generated rules when no conflict', () => {
      const definition = createDefinition({
        layers: {
          domain: { patterns: ['src/domain/**'], depends_on: [] },
          infra: { patterns: ['src/infra/**'], depends_on: ['domain'] },
        },
        options: {
          detect_orphans: true,
          detect_circular: true,
          default_severity: 'error',
          exclude_patterns: [],
        },
      });

      const config = generateDCConfig(definition);
      const parsed = extractForbiddenRules(config);

      expect(parsed.some((r) => r.name === 'no-circular')).toBe(true);
      expect(parsed.some((r) => r.name === 'no-orphans')).toBe(true);
    });
  });

  describe('generated regex path matching', () => {
    it('should match files at any depth with **/*.ts pattern', () => {
      const definition = createDefinition({
        layers: {
          all: { patterns: ['**/*.ts'], depends_on: [] },
          other: { patterns: ['other/**'], depends_on: ['all'] },
        },
      });

      const config = generateDCConfig(definition);
      const parsed = extractForbiddenRules(config);
      const layerRule = parsed.find((r) => r.name.includes('no-all-to'));
      expect(layerRule).toBeDefined();
      expect(layerRule!.from?.path).toBeDefined();

      const regex = new RegExp(layerRule!.from!.path!);

      expect(regex.test('file.ts')).toBe(true);
      expect(regex.test('src/file.ts')).toBe(true);
      expect(regex.test('src/deep/nested/file.ts')).toBe(true);
      expect(regex.test('file.js')).toBe(false);
    });

    it('should match layer paths without requiring leading anchor', () => {
      const definition = createDefinition({
        layers: {
          domain: { patterns: ['src/domain/**'], depends_on: [] },
          infra: { patterns: ['src/infra/**'], depends_on: ['domain'] },
        },
      });

      const config = generateDCConfig(definition);
      const parsed = extractForbiddenRules(config);
      const layerRule = parsed.find((r) => r.name === 'no-domain-to-disallowed');
      expect(layerRule).toBeDefined();
      expect(layerRule!.from?.path).toBeDefined();

      const regex = new RegExp(layerRule!.from!.path!);

      expect(regex.test('src/domain/entity.ts')).toBe(true);
      expect(regex.test('src/domain/nested/service.ts')).toBe(true);
      expect(regex.test('./src/domain/file.ts')).toBe(true);
      expect(regex.test('src/infra/repo.ts')).toBe(false);
    });

    it('should match specific file patterns', () => {
      const definition = createDefinition({
        layers: {
          config: { patterns: ['src/config.service.ts'], depends_on: [] },
          other: { patterns: ['other/**'], depends_on: ['config'] },
        },
      });

      const config = generateDCConfig(definition);
      const parsed = extractForbiddenRules(config);
      const layerRule = parsed.find((r) => r.name.includes('no-config-to'));
      expect(layerRule).toBeDefined();
      expect(layerRule!.from?.path).toBeDefined();

      const regex = new RegExp(layerRule!.from!.path!);

      expect(regex.test('src/config.service.ts')).toBe(true);
      expect(regex.test('src/config-service.ts')).toBe(false);
      expect(regex.test('src/configXservice.ts')).toBe(false);
    });
  });

  describe('default_severity', () => {
    it('should use default_severity for orphan detection', () => {
      const definition = createDefinition({
        options: {
          detect_orphans: true,
          detect_circular: false,
          default_severity: 'info',
          exclude_patterns: [],
        },
      });

      const config = generateDCConfig(definition);
      const parsed = extractForbiddenRules(config);

      const orphanRule = parsed.find((r) => r.name === 'no-orphans');
      expect(orphanRule).toBeDefined();
      expect(orphanRule?.severity).toBe('info');
    });

    it('should use default_severity for circular detection', () => {
      const definition = createDefinition({
        options: {
          detect_orphans: false,
          detect_circular: true,
          default_severity: 'warn',
          exclude_patterns: [],
        },
      });

      const config = generateDCConfig(definition);
      const parsed = extractForbiddenRules(config);

      const circularRule = parsed.find((r) => r.name === 'no-circular');
      expect(circularRule).toBeDefined();
      expect(circularRule?.severity).toBe('warn');
    });

    it('should use default_severity for layer rules', () => {
      const definition = createDefinition({
        layers: {
          domain: { patterns: ['src/domain/**'], depends_on: [] },
          infra: { patterns: ['src/infra/**'], depends_on: ['domain'] },
        },
        options: {
          detect_orphans: false,
          detect_circular: false,
          default_severity: 'info',
          exclude_patterns: [],
        },
      });

      const config = generateDCConfig(definition);
      const parsed = extractForbiddenRules(config);

      const layerRule = parsed.find((r) => r.name.includes('no-domain-to'));
      expect(layerRule).toBeDefined();
      expect(layerRule?.severity).toBe('info');
    });
  });
});

interface ParsedRule {
  name: string;
  severity: string;
  comment?: string;
  from?: { path?: string };
  to?: { path?: string };
}

function extractForbiddenRules(configContent: string): ParsedRule[] {
  const match = configContent.match(/module\.exports = ({[\s\S]*});/);
  if (!match) return [];

  const configObj = JSON.parse(match[1]);
  return configObj.forbidden || [];
}
