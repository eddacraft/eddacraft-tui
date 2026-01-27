import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ArchitectureCheck } from './architecture.check.js';
import { CircularDetector } from './architecture/circular-detector.js';
import { LayerValidator } from './architecture/layer-validator.js';
import { DependencyAnalyzer, type CruiserViolation } from './architecture/dependency-analyzer.js';
import type { CheckContext } from '../../types/gate.types.js';
import type { APSPlan } from '../../schema/aps.schema.js';
import { mkdirSync, writeFileSync, rmSync, existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';

const createMockPlan = (changes: Array<{ type: string; path: string }>): APSPlan => ({
  id: 'aps-12345678',
  hash: 'a'.repeat(64),
  schema_version: '0.1.0',
  intent: 'Test plan',
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

// Mock context factory
const createMockContext = (
  workspaceRoot: string,
  checkConfig: Record<string, unknown> = {},
  options: { plan?: APSPlan; targetFiles?: string[]; fullScan?: boolean } = {}
): CheckContext => ({
  plan: options.plan || createMockPlan([{ type: 'file_update', path: 'src/index.ts' }]),
  workspace_root: workspaceRoot,
  config: {
    version: 1,
    checks: [],
    thresholds: { overall_score: 80 },
  },
  check_config: checkConfig,
  targetFiles: options.targetFiles,
  fullScan: options.fullScan,
});

describe('ArchitectureCheck', () => {
  let check: ArchitectureCheck;
  let testDir: string;
  let analyzer: DependencyAnalyzer;

  beforeEach(() => {
    check = new ArchitectureCheck();
    testDir = join(
      tmpdir(),
      `anvil-architecture-test-${Date.now()}-${Math.random().toString(36).slice(2)}`
    );
    mkdirSync(testDir, { recursive: true });

    // Access the private analyzer for mocking
    // eslint-disable-next-line anvil/no-any-in-tests -- accessing private member for test setup
    analyzer = (check as any).analyzer;
  });

  afterEach(() => {
    vi.restoreAllMocks();
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true, force: true });
    }
  });

  describe('metadata', () => {
    it('should have correct name', () => {
      expect(check.name).toBe('architecture');
    });

    it('should have a description', () => {
      expect(check.description).toBeTruthy();
      expect(check.description).toContain('dependency-cruiser');
    });
  });

  describe('run() - dependency-cruiser availability', () => {
    it('should skip gracefully when dependency-cruiser is not installed', async () => {
      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({
        success: false,
        error: 'dependency-cruiser not installed',
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      expect(result.passed).toBe(true);
      expect(result.score).toBe(100);
      expect(result.details?.skipped).toBe(true);
      expect(result.message).toContain('dependency-cruiser not installed');
      expect(result.details?.warnings).toBeDefined();
    });

    it('should proceed when dependency-cruiser is available', async () => {
      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [],
            error: 0,
            warn: 0,
            info: 0,
            totalCruised: 5,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      expect(result.passed).toBe(true);
      expect(result.details?.skipped).toBe(undefined);
      expect(result.details?.totalModulesCruised).toBe(5);
    });
  });

  describe('run() - configuration loading', () => {
    it('should use custom config file when specified', async () => {
      const customConfigPath = join(testDir, 'custom.dependency-cruiser.js');
      writeFileSync(customConfigPath, 'module.exports = { validate: true };');

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      const loadConfigSpy = vi.spyOn(analyzer, 'loadConfig').mockResolvedValue({
        validate: true,
      });
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: { violations: [], error: 0, warn: 0, info: 0, totalCruised: 0 },
          modules: [],
        },
      });

      const context = createMockContext(testDir, {
        config_file: 'custom.dependency-cruiser.js',
      });
      await check.run(context);

      expect(loadConfigSpy).toHaveBeenCalledWith(testDir, 'custom.dependency-cruiser.js');
    });

    it('should use default config when no config file exists', async () => {
      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      const getDefaultSpy = vi.spyOn(analyzer, 'getDefaultCruiseOptions');
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: { violations: [], error: 0, warn: 0, info: 0, totalCruised: 0 },
          modules: [],
        },
      });

      const context = createMockContext(testDir);
      await check.run(context);

      expect(getDefaultSpy).toHaveBeenCalled();
    });

    it('should fail when config file exists but cannot be loaded', async () => {
      const configPath = join(testDir, '.anvil', 'dependency-cruiser.js');
      mkdirSync(join(testDir, '.anvil'), { recursive: true });
      writeFileSync(configPath, 'invalid javascript {{{');

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);

      const context = createMockContext(testDir);
      const result = await check.run(context);

      expect(result.passed).toBe(false);
      expect(result.message).toContain('Failed to load');
    });
  });

  describe('run() - circular dependency detection', () => {
    it('should detect circular dependencies', async () => {
      const circularViolation: CruiserViolation = {
        from: 'src/a.ts',
        to: 'src/b.ts',
        rule: { name: 'no-circular', severity: 'error' },
        cycle: ['src/a.ts', 'src/b.ts', 'src/a.ts'],
      };

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [circularViolation],
            error: 1,
            warn: 0,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      expect(result.passed).toBe(false);
      expect(result.score).toBe(85); // 100 - 15 penalty for error
      expect(result.details?.violationsByType?.circular).toBe(1);
      expect(result.details?.warnings?.warnings).toHaveLength(1);
      expect(result.details?.warnings?.warnings[0].id).toBe('ARCH-001');
      expect(result.details?.warnings?.warnings[0].severity).toBe('error');
    });

    it('should pass with circular warnings when fail_on_circular is false', async () => {
      const circularViolation: CruiserViolation = {
        from: 'src/a.ts',
        to: 'src/b.ts',
        rule: { name: 'no-circular', severity: 'warn' },
        cycle: ['src/a.ts', 'src/b.ts', 'src/a.ts'],
      };

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [circularViolation],
            error: 0,
            warn: 1,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir, { fail_on_circular: false });
      const result = await check.run(context);

      expect(result.passed).toBe(true);
      expect(result.score).toBe(95); // 100 - 5 penalty for warn
      expect(result.details?.violationsByType?.circular).toBe(1);
    });
  });

  describe('run() - orphan module detection', () => {
    it('should detect orphaned modules', async () => {
      const orphanViolation: CruiserViolation = {
        from: 'src/orphan.ts',
        to: 'src/orphan.ts',
        rule: { name: 'no-orphans', severity: 'warn' },
      };

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [orphanViolation],
            error: 0,
            warn: 1,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      expect(result.passed).toBe(true); // orphans don't fail by default
      expect(result.details?.violationsByType?.orphan).toBe(1);
      expect(result.details?.warnings?.warnings[0].id).toBe('ARCH-002');
    });

    it('should fail on orphans when fail_on_orphan is true', async () => {
      const orphanViolation: CruiserViolation = {
        from: 'src/orphan.ts',
        to: 'src/orphan.ts',
        rule: { name: 'no-orphans', severity: 'error' },
      };

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [orphanViolation],
            error: 1,
            warn: 0,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir, { fail_on_orphan: true });
      const result = await check.run(context);

      expect(result.passed).toBe(false);
      expect(result.details?.violationsByType?.orphan).toBe(1);
    });
  });

  describe('run() - layer violations', () => {
    it('should detect layer boundary violations', async () => {
      const layerViolation: CruiserViolation = {
        from: 'src/domain/model.ts',
        to: 'src/infrastructure/db.ts',
        rule: { name: 'no-layer-crossing', severity: 'error' },
      };

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [layerViolation],
            error: 1,
            warn: 0,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      expect(result.passed).toBe(false);
      expect(result.details?.violationsByType?.layer).toBe(1);
      expect(result.details?.warnings?.warnings[0].id).toBe('ARCH-003');
    });

    it('should detect boundary violations', async () => {
      const boundaryViolation: CruiserViolation = {
        from: 'src/module-a/index.ts',
        to: 'src/module-b/internal.ts',
        rule: { name: 'no-boundary-cross', severity: 'error' },
      };

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [boundaryViolation],
            error: 1,
            warn: 0,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      expect(result.passed).toBe(false);
      expect(result.details?.violationsByType?.layer).toBe(1);
      expect(result.details?.warnings?.warnings[0].id).toBe('ARCH-003');
    });
  });

  describe('run() - baseline comparison', () => {
    it('should include drift information when baseline would exist', async () => {
      // Note: This test validates the baseline logic but doesn't actually load a baseline
      // because loadBaseline looks in specific locations. In a real scenario, a baseline
      // would be created via the baseline command.

      const violation: CruiserViolation = {
        from: 'src/a.ts',
        to: 'src/b.ts',
        rule: { name: 'no-circular', severity: 'error' },
        cycle: ['src/a.ts', 'src/b.ts'],
      };

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [violation],
            error: 1,
            warn: 0,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      // Without a baseline, all violations are new
      expect(result.details?.violationCount).toBe(1);
      expect(result.details?.newViolationCount).toBe(1);
      expect(result.details?.baselineLoaded).toBe(false);
    });

    it('should mark all violations as new when no baseline exists', async () => {
      const violation: CruiserViolation = {
        from: 'src/a.ts',
        to: 'src/b.ts',
        rule: { name: 'no-circular', severity: 'error' },
        cycle: ['src/a.ts', 'src/b.ts'],
      };

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [violation],
            error: 1,
            warn: 0,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      expect(result.details?.newViolationCount).toBe(1);
      expect(result.details?.violationCount).toBe(1);
      expect(result.details?.baselineLoaded).toBe(false);
    });
  });

  describe('run() - severity threshold', () => {
    it('should pass errors when threshold is error', async () => {
      const violation: CruiserViolation = {
        from: 'src/a.ts',
        to: 'src/b.ts',
        rule: { name: 'some-rule', severity: 'error' },
      };

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [violation],
            error: 1,
            warn: 0,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir, { severity_threshold: 'error' });
      const result = await check.run(context);

      expect(result.passed).toBe(false);
    });

    it('should fail on warnings when threshold is warn', async () => {
      const violation: CruiserViolation = {
        from: 'src/a.ts',
        to: 'src/b.ts',
        rule: { name: 'some-rule', severity: 'warn' },
      };

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [violation],
            error: 0,
            warn: 1,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir, { severity_threshold: 'warn' });
      const result = await check.run(context);

      expect(result.passed).toBe(false);
    });

    it('should fail on info when threshold is info', async () => {
      const violation: CruiserViolation = {
        from: 'src/a.ts',
        to: 'src/b.ts',
        rule: { name: 'some-rule', severity: 'info' },
      };

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [violation],
            error: 0,
            warn: 0,
            info: 1,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir, { severity_threshold: 'info' });
      const result = await check.run(context);

      expect(result.passed).toBe(false);
    });
  });

  describe('run() - scoring', () => {
    it('should calculate score based on violation severity', async () => {
      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);

      // Test with 1 error (15 penalty)
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [
              {
                from: 'src/a.ts',
                to: 'src/b.ts',
                rule: { name: 'rule', severity: 'error' },
              },
            ],
            error: 1,
            warn: 0,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const result1 = await check.run(createMockContext(testDir));
      expect(result1.score).toBe(85);

      // Test with 1 warning (5 penalty)
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [
              {
                from: 'src/a.ts',
                to: 'src/b.ts',
                rule: { name: 'rule', severity: 'warn' },
              },
            ],
            error: 0,
            warn: 1,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const result2 = await check.run(createMockContext(testDir));
      expect(result2.score).toBe(95);

      // Test with 1 info (1 penalty)
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [
              {
                from: 'src/a.ts',
                to: 'src/b.ts',
                rule: { name: 'rule', severity: 'info' },
              },
            ],
            error: 0,
            warn: 0,
            info: 1,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const result3 = await check.run(createMockContext(testDir));
      expect(result3.score).toBe(99);
    });

    it('should not go below 0 score', async () => {
      // Create 10 error violations (150 penalty)
      const violations: CruiserViolation[] = Array.from({ length: 10 }, (_, i) => ({
        from: `src/${i}.ts`,
        to: `src/${i + 1}.ts`,
        rule: { name: 'rule', severity: 'error' },
      }));

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations,
            error: 10,
            warn: 0,
            info: 0,
            totalCruised: 15,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      expect(result.score).toBe(0);
    });
  });

  describe('run() - scope configuration', () => {
    it('should use affected scope by default', async () => {
      // Create actual files so they're not filtered out
      const srcDir = join(testDir, 'src');
      mkdirSync(srcDir, { recursive: true });
      writeFileSync(join(srcDir, 'index.ts'), 'export default {};');
      writeFileSync(join(srcDir, 'utils.ts'), 'export const util = 1;');

      const plan = createMockPlan([
        { type: 'file_update', path: 'src/index.ts' },
        { type: 'file_update', path: 'src/utils.ts' },
      ]);

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      const analyzeSpy = vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: { violations: [], error: 0, warn: 0, info: 0, totalCruised: 2 },
          modules: [],
        },
      });

      const context = createMockContext(testDir, {}, { plan });
      await check.run(context);

      // Should analyze affected files
      const callArgs = analyzeSpy.mock.calls[0][0];
      expect(callArgs).toHaveLength(2);
      expect(callArgs.some((p: string) => p.endsWith('src/index.ts'))).toBe(true);
      expect(callArgs.some((p: string) => p.endsWith('src/utils.ts'))).toBe(true);
    });

    it('should use full scope when configured', async () => {
      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      const analyzeSpy = vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: { violations: [], error: 0, warn: 0, info: 0, totalCruised: 100 },
          modules: [],
        },
      });

      const context = createMockContext(testDir, { scope: 'full' });
      await check.run(context);

      // Should use include patterns for full scan
      const callArgs = analyzeSpy.mock.calls[0][0];
      expect(callArgs).toContain('src/**/*.ts');
    });

    it('should use fullScan flag to override scope', async () => {
      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      const analyzeSpy = vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: { violations: [], error: 0, warn: 0, info: 0, totalCruised: 100 },
          modules: [],
        },
      });

      const context = createMockContext(testDir, { scope: 'affected' }, { fullScan: true });
      await check.run(context);

      // Should use include patterns despite affected scope
      const callArgs = analyzeSpy.mock.calls[0][0];
      expect(callArgs).toContain('src/**/*.ts');
    });
  });

  describe('run() - file filtering', () => {
    it('should only analyze TypeScript/JavaScript files', async () => {
      // Create actual files
      const srcDir = join(testDir, 'src');
      mkdirSync(srcDir, { recursive: true });
      writeFileSync(join(srcDir, 'index.ts'), 'export default {};');
      writeFileSync(join(srcDir, 'utils.js'), 'export const util = 1;');
      writeFileSync(join(testDir, 'README.md'), '# README');

      const plan = createMockPlan([
        { type: 'file_update', path: 'src/index.ts' },
        { type: 'file_update', path: 'README.md' },
        { type: 'file_update', path: 'src/utils.js' },
        { type: 'file_create', path: 'docs/guide.md' },
      ]);

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      const analyzeSpy = vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: { violations: [], error: 0, warn: 0, info: 0, totalCruised: 2 },
          modules: [],
        },
      });

      const context = createMockContext(testDir, {}, { plan });
      await check.run(context);

      const callArgs = analyzeSpy.mock.calls[0][0];
      expect(callArgs.some((p: string) => p.endsWith('src/index.ts'))).toBe(true);
      expect(callArgs.some((p: string) => p.endsWith('src/utils.js'))).toBe(true);
      expect(callArgs.every((p: string) => !p.endsWith('README.md'))).toBe(true);
    });

    it('should respect exclude patterns', async () => {
      // Create actual files
      const srcDir = join(testDir, 'src');
      mkdirSync(srcDir, { recursive: true });
      writeFileSync(join(srcDir, 'index.test.ts'), 'test();');
      writeFileSync(join(srcDir, 'utils.spec.ts'), 'spec();');
      writeFileSync(join(srcDir, 'component.ts'), 'export {};');

      const plan = createMockPlan([
        { type: 'file_update', path: 'src/index.test.ts' },
        { type: 'file_update', path: 'src/utils.spec.ts' },
        { type: 'file_update', path: 'src/component.ts' },
      ]);

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      const analyzeSpy = vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: { violations: [], error: 0, warn: 0, info: 0, totalCruised: 1 },
          modules: [],
        },
      });

      const context = createMockContext(testDir, {}, { plan });
      await check.run(context);

      const callArgs = analyzeSpy.mock.calls[0][0];
      expect(callArgs.some((p: string) => p.endsWith('src/component.ts'))).toBe(true);
      expect(callArgs.every((p: string) => !p.endsWith('.test.ts'))).toBe(true);
      expect(callArgs.every((p: string) => !p.endsWith('.spec.ts'))).toBe(true);
    });

    it('should return success when no analysable files exist', async () => {
      // Create non-TS/JS files
      writeFileSync(join(testDir, 'README.md'), '# README');
      writeFileSync(join(testDir, 'package.json'), '{}');

      const plan = createMockPlan([
        { type: 'file_update', path: 'README.md' },
        { type: 'file_update', path: 'package.json' },
      ]);

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'getDefaultCruiseOptions');
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: { violations: [], error: 0, warn: 0, info: 0, totalCruised: 0 },
          modules: [],
        },
      });

      const context = createMockContext(testDir, {}, { plan });
      const result = await check.run(context);

      expect(result.passed).toBe(true);
      expect(result.score).toBe(100);
      // Message will vary based on whether files were filtered or not
      expect(result.message).toBeTruthy();
    });
  });

  describe('run() - warning result format', () => {
    it('should include properly formatted warnings', async () => {
      const violation: CruiserViolation = {
        from: 'src/a.ts',
        to: 'src/b.ts',
        rule: { name: 'no-circular', severity: 'error' },
        cycle: ['src/a.ts', 'src/b.ts'],
      };

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [violation],
            error: 1,
            warn: 0,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      const warnings = result.details?.warnings?.warnings;
      expect(warnings).toHaveLength(1);
      expect(warnings![0]).toMatchObject({
        id: 'ARCH-001',
        category: 'architecture',
        severity: 'error',
        confidence: 'high',
        title: 'Circular dependency detected',
        location: {
          file: 'src/a.ts',
          line: 1,
        },
        pattern: 'no-circular',
      });
      expect(warnings![0].fingerprint).toBeDefined();
    });

    it('should include patterns_checked in warning result', async () => {
      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: { violations: [], error: 0, warn: 0, info: 0, totalCruised: 10 },
          modules: [],
        },
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      expect(result.details?.warnings?.patterns_checked).toBeInstanceOf(Array);
    });

    it('should include summary counts in warning result', async () => {
      const violations: CruiserViolation[] = [
        {
          from: 'src/a.ts',
          to: 'src/b.ts',
          rule: { name: 'no-circular', severity: 'error' },
          cycle: ['src/a.ts', 'src/b.ts'],
        },
        {
          from: 'src/c.ts',
          to: 'src/d.ts',
          rule: { name: 'no-orphans', severity: 'warn' },
        },
        {
          from: 'src/e.ts',
          to: 'src/f.ts',
          rule: { name: 'some-rule', severity: 'info' },
        },
      ];

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations,
            error: 1,
            warn: 1,
            info: 1,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      const summary = result.details?.warnings?.summary;
      expect(summary?.total).toBe(3);
      expect(summary?.errors).toBe(1);
      expect(summary?.warnings).toBe(1);
      expect(summary?.info).toBe(1);
    });
  });

  describe('run() - architecture context', () => {
    it('should include architecture context in details', async () => {
      const violation: CruiserViolation = {
        from: 'src/a.ts',
        to: 'src/b.ts',
        rule: { name: 'no-circular', severity: 'error' },
        cycle: ['src/a.ts', 'src/b.ts'],
      };

      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: true,
        result: {
          summary: {
            violations: [violation],
            error: 1,
            warn: 0,
            info: 0,
            totalCruised: 10,
          },
          modules: [],
        },
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      const archContext = result.details?.architectureContext;
      expect(archContext).toBeDefined();
      expect(archContext?.violations).toHaveLength(1);
      expect(archContext?.summary).toMatchObject({
        total_modules: 10,
        total_violations: 1,
        new_violations: 1,
        error_count: 1,
        warn_count: 0,
        info_count: 0,
        circular_count: 1,
        baseline_loaded: false,
      });
      expect(archContext?.config).toMatchObject({
        config_file: '.anvil/dependency-cruiser.js',
        scope: 'affected',
        severity_threshold: 'error',
      });
    });
  });

  describe('run() - error handling', () => {
    it('should handle analysis failure gracefully', async () => {
      vi.spyOn(analyzer, 'loadCruiser').mockResolvedValue({ success: true });
      vi.spyOn(analyzer, 'loadConfig').mockResolvedValue(null);
      vi.spyOn(analyzer, 'analyze').mockResolvedValue({
        success: false,
        error: 'Analysis failed due to syntax error',
      });

      const context = createMockContext(testDir);
      const result = await check.run(context);

      expect(result.passed).toBe(false);
      expect(result.message).toContain('Dependency analysis failed');
    });

    it('should handle unexpected errors', async () => {
      vi.spyOn(analyzer, 'loadCruiser').mockRejectedValue(new Error('Unexpected error'));

      const context = createMockContext(testDir);
      const result = await check.run(context);

      expect(result.passed).toBe(false);
      expect(result.message).toContain('failed unexpectedly');
    });
  });

  describe('CircularDetector', () => {
    let detector: CircularDetector;

    beforeEach(() => {
      detector = new CircularDetector();
    });

    describe('categoriseViolation', () => {
      it('should categorize circular dependencies', () => {
        const violation: CruiserViolation = {
          from: 'src/a.ts',
          to: 'src/b.ts',
          rule: { name: 'no-circular', severity: 'error' },
          cycle: ['src/a.ts', 'src/b.ts'],
        };

        expect(detector.categoriseViolation(violation)).toBe('circular');
      });

      it('should categorize orphan violations', () => {
        const violation: CruiserViolation = {
          from: 'src/orphan.ts',
          to: 'src/orphan.ts',
          rule: { name: 'no-orphans', severity: 'warn' },
        };

        expect(detector.categoriseViolation(violation)).toBe('orphan');
      });

      it('should categorize layer violations', () => {
        const violation: CruiserViolation = {
          from: 'src/a.ts',
          to: 'src/b.ts',
          rule: { name: 'no-layer-crossing', severity: 'error' },
        };

        expect(detector.categoriseViolation(violation)).toBe('layer');
      });

      it('should categorize boundary violations', () => {
        const violation: CruiserViolation = {
          from: 'src/a.ts',
          to: 'src/b.ts',
          rule: { name: 'no-boundary-cross', severity: 'error' },
        };

        expect(detector.categoriseViolation(violation)).toBe('layer');
      });

      it('should categorize other violations', () => {
        const violation: CruiserViolation = {
          from: 'src/a.ts',
          to: 'src/b.ts',
          rule: { name: 'custom-rule', severity: 'error' },
        };

        expect(detector.categoriseViolation(violation)).toBe('other');
      });
    });

    describe('countViolationsByType', () => {
      it('should count violations by type', () => {
        const violations: CruiserViolation[] = [
          {
            from: 'src/a.ts',
            to: 'src/b.ts',
            rule: { name: 'no-circular', severity: 'error' },
            cycle: ['src/a.ts', 'src/b.ts'],
          },
          {
            from: 'src/c.ts',
            to: 'src/c.ts',
            rule: { name: 'no-orphans', severity: 'warn' },
          },
          {
            from: 'src/d.ts',
            to: 'src/e.ts',
            rule: { name: 'no-layer-crossing', severity: 'error' },
          },
          {
            from: 'src/f.ts',
            to: 'src/g.ts',
            rule: { name: 'custom-rule', severity: 'info' },
          },
        ];

        const counts = detector.countViolationsByType(violations);

        expect(counts.circular).toBe(1);
        expect(counts.orphan).toBe(1);
        expect(counts.layer).toBe(1);
        expect(counts.other).toBe(1);
      });
    });
  });

  describe('LayerValidator', () => {
    let validator: LayerValidator;

    beforeEach(() => {
      validator = new LayerValidator();
    });

    describe('parseConfig', () => {
      it('should parse valid configuration', () => {
        const config = validator.parseConfig({
          config_file: 'custom.js',
          scope: 'full',
          severity_threshold: 'warn',
          fail_on_circular: false,
          fail_on_orphan: true,
        });

        expect(config.config_file).toBe('custom.js');
        expect(config.scope).toBe('full');
        expect(config.severity_threshold).toBe('warn');
        expect(config.fail_on_circular).toBe(false);
        expect(config.fail_on_orphan).toBe(true);
      });

      it('should use defaults for missing values', () => {
        const config = validator.parseConfig({});

        expect(config.config_file).toBe('.anvil/dependency-cruiser.js');
        expect(config.scope).toBe('affected');
        expect(config.severity_threshold).toBe('error');
        expect(config.fail_on_circular).toBe(true);
        expect(config.fail_on_orphan).toBe(false);
      });

      it('should parse custom patterns', () => {
        const config = validator.parseConfig({
          include_patterns: ['lib/**/*.ts'],
          exclude_patterns: ['**/fixtures/**'],
        });

        expect(config.include_patterns).toEqual(['lib/**/*.ts']);
        expect(config.exclude_patterns).toEqual(['**/fixtures/**']);
      });
    });

    describe('isAnalysableFile', () => {
      it('should accept TypeScript files', () => {
        const config = validator.parseConfig({});

        expect(validator.isAnalysableFile('src/index.ts', config)).toBe(true);
        expect(validator.isAnalysableFile('src/component.tsx', config)).toBe(true);
      });

      it('should accept JavaScript files', () => {
        const config = validator.parseConfig({});

        expect(validator.isAnalysableFile('src/utils.js', config)).toBe(true);
        expect(validator.isAnalysableFile('src/component.jsx', config)).toBe(true);
        expect(validator.isAnalysableFile('src/module.mjs', config)).toBe(true);
        expect(validator.isAnalysableFile('src/config.cjs', config)).toBe(true);
      });

      it('should reject non-code files', () => {
        const config = validator.parseConfig({});

        expect(validator.isAnalysableFile('README.md', config)).toBe(false);
        expect(validator.isAnalysableFile('package.json', config)).toBe(false);
        expect(validator.isAnalysableFile('styles.css', config)).toBe(false);
      });

      it('should respect exclude patterns', () => {
        const config = validator.parseConfig({});

        expect(validator.isAnalysableFile('src/index.test.ts', config)).toBe(false);
        expect(validator.isAnalysableFile('src/utils.spec.ts', config)).toBe(false);
        // __tests__ pattern needs full path to match the glob **/__tests__/**
        expect(validator.isAnalysableFile('src/__tests__/index.ts', config)).toBe(false);
      });
    });

    describe('buildMessage', () => {
      it('should build message for no violations', () => {
        const message = validator.buildMessage([], 50, true);

        expect(message).toContain('passed');
        expect(message).toContain('50 modules');
        expect(message).toContain('no violations');
      });

      it('should build message for errors only', () => {
        const violations: CruiserViolation[] = [
          {
            from: 'src/a.ts',
            to: 'src/b.ts',
            rule: { name: 'no-circular', severity: 'error' },
          },
          {
            from: 'src/c.ts',
            to: 'src/d.ts',
            rule: { name: 'no-layer', severity: 'error' },
          },
        ];

        const message = validator.buildMessage(violations, 50, false);

        expect(message).toContain('failed');
        expect(message).toContain('2 errors');
      });

      it('should build message for mixed severities', () => {
        const violations: CruiserViolation[] = [
          {
            from: 'src/a.ts',
            to: 'src/b.ts',
            rule: { name: 'no-circular', severity: 'error' },
          },
          {
            from: 'src/c.ts',
            to: 'src/d.ts',
            rule: { name: 'no-orphans', severity: 'warn' },
          },
          {
            from: 'src/e.ts',
            to: 'src/f.ts',
            rule: { name: 'some-rule', severity: 'info' },
          },
        ];

        const message = validator.buildMessage(violations, 50, false);

        expect(message).toContain('1 error');
        expect(message).toContain('1 warning');
        expect(message).toContain('1 info');
      });
    });
  });
});
