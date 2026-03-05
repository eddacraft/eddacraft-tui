/**
 * Tests for yaml-parser.ts and definition-schema.ts
 * Covers: parsing, writing, path utilities, template defaults, merging,
 * round-trip serialization, Zod validation, and template enumeration.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdirSync, mkdtempSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';
import YAML from 'yaml';
import { minimatch } from 'minimatch';
import {
  getArchitectureYamlPath,
  architectureYamlExists,
  parseArchitectureDefinition,
  writeArchitectureYaml,
  getTemplateDefaults,
  mergeWithTemplate,
  createDefinitionFromTemplate,
  ARCHITECTURE_YAML_FILENAME,
  ANVIL_DIR,
} from './yaml-parser.js';
import {
  validateArchitectureDefinition,
  getAvailableTemplates,
  isValidTemplate,
  getDefaultOptions,
  type ArchitectureDefinition,
  type ArchitectureTemplate,
  ARCHITECTURE_DEFINITION_VERSION,
} from './definition-schema.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeTempDir(): string {
  return mkdtempSync(join(tmpdir(), 'anvil-yaml-test-'));
}

function writeYamlFile(workspaceRoot: string, content: string): void {
  const anvilDir = join(workspaceRoot, ANVIL_DIR);
  mkdirSync(anvilDir, { recursive: true });
  writeFileSync(join(anvilDir, ARCHITECTURE_YAML_FILENAME), content, 'utf-8');
}

function minimalValidYaml(): string {
  return YAML.stringify({
    schema_version: '0.1.0',
    template: 'starter',
    layers: {
      lib: { patterns: ['src/lib/**'], depends_on: [] },
    },
  });
}

function minimalDefinition(): ArchitectureDefinition {
  return {
    schema_version: '0.1.0',
    template: 'starter',
    layers: {
      lib: { patterns: ['src/lib/**'], depends_on: [] },
    },
    rules: [],
    options: getDefaultOptions(),
  };
}

// ---------------------------------------------------------------------------
// yaml-parser.ts
// ---------------------------------------------------------------------------

describe('yaml-parser', () => {
  let testDir: string;

  beforeEach(() => {
    testDir = makeTempDir();
  });

  afterEach(async () => {
    await safeCleanup(testDir);
  });

  // --- getArchitectureYamlPath -------------------------------------------

  describe('getArchitectureYamlPath', () => {
    it('should return path ending in .anvil/architecture.yaml', () => {
      const p = getArchitectureYamlPath('/workspace');
      expect(p).toBe(join('/workspace', ANVIL_DIR, ARCHITECTURE_YAML_FILENAME));
    });

    it('should combine workspace root with .anvil dir and filename', () => {
      const p = getArchitectureYamlPath(testDir);
      expect(p).toBe(join(testDir, '.anvil', 'architecture.yaml'));
    });
  });

  // --- architectureYamlExists --------------------------------------------

  describe('architectureYamlExists', () => {
    it('should return false when file does not exist', () => {
      expect(architectureYamlExists(testDir)).toBe(false);
    });

    it('should return true when file exists', () => {
      writeYamlFile(testDir, minimalValidYaml());
      expect(architectureYamlExists(testDir)).toBe(true);
    });
  });

  // --- parseArchitectureDefinition ---------------------------------------

  describe('parseArchitectureDefinition', () => {
    it('should parse valid YAML and return validated definition', async () => {
      writeYamlFile(testDir, minimalValidYaml());

      const def = await parseArchitectureDefinition(testDir);

      expect(def.schema_version).toBe('0.1.0');
      expect(def.template).toBe('starter');
      expect(def.layers).toHaveProperty('lib');
      expect(def.layers.lib.patterns).toEqual(['src/lib/**']);
    });

    it('should apply default options when options are omitted', async () => {
      writeYamlFile(
        testDir,
        YAML.stringify({
          schema_version: '0.1.0',
          template: 'custom',
          layers: {
            core: { patterns: ['src/core/**'], depends_on: [] },
          },
        })
      );

      const def = await parseArchitectureDefinition(testDir);

      expect(def.options).toEqual(getDefaultOptions());
    });

    it('should preserve explicitly provided options', async () => {
      const customOptions = {
        detect_orphans: false,
        detect_circular: false,
        default_severity: 'warn' as const,
        exclude_patterns: ['custom/**'],
      };
      writeYamlFile(
        testDir,
        YAML.stringify({
          schema_version: '0.1.0',
          template: 'custom',
          layers: {
            core: { patterns: ['src/core/**'], depends_on: [] },
          },
          options: customOptions,
        })
      );

      const def = await parseArchitectureDefinition(testDir);

      expect(def.options?.detect_orphans).toBe(false);
      expect(def.options?.detect_circular).toBe(false);
      expect(def.options?.default_severity).toBe('warn');
      expect(def.options?.exclude_patterns).toEqual(['custom/**']);
    });

    it('should throw when YAML file does not exist', async () => {
      await expect(parseArchitectureDefinition(testDir)).rejects.toThrow(
        /Architecture YAML not found/
      );
    });

    it('should throw on malformed YAML', async () => {
      // Unclosed flow mapping causes a YAML parse error
      writeYamlFile(testDir, '{ unclosed: brace');

      await expect(parseArchitectureDefinition(testDir)).rejects.toThrow();
    });

    it('should throw on invalid schema (wrong version)', async () => {
      writeYamlFile(
        testDir,
        YAML.stringify({
          schema_version: '9.9.9',
          template: 'starter',
          layers: {
            lib: { patterns: ['src/lib/**'], depends_on: [] },
          },
        })
      );

      await expect(parseArchitectureDefinition(testDir)).rejects.toThrow(
        /Invalid architecture\.yaml/
      );
    });

    it('should throw on invalid template name', async () => {
      writeYamlFile(
        testDir,
        YAML.stringify({
          schema_version: '0.1.0',
          template: 'nonexistent-template',
          layers: {
            lib: { patterns: ['src/lib/**'], depends_on: [] },
          },
        })
      );

      await expect(parseArchitectureDefinition(testDir)).rejects.toThrow(
        /Invalid architecture\.yaml/
      );
    });

    it('should throw when layer patterns array is empty', async () => {
      writeYamlFile(
        testDir,
        YAML.stringify({
          schema_version: '0.1.0',
          template: 'custom',
          layers: {
            bad: { patterns: [], depends_on: [] },
          },
        })
      );

      await expect(parseArchitectureDefinition(testDir)).rejects.toThrow(
        /Invalid architecture\.yaml/
      );
    });

    it('should parse definition with rules', async () => {
      writeYamlFile(
        testDir,
        YAML.stringify({
          schema_version: '0.1.0',
          template: 'layered',
          layers: {
            ui: { patterns: ['src/ui/**'], depends_on: ['logic'] },
            logic: { patterns: ['src/logic/**'], depends_on: [] },
          },
          rules: [
            {
              name: 'no-backward',
              from: 'logic',
              to: 'ui',
              severity: 'error',
              allowed: false,
            },
          ],
        })
      );

      const def = await parseArchitectureDefinition(testDir);

      expect(def.rules).toHaveLength(1);
      expect(def.rules[0].name).toBe('no-backward');
      expect(def.rules[0].severity).toBe('error');
      expect(def.rules[0].allowed).toBe(false);
    });

    it('should parse definition with bounded_contexts', async () => {
      writeYamlFile(
        testDir,
        YAML.stringify({
          schema_version: '0.1.0',
          template: 'ddd',
          layers: {
            domain: { patterns: ['src/domain/**'], depends_on: [] },
          },
          bounded_contexts: {
            ordering: {
              allowed_dependencies: ['shared'],
              description: 'Order management context',
            },
          },
        })
      );

      const def = await parseArchitectureDefinition(testDir);

      expect(def.bounded_contexts).toBeDefined();
      expect(def.bounded_contexts?.ordering).toBeDefined();
      expect(def.bounded_contexts?.ordering.description).toBe('Order management context');
    });

    it('should apply Zod defaults for omitted fields', async () => {
      // Minimal YAML: only layers with patterns
      writeYamlFile(
        testDir,
        YAML.stringify({
          layers: {
            core: { patterns: ['src/**'] },
          },
        })
      );

      const def = await parseArchitectureDefinition(testDir);

      // Zod should fill in defaults
      expect(def.schema_version).toBe(ARCHITECTURE_DEFINITION_VERSION);
      expect(def.template).toBe('custom');
      expect(def.rules).toEqual([]);
      expect(def.layers.core.depends_on).toEqual([]);
    });
  });

  // --- writeArchitectureYaml ---------------------------------------------

  describe('writeArchitectureYaml', () => {
    it('should write a YAML file to the correct path', async () => {
      mkdirSync(join(testDir, ANVIL_DIR), { recursive: true });

      await writeArchitectureYaml(testDir, minimalDefinition());

      expect(existsSync(getArchitectureYamlPath(testDir))).toBe(true);
    });

    it('should write parsable YAML content', async () => {
      mkdirSync(join(testDir, ANVIL_DIR), { recursive: true });
      const def = minimalDefinition();

      await writeArchitectureYaml(testDir, def);

      const raw = readFileSync(getArchitectureYamlPath(testDir), 'utf-8');
      const parsed = YAML.parse(raw);
      expect(parsed.schema_version).toBe('0.1.0');
      expect(parsed.template).toBe('starter');
    });

    it('should fail when .anvil directory does not exist', async () => {
      // writeFile will throw ENOENT when parent dir is missing
      await expect(writeArchitectureYaml(testDir, minimalDefinition())).rejects.toThrow();
    });
  });

  // --- Round-trip: write then parse --------------------------------------

  describe('round-trip', () => {
    it('should return equivalent data after write then parse', async () => {
      mkdirSync(join(testDir, ANVIL_DIR), { recursive: true });
      const original = minimalDefinition();

      await writeArchitectureYaml(testDir, original);
      const parsed = await parseArchitectureDefinition(testDir);

      expect(parsed.schema_version).toBe(original.schema_version);
      expect(parsed.template).toBe(original.template);
      expect(parsed.layers).toEqual(original.layers);
      expect(parsed.rules).toEqual(original.rules);
      expect(parsed.options).toEqual(original.options);
    });

    it('should preserve complex definitions through round-trip', async () => {
      mkdirSync(join(testDir, ANVIL_DIR), { recursive: true });

      const complex: ArchitectureDefinition = {
        schema_version: '0.1.0',
        template: 'hexagonal',
        layers: {
          core: {
            patterns: ['src/domain/**', 'src/core/**'],
            depends_on: [],
            description: 'Domain logic',
          },
          adapters: {
            patterns: ['src/adapters/**'],
            depends_on: ['core'],
            description: 'Adapter implementations',
          },
        },
        rules: [
          {
            name: 'core-isolation',
            from: 'core',
            to: 'adapters',
            severity: 'error',
            allowed: false,
            message: 'Core must not depend on adapters',
          },
        ],
        options: {
          detect_orphans: true,
          detect_circular: true,
          default_severity: 'warn',
          exclude_patterns: ['**/*.test.ts'],
        },
      };

      await writeArchitectureYaml(testDir, complex);
      const parsed = await parseArchitectureDefinition(testDir);

      expect(parsed.schema_version).toBe(complex.schema_version);
      expect(parsed.template).toBe(complex.template);
      expect(parsed.layers.core.description).toBe('Domain logic');
      expect(parsed.layers.adapters.depends_on).toEqual(['core']);
      expect(parsed.rules).toHaveLength(1);
      expect(parsed.rules[0].message).toBe('Core must not depend on adapters');
      expect(parsed.options?.default_severity).toBe('warn');
    });
  });

  // --- getTemplateDefaults -----------------------------------------------

  describe('getTemplateDefaults', () => {
    const ALL_TEMPLATES: ArchitectureTemplate[] = [
      'starter',
      'layered',
      'hexagonal',
      'clean',
      'ddd',
      'monorepo',
      'serverless',
      'nx-workspace',
      'custom',
    ];

    it.each(ALL_TEMPLATES)('should return layers for template "%s"', (template) => {
      const layers = getTemplateDefaults(template);
      expect(typeof layers).toBe('object');
      expect(layers).not.toBeNull();
    });

    it('should return empty object for custom template', () => {
      const layers = getTemplateDefaults('custom');
      expect(Object.keys(layers)).toHaveLength(0);
    });

    it('should return expected layers for starter template', () => {
      const layers = getTemplateDefaults('starter');
      expect(layers).toHaveProperty('components');
      expect(layers).toHaveProperty('lib');
      expect(layers).toHaveProperty('services');
    });

    it('should return expected layers for layered template', () => {
      const layers = getTemplateDefaults('layered');
      expect(layers).toHaveProperty('presentation');
      expect(layers).toHaveProperty('business');
      expect(layers).toHaveProperty('data');
      expect(layers).toHaveProperty('shared');
    });

    it('should return expected layers for hexagonal template', () => {
      const layers = getTemplateDefaults('hexagonal');
      expect(layers).toHaveProperty('core');
      expect(layers).toHaveProperty('ports');
      expect(layers).toHaveProperty('adapters');
      expect(layers).toHaveProperty('application');
    });

    it('should return expected layers for clean template', () => {
      const layers = getTemplateDefaults('clean');
      expect(layers).toHaveProperty('entities');
      expect(layers).toHaveProperty('use_cases');
      expect(layers).toHaveProperty('interface_adapters');
      expect(layers).toHaveProperty('frameworks');
    });

    it('should return expected layers for ddd template', () => {
      const layers = getTemplateDefaults('ddd');
      expect(layers).toHaveProperty('domain');
      expect(layers).toHaveProperty('application');
      expect(layers).toHaveProperty('infrastructure');
      expect(layers).toHaveProperty('interfaces');
    });

    it('should return expected layers for monorepo template', () => {
      const layers = getTemplateDefaults('monorepo');
      expect(layers).toHaveProperty('packages');
      expect(layers).toHaveProperty('shared');
    });

    it('should return expected layers for serverless template', () => {
      const layers = getTemplateDefaults('serverless');
      expect(layers).toHaveProperty('functions');
      expect(layers).toHaveProperty('services');
      expect(layers).toHaveProperty('shared');
    });

    it('should return expected layers for nx-workspace template', () => {
      const layers = getTemplateDefaults('nx-workspace');
      expect(layers).toHaveProperty('apps');
      expect(layers).toHaveProperty('feature-libs');
      expect(layers).toHaveProperty('data-access-libs');
      expect(layers).toHaveProperty('ui-libs');
      expect(layers).toHaveProperty('shared-libs');
    });

    it('should return a shallow copy (not the same reference)', () => {
      const a = getTemplateDefaults('layered');
      const b = getTemplateDefaults('layered');
      expect(a).toEqual(b);
      expect(a).not.toBe(b);
    });

    it('should include patterns and depends_on for each layer', () => {
      const layers = getTemplateDefaults('layered');
      for (const [, layer] of Object.entries(layers)) {
        expect(Array.isArray(layer.patterns)).toBe(true);
        expect(layer.patterns.length).toBeGreaterThan(0);
        expect(Array.isArray(layer.depends_on)).toBe(true);
      }
    });
  });

  // --- mergeWithTemplate -------------------------------------------------

  describe('mergeWithTemplate', () => {
    it('should use template layers when user layers are empty', () => {
      const def: ArchitectureDefinition = {
        schema_version: '0.1.0',
        template: 'layered',
        layers: {},
        rules: [],
      };

      const merged = mergeWithTemplate(def);

      expect(Object.keys(merged.layers).length).toBeGreaterThan(0);
      expect(merged.layers).toHaveProperty('presentation');
      expect(merged.layers).toHaveProperty('business');
    });

    it('should keep user layers when they are present', () => {
      const userLayers = {
        myLayer: { patterns: ['src/my/**'], depends_on: [] },
      };
      const def: ArchitectureDefinition = {
        schema_version: '0.1.0',
        template: 'layered',
        layers: userLayers,
        rules: [],
      };

      const merged = mergeWithTemplate(def);

      expect(merged.layers).toEqual(userLayers);
      expect(merged.layers).not.toHaveProperty('presentation');
    });

    it('should apply default options when options are undefined', () => {
      const def: ArchitectureDefinition = {
        schema_version: '0.1.0',
        template: 'custom',
        layers: {},
        rules: [],
      };

      const merged = mergeWithTemplate(def);

      expect(merged.options).toEqual(getDefaultOptions());
    });

    it('should preserve existing options', () => {
      const customOptions = {
        detect_orphans: false,
        detect_circular: false,
        default_severity: 'warn' as const,
        exclude_patterns: [],
      };
      const def: ArchitectureDefinition = {
        schema_version: '0.1.0',
        template: 'custom',
        layers: {},
        rules: [],
        options: customOptions,
      };

      const merged = mergeWithTemplate(def);

      expect(merged.options).toEqual(customOptions);
    });

    it('should not mutate the input definition', () => {
      const def: ArchitectureDefinition = {
        schema_version: '0.1.0',
        template: 'layered',
        layers: {},
        rules: [],
      };

      const original = { ...def, layers: { ...def.layers } };
      mergeWithTemplate(def);

      expect(def).toEqual(original);
    });
  });

  // --- createDefinitionFromTemplate --------------------------------------

  describe('createDefinitionFromTemplate', () => {
    const ALL_TEMPLATES: ArchitectureTemplate[] = [
      'starter',
      'layered',
      'hexagonal',
      'clean',
      'ddd',
      'monorepo',
      'serverless',
      'nx-workspace',
      'custom',
    ];

    it.each(ALL_TEMPLATES)('should create valid definition for template "%s"', (template) => {
      const def = createDefinitionFromTemplate(template);

      expect(def.schema_version).toBe('0.1.0');
      expect(def.template).toBe(template);
      expect(def.rules).toEqual([]);
      expect(def.options).toEqual(getDefaultOptions());
      expect(def.layers).toEqual(getTemplateDefaults(template));
    });

    it('should create a definition that passes Zod validation', () => {
      const def = createDefinitionFromTemplate('hexagonal');
      const result = validateArchitectureDefinition(def);
      expect(result.success).toBe(true);
    });

    it('should produce write-then-parse-ready definitions', async () => {
      mkdirSync(join(testDir, ANVIL_DIR), { recursive: true });

      const def = createDefinitionFromTemplate('ddd');
      await writeArchitectureYaml(testDir, def);
      const parsed = await parseArchitectureDefinition(testDir);

      expect(parsed.template).toBe('ddd');
      expect(parsed.layers).toHaveProperty('domain');
    });
  });
});

