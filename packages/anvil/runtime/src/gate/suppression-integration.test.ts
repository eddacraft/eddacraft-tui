import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import * as os from 'node:os';
import { GateRunner } from './gate-runner.js';
import { SuppressionStore } from '@anvil/core';

describe('Suppression Integration', () => {
  let tempDir: string;
  let anvilDir: string;

  beforeEach(async () => {
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'suppression-integration-'));
    anvilDir = path.join(tempDir, '.anvil');
    await fs.mkdir(anvilDir, { recursive: true });
  });

  afterEach(async () => {
    await fs.rm(tempDir, { recursive: true, force: true });
  });

  describe('analyzeFiles with suppressions', () => {
    it('suppresses matching warnings with @anvil-ignore', async () => {
      const testFile = path.join(tempDir, 'test.ts');
      await fs.writeFile(
        testFile,
        `// @anvil-ignore AP-001: Legacy code - broad eslint-disable
// eslint-disable
const x = 1;
`
      );

      const runner = new GateRunner();
      const result = await runner.analyzeFiles([testFile], tempDir, {
        checks: ['antipattern'],
        suppressions: true,
      });

      expect(result.suppressionStats).toBeDefined();
      expect(result.suppressionStats!.total).toBeGreaterThan(0);

      const ap001Warnings = result.warnings.warnings.filter((w) => w.id === 'AP-001');
      expect(ap001Warnings.length).toBeGreaterThan(0);

      const suppressedAp001 = ap001Warnings.filter((w) => w.suppressed);
      expect(suppressedAp001.length).toBeGreaterThan(0);
    });

    it('does not apply suppressions when disabled', async () => {
      const testFile = path.join(tempDir, 'test.ts');
      await fs.writeFile(
        testFile,
        `// @anvil-ignore AP-001: Legacy code
// eslint-disable
const x = 1;
`
      );

      const runner = new GateRunner();
      const resultWithSuppressions = await runner.analyzeFiles([testFile], tempDir, {
        checks: ['antipattern'],
        suppressions: true,
      });

      const resultWithoutSuppressions = await runner.analyzeFiles([testFile], tempDir, {
        checks: ['antipattern'],
        suppressions: false,
      });

      expect(resultWithoutSuppressions.suppressionStats).toBeUndefined();

      const ap001With = resultWithSuppressions.warnings.warnings.filter((w) => w.id === 'AP-001');
      const ap001Without = resultWithoutSuppressions.warnings.warnings.filter(
        (w) => w.id === 'AP-001'
      );
      expect(ap001With.length).toBeGreaterThan(0);
      expect(ap001Without.length).toBeGreaterThan(0);

      const suppressedWith = ap001With.filter((w) => w.suppressed);
      const suppressedWithout = ap001Without.filter((w) => w.suppressed);
      expect(suppressedWith.length).toBeGreaterThan(0);
      expect(suppressedWithout.length).toBe(0);
    });

    it('accepts pre-configured suppression store', async () => {
      const testFile = path.join(tempDir, 'test.ts');
      await fs.writeFile(testFile, `const x = 1;\n`);

      const store = new SuppressionStore(anvilDir);
      await store.load();

      const runner = new GateRunner();
      const result = await runner.analyzeFiles([testFile], tempDir, {
        checks: ['antipattern'],
        suppressions: true,
        suppressionStore: store,
      });

      expect(result.warnings).toBeDefined();
      expect(result.suppressionStats).toBeDefined();
      expect(result.suppressionStats!.total).toBe(0);
    });

    it('does not suppress warnings with expired suppressions', async () => {
      const testFile = path.join(tempDir, 'test.ts');
      await fs.writeFile(
        testFile,
        `// @anvil-ignore-until 2020-01-01 AP-001: Expired suppression
// eslint-disable
const x = 1;
`
      );

      const runner = new GateRunner();
      const result = await runner.analyzeFiles([testFile], tempDir, {
        checks: ['antipattern'],
        suppressions: true,
      });

      expect(result.suppressionStats).toBeDefined();
      expect(result.suppressionStats!.expired).toBeGreaterThan(0);

      const ap001Warnings = result.warnings.warnings.filter((w) => w.id === 'AP-001');
      expect(ap001Warnings.length).toBeGreaterThan(0);

      const unsuppressedAp001 = ap001Warnings.filter((w) => !w.suppressed);
      expect(unsuppressedAp001.length).toBeGreaterThan(0);
    });
  });
});
