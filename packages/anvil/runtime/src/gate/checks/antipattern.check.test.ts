import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { AntipatternCheck } from './antipattern.check.js';
import type { CheckContext } from '../../types/gate.types.js';
import type { APSPlan } from '../../schema/aps.schema.js';
import { join } from 'node:path';
import { mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { safeCleanup } from '../../../../../../tools/test-utils/safe-cleanup.js';

const createMockPlan = (changes: Array<{ type: string; path: string }>): APSPlan => ({
  id: 'aps-12345678',
  hash: 'a'.repeat(64),
  schema_version: '0.1.0',
  intent: 'Test plan for antipattern check',
  proposed_changes: changes.map((c) => ({
    type: c.type as 'file_create' | 'file_update' | 'file_delete',
    path: c.path,
    description: `Test change to ${c.path}`,
  })),
  provenance: {
    timestamp: new Date().toISOString(),
    source: 'cli',
    version: '1.0.0',
  },
  validations: {
    required_checks: [],
    skip_checks: [],
  },
  tags: [],
});

const createMockContext = (
  workspaceRoot: string,
  checkConfig: Record<string, unknown> = {},
  options: { targetFiles?: string[]; plan?: APSPlan } = {}
): CheckContext => ({
  plan: options.plan ?? createMockPlan([{ type: 'file_update', path: 'src/index.ts' }]),
  workspace_root: workspaceRoot,
  config: {
    version: 1,
    checks: [],
    thresholds: { overall_score: 80 },
  },
  check_config: checkConfig,
  targetFiles: options.targetFiles,
});

describe('AntipatternCheck', () => {
  let check: AntipatternCheck;
  let testDir: string;

  beforeEach(() => {
    check = new AntipatternCheck();
    testDir = join(
      tmpdir(),
      `anvil-antipattern-test-${Date.now()}-${Math.random().toString(36).slice(2)}`
    );
    mkdirSync(testDir, { recursive: true });
  });

  afterEach(async () => {
    await safeCleanup(testDir);
  });

  describe('metadata', () => {
    it('should have correct name', () => {
      expect(check.name).toBe('antipattern');
    });

    it('should have a description', () => {
      expect(check.description).toBeTruthy();
      expect(check.description).toContain('anti-pattern');
    });
  });

  describe('run() with no files', () => {
    it('should return success when no files to scan', async () => {
      const context = createMockContext(testDir, {}, { targetFiles: [] });
      const result = await check.run(context);

      expect(result.passed).toBe(true);
      expect(result.score).toBe(100);
      expect(result.message).toContain('No files to scan');
    });

    it('should return success when files filtered by extension', async () => {
      const filePath = join(testDir, 'file.txt');
      writeFileSync(filePath, 'const x: any = 1;');

      const context = createMockContext(
        testDir,
        { extensions: ['.ts'] },
        { targetFiles: [filePath] }
      );
      const result = await check.run(context);

      expect(result.passed).toBe(true);
      expect(result.details?.filesScanned).toBe(0);
    });
  });

  describe('run() with clean files', () => {
    it('should pass when files have no anti-patterns', async () => {
      const filePath = join(testDir, 'clean.ts');
      writeFileSync(filePath, 'const x = 1;\nexport default x;');

      const context = createMockContext(testDir, {}, { targetFiles: [filePath] });
      const result = await check.run(context);

      expect(result.passed).toBe(true);
      expect(result.score).toBe(100);
      expect(result.details?.warnings?.warnings).toHaveLength(0);
      expect(result.details?.filesScanned).toBe(1);
    });
  });

  describe('run() detecting anti-patterns', () => {
    it('should detect eslint-disable comments', async () => {
      const filePath = join(testDir, 'bad.ts');
      writeFileSync(filePath, '/* eslint-disable */\nconst x = 1;');

      const context = createMockContext(testDir, {}, { targetFiles: [filePath] });
      const result = await check.run(context);

      expect(result.details?.warnings?.warnings.length).toBeGreaterThan(0);
      expect(result.details?.warnings?.warnings[0].id).toBe('AP-001');
    });

    it('should detect any type usage', async () => {
      const filePath = join(testDir, 'any.ts');
      writeFileSync(filePath, 'const x: any = 1;');

      const context = createMockContext(testDir, {}, { targetFiles: [filePath] });
      const result = await check.run(context);

      const anyWarnings = result.details?.warnings?.warnings.filter(
        (w: { id: string }) => w.id === 'AP-003'
      );
      expect(anyWarnings?.length).toBeGreaterThan(0);
    });

    it('should detect @ts-ignore comments', async () => {
      const filePath = join(testDir, 'ignore.ts');
      writeFileSync(filePath, '// @ts-ignore\nconst x = badCall();');

      const context = createMockContext(testDir, {}, { targetFiles: [filePath] });
      const result = await check.run(context);

      const ignoreWarnings = result.details?.warnings?.warnings.filter(
        (w: { id: string }) => w.id === 'AP-004'
      );
      expect(ignoreWarnings?.length).toBeGreaterThan(0);
    });

    it('should detect empty catch blocks', async () => {
      const filePath = join(testDir, 'catch.ts');
      writeFileSync(filePath, 'try { x() } catch (e) {}');

      const context = createMockContext(testDir, {}, { targetFiles: [filePath] });
      const result = await check.run(context);

      const catchWarnings = result.details?.warnings?.warnings.filter(
        (w: { id: string }) => w.id === 'AP-006'
      );
      expect(catchWarnings?.length).toBeGreaterThan(0);
    });
  });

  describe('scoring', () => {
    it('should reduce score for warnings', async () => {
      const filePath = join(testDir, 'many.ts');
      writeFileSync(filePath, 'const x: any = 1;\nconst y: any = 2;');

      const context = createMockContext(testDir, {}, { targetFiles: [filePath] });
      const result = await check.run(context);

      expect(result.score).toBeLessThan(100);
    });

    it('should fail when severity threshold is met', async () => {
      const filePath = join(testDir, 'fail.ts');
      writeFileSync(filePath, '/* eslint-disable */');

      const context = createMockContext(
        testDir,
        { severityThreshold: 'warning' },
        { targetFiles: [filePath] }
      );
      const result = await check.run(context);

      expect(result.passed).toBe(false);
    });

    it('should pass when severity is below threshold', async () => {
      const filePath = join(testDir, 'pass.ts');
      writeFileSync(filePath, '/* eslint-disable */');

      const context = createMockContext(
        testDir,
        { severityThreshold: 'error' },
        { targetFiles: [filePath] }
      );
      const result = await check.run(context);

      expect(result.passed).toBe(true);
    });
  });

  describe('configuration', () => {
    it('should respect custom extensions', async () => {
      const filePath = join(testDir, 'file.ts');
      writeFileSync(filePath, 'const x: any = 1;');

      const context = createMockContext(
        testDir,
        { extensions: ['.ts'] },
        { targetFiles: [filePath] }
      );
      const result = await check.run(context);

      expect(result.details?.filesScanned).toBe(1);
    });

    it('should include opt-in patterns when configured', async () => {
      const filePath = join(testDir, 'console.ts');
      writeFileSync(filePath, 'console.log("test");');

      const context = createMockContext(
        testDir,
        { includeOptIn: true },
        { targetFiles: [filePath] }
      );
      const result = await check.run(context);

      const consoleWarnings = result.details?.warnings?.warnings.filter(
        (w: { id: string }) => w.id === 'AP-007'
      );
      expect(consoleWarnings?.length).toBeGreaterThan(0);
    });
  });

  describe('error handling', () => {
    it('should skip non-existent files gracefully', async () => {
      const nonExistentPath = join(testDir, 'does-not-exist.ts');

      const context = createMockContext(testDir, {}, { targetFiles: [nonExistentPath] });
      const result = await check.run(context);

      expect(result.passed).toBe(true);
      expect(result.details?.filesScanned).toBe(0);
    });
  });
});
