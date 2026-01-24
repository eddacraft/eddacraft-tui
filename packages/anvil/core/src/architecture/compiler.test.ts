import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { compileArchitecture, needsCompilation } from './compiler.js';
import { mkdir, writeFile, rm, readFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { randomUUID } from 'node:crypto';
import YAML from 'yaml';
import type { ArchitectureDefinition } from './definition-schema.js';

describe('compiler', () => {
  let testDir: string;

  const createArchitectureYaml = async (definition: Partial<ArchitectureDefinition>) => {
    const full: ArchitectureDefinition = {
      schema_version: '0.1.0',
      template: 'custom',
      layers: {},
      rules: [],
      ...definition,
    };
    const yamlPath = join(testDir, '.anvil', 'architecture.yaml');
    await mkdir(join(testDir, '.anvil'), { recursive: true });
    await writeFile(yamlPath, YAML.stringify(full), 'utf-8');
    return full;
  };

  beforeEach(async () => {
    testDir = join(tmpdir(), `anvil-compiler-test-${randomUUID()}`);
    await mkdir(testDir, { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('compileArchitecture', () => {
    it('throws when no architecture.yaml exists', async () => {
      await expect(compileArchitecture(testDir)).rejects.toThrow('No architecture.yaml found');
    });

    it('generates both DC config and Rego policy', async () => {
      await createArchitectureYaml({
        layers: {
          api: { patterns: ['src/api/**'], depends_on: ['service'] },
          service: { patterns: ['src/service/**'], depends_on: [] },
        },
      });

      const result = await compileArchitecture(testDir);

      expect(result.dcConfig.regenerated).toBe(true);
      expect(result.regoPolicy.regenerated).toBe(true);
      expect(existsSync(result.dcConfig.path)).toBe(true);
      expect(existsSync(result.regoPolicy.path)).toBe(true);
    });

    it('skips regeneration when up to date', async () => {
      await createArchitectureYaml({
        layers: {
          api: { patterns: ['src/api/**'], depends_on: [] },
        },
      });

      const first = await compileArchitecture(testDir);
      expect(first.dcConfig.regenerated).toBe(true);
      expect(first.regoPolicy.regenerated).toBe(true);

      const second = await compileArchitecture(testDir);
      expect(second.dcConfig.regenerated).toBe(false);
      expect(second.regoPolicy.regenerated).toBe(false);
    });

    it('regenerates when forced', async () => {
      await createArchitectureYaml({
        layers: {
          api: { patterns: ['src/api/**'], depends_on: [] },
        },
      });

      await compileArchitecture(testDir);
      const forced = await compileArchitecture(testDir, { force: true });

      expect(forced.dcConfig.regenerated).toBe(true);
      expect(forced.regoPolicy.regenerated).toBe(true);
    });

    it('skips DC when skipDC option is set', async () => {
      await createArchitectureYaml({
        layers: {
          api: { patterns: ['src/api/**'], depends_on: [] },
        },
      });

      const result = await compileArchitecture(testDir, { skipDC: true });

      expect(result.dcConfig.regenerated).toBe(false);
      expect(result.regoPolicy.regenerated).toBe(true);
    });

    it('skips Rego when skipRego option is set', async () => {
      await createArchitectureYaml({
        layers: {
          api: { patterns: ['src/api/**'], depends_on: [] },
        },
      });

      const result = await compileArchitecture(testDir, { skipRego: true });

      expect(result.dcConfig.regenerated).toBe(true);
      expect(result.regoPolicy.regenerated).toBe(false);
    });

    it('generated files contain matching hashes', async () => {
      await createArchitectureYaml({
        layers: {
          api: { patterns: ['src/api/**'], depends_on: [] },
        },
      });

      const result = await compileArchitecture(testDir);

      const dcContent = await readFile(result.dcConfig.path, 'utf-8');
      const regoContent = await readFile(result.regoPolicy.path, 'utf-8');

      const dcHash = dcContent.match(/\/\/ hash: ([a-f0-9]+)/)?.[1];
      const regoHash = regoContent.match(/# hash: ([a-f0-9]+)/)?.[1];

      expect(dcHash).toBeDefined();
      expect(regoHash).toBeDefined();
      expect(dcHash).toBe(regoHash);
    });
  });

  describe('needsCompilation', () => {
    it('returns false when no architecture.yaml exists', async () => {
      const result = await needsCompilation(testDir);
      expect(result).toEqual({ dc: false, rego: false, any: false });
    });

    it('returns true for both when configs do not exist', async () => {
      await createArchitectureYaml({
        layers: { api: { patterns: ['src/api/**'], depends_on: [] } },
      });

      const result = await needsCompilation(testDir);
      expect(result.dc).toBe(true);
      expect(result.rego).toBe(true);
      expect(result.any).toBe(true);
    });

    it('returns false when configs are up to date', async () => {
      await createArchitectureYaml({
        layers: { api: { patterns: ['src/api/**'], depends_on: [] } },
      });

      await compileArchitecture(testDir);
      const result = await needsCompilation(testDir);

      expect(result.dc).toBe(false);
      expect(result.rego).toBe(false);
      expect(result.any).toBe(false);
    });
  });
});