// ---------------------------------------------------------------------------
// definition-schema.ts
// ---------------------------------------------------------------------------

describe('definition-schema', () => {
  // --- validateArchitectureDefinition ------------------------------------

  describe('validateArchitectureDefinition', () => {
    it('should succeed for minimal valid data', () => {
      const result = validateArchitectureDefinition({
        schema_version: '0.1.0',
        template: 'custom',
        layers: {
          core: { patterns: ['src/**'], depends_on: [] },
        },
      });

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.schema_version).toBe('0.1.0');
      }
    });

    it('should succeed with only layers (defaults applied)', () => {
      const result = validateArchitectureDefinition({
        layers: {
          core: { patterns: ['src/**'] },
        },
      });

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.schema_version).toBe(ARCHITECTURE_DEFINITION_VERSION);
        expect(result.data.template).toBe('custom');
        expect(result.data.rules).toEqual([]);
        expect(result.data.layers.core.depends_on).toEqual([]);
      }
    });

    it('should succeed with empty object (all defaults)', () => {
      const result = validateArchitectureDefinition({});
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.schema_version).toBe(ARCHITECTURE_DEFINITION_VERSION);
        expect(result.data.template).toBe('custom');
        expect(result.data.layers).toEqual({});
        expect(result.data.rules).toEqual([]);
      }
    });

    it('should fail for invalid schema_version', () => {
      const result = validateArchitectureDefinition({
        schema_version: '2.0.0',
        template: 'custom',
        layers: {},
      });

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toBeDefined();
      }
    });

    it('should fail for invalid template name', () => {
      const result = validateArchitectureDefinition({
        schema_version: '0.1.0',
        template: 'does-not-exist',
        layers: {},
      });

      expect(result.success).toBe(false);
    });

    it('should fail when patterns is empty array', () => {
      const result = validateArchitectureDefinition({
        schema_version: '0.1.0',
        template: 'custom',
        layers: {
          bad: { patterns: [], depends_on: [] },
        },
      });

      expect(result.success).toBe(false);
    });

    it('should fail when patterns is missing', () => {
      const result = validateArchitectureDefinition({
        schema_version: '0.1.0',
        template: 'custom',
        layers: {
          bad: { depends_on: [] },
        },
      });

      expect(result.success).toBe(false);
    });

    it('should validate rules with all fields', () => {
      const result = validateArchitectureDefinition({
        schema_version: '0.1.0',
        template: 'custom',
        layers: {
          a: { patterns: ['a/**'], depends_on: [] },
          b: { patterns: ['b/**'], depends_on: [] },
        },
        rules: [
          {
            name: 'test-rule',
            from: 'a',
            to: 'b',
            severity: 'warn',
            allowed: true,
            message: 'Test message',
          },
        ],
      });

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.rules[0].severity).toBe('warn');
        expect(result.data.rules[0].allowed).toBe(true);
        expect(result.data.rules[0].message).toBe('Test message');
      }
    });

    it('should apply rule defaults (severity=error, allowed=false)', () => {
      const result = validateArchitectureDefinition({
        schema_version: '0.1.0',
        template: 'custom',
        layers: {},
        rules: [{ name: 'r', from: 'a', to: 'b' }],
      });

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.rules[0].severity).toBe('error');
        expect(result.data.rules[0].allowed).toBe(false);
      }
    });

    it('should fail for invalid severity in rules', () => {
      const result = validateArchitectureDefinition({
        schema_version: '0.1.0',
        template: 'custom',
        layers: {},
        rules: [{ name: 'r', from: 'a', to: 'b', severity: 'critical' }],
      });

      expect(result.success).toBe(false);
    });

    it('should validate options with all severity levels', () => {
      for (const severity of ['error', 'warn', 'info', 'ignore'] as const) {
        const result = validateArchitectureDefinition({
          schema_version: '0.1.0',
          template: 'custom',
          layers: {},
          options: {
            detect_orphans: true,
            detect_circular: true,
            default_severity: severity,
            exclude_patterns: [],
          },
        });
        expect(result.success).toBe(true);
      }
    });

    it('should validate bounded_contexts', () => {
      const result = validateArchitectureDefinition({
        schema_version: '0.1.0',
        template: 'ddd',
        layers: {
          domain: { patterns: ['src/domain/**'], depends_on: [] },
        },
        bounded_contexts: {
          ordering: {
            allowed_dependencies: ['shared'],
            description: 'Order management',
            layers: {
              model: { patterns: ['src/ordering/model/**'], depends_on: [] },
            },
          },
        },
      });

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data.bounded_contexts?.ordering.description).toBe('Order management');
        expect(result.data.bounded_contexts?.ordering.allowed_dependencies).toEqual(['shared']);
      }
    });

    it('should fail for non-object input', () => {
      expect(validateArchitectureDefinition(null).success).toBe(false);
      expect(validateArchitectureDefinition(undefined).success).toBe(false);
      expect(validateArchitectureDefinition('string').success).toBe(false);
      expect(validateArchitectureDefinition(42).success).toBe(false);
    });

    it('should validate all 9 template names', () => {
      const templates = getAvailableTemplates();
      for (const template of templates) {
        const result = validateArchitectureDefinition({
          schema_version: '0.1.0',
          template,
          layers: {},
        });
        expect(result.success).toBe(true);
      }
    });
  });

  // --- getAvailableTemplates ---------------------------------------------

  describe('getAvailableTemplates', () => {
    it('should return all 9 template names', () => {
      const templates = getAvailableTemplates();
      expect(templates).toHaveLength(9);
    });

    it('should include every expected template', () => {
      const templates = getAvailableTemplates();
      const expected: ArchitectureTemplate[] = [
        'starter',
        'layered',
        'hexagonal',
        'clean',
        'ddd',
        'monorepo',
        'serverless',
        'nx-workspace',
        'custom',
      ];
      for (const t of expected) {
        expect(templates).toContain(t);
      }
    });

    it('should return a new array each time (not the same reference)', () => {
      const a = getAvailableTemplates();
      const b = getAvailableTemplates();
      expect(a).toEqual(b);
      expect(a).not.toBe(b);
    });
  });

  // --- isValidTemplate ---------------------------------------------------

  describe('isValidTemplate', () => {
    it.each([
      'starter',
      'layered',
      'hexagonal',
      'clean',
      'ddd',
      'monorepo',
      'serverless',
      'nx-workspace',
      'custom',
    ])('should return true for valid template "%s"', (template) => {
      expect(isValidTemplate(template)).toBe(true);
    });

    it('should return false for invalid template names', () => {
      expect(isValidTemplate('nonexistent')).toBe(false);
      expect(isValidTemplate('')).toBe(false);
      expect(isValidTemplate('Layered')).toBe(false); // case-sensitive
      expect(isValidTemplate('CUSTOM')).toBe(false);
    });
  });

  // --- getDefaultOptions -------------------------------------------------

  describe('getDefaultOptions', () => {
    it('should return expected default values', () => {
      const opts = getDefaultOptions();
      expect(opts.detect_orphans).toBe(true);
      expect(opts.detect_circular).toBe(true);
      expect(opts.default_severity).toBe('error');
    });

    it('should include standard exclude patterns', () => {
      const opts = getDefaultOptions();
      expect(opts.exclude_patterns).toContain('**/*.test.ts');
      expect(opts.exclude_patterns).toContain('**/*.spec.ts');
      expect(opts.exclude_patterns).toContain('**/__tests__/**');
      expect(opts.exclude_patterns).toContain('**/__fixtures__/**');
      expect(opts.exclude_patterns).toContain('**/node_modules/**');
    });

    it('should return 5 default exclude patterns', () => {
      const opts = getDefaultOptions();
      expect(opts.exclude_patterns).toHaveLength(5);
    });

    it('should return a new object each time', () => {
      const a = getDefaultOptions();
      const b = getDefaultOptions();
      expect(a).toEqual(b);
      expect(a).not.toBe(b);
    });
  });
});

// ---------------------------------------------------------------------------
// XPLAT-005: Windows path compatibility
// ---------------------------------------------------------------------------

describe('template patterns with Windows-style paths', () => {
  /**
   * Template patterns use forward slashes (e.g. 'src/controllers/**').
   * On Windows, file paths use backslashes. Code that consumes these patterns
   * must normalise separators before matching. This test verifies that
   * normalising backslashes to forward slashes makes minimatch work correctly
   * with all template patterns — the same approach used by layer-detector.ts.
   */

  function normaliseToForwardSlash(p: string): string {
    return p.replace(/\\/g, '/');
  }

  it('template layer patterns match forward-slash paths', () => {
    const templates = getAvailableTemplates();

    for (const template of templates) {
      const defaults = getTemplateDefaults(template as ArchitectureTemplate);
      for (const [, layer] of Object.entries(defaults)) {
        for (const pattern of layer.patterns ?? []) {
          // A forward-slash path should match its own pattern
          const samplePath = pattern.replace('**/*', 'example.ts').replace('**', 'example.ts');

          expect(
            minimatch(samplePath, pattern, { matchBase: true }),
            `pattern '${pattern}' from template '${template}' should match '${samplePath}'`
          ).toBe(true);
        }
      }
    }
  });

  it('template layer patterns match backslash paths after normalisation', () => {
    const templates = getAvailableTemplates();

    for (const template of templates) {
      const defaults = getTemplateDefaults(template as ArchitectureTemplate);
      for (const [, layer] of Object.entries(defaults)) {
        for (const pattern of layer.patterns ?? []) {
          // Simulate a Windows path by replacing / with \ in the sample
          const samplePath = pattern.replace('**/*', 'example.ts').replace('**', 'example.ts');
          const windowsPath = samplePath.replace(/\//g, '\\');

          // Without normalisation it may fail; with normalisation it must match
          const normalised = normaliseToForwardSlash(windowsPath);
          expect(
            minimatch(normalised, pattern, { matchBase: true }),
            `normalised Windows path '${normalised}' should match pattern '${pattern}'`
          ).toBe(true);
        }
      }
    }
  });

  it('all template patterns use forward slashes only', () => {
    const templates = getAvailableTemplates();

    for (const template of templates) {
      const defaults = getTemplateDefaults(template as ArchitectureTemplate);
      for (const [layerName, layer] of Object.entries(defaults)) {
        for (const pattern of layer.patterns ?? []) {
          expect(
            pattern.includes('\\'),
            `pattern '${pattern}' in layer '${layerName}' of template '${template}' should not contain backslashes`
          ).toBe(false);
        }
      }
    }
  });
});
