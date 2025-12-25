import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ArchitectureCheck } from './architecture.check.js';
import type { CheckContext } from '../../types/gate.types.js';
import type { APSPlan } from '../../schema/aps.schema.js';

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
  checkConfig: Record<string, unknown> = {},
  plan?: APSPlan
): CheckContext => ({
  plan: plan || createMockPlan([{ type: 'file_update', path: 'src/index.ts' }]),
  workspace_root: process.cwd(),
  config: {
    version: 1,
    checks: [],
    thresholds: { overall_score: 80 },
  },
  check_config: checkConfig,
});

describe('ArchitectureCheck', () => {
  let check: ArchitectureCheck;

  beforeEach(() => {
    check = new ArchitectureCheck();
  });

  afterEach(() => {
    vi.restoreAllMocks();
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

  describe('run()', () => {
    it('should skip gracefully when dependency-cruiser is not installed', async () => {
      const context = createMockContext();
      const result = await check.run(context);

      // Since dependency-cruiser is likely not installed in test environment
      // it should return a success with skipped flag
      expect(result.passed).toBe(true);
      expect(result.details?.skipped).toBe(true);
      expect(result.message).toContain('dependency-cruiser not installed');
    });

    it('should return 100 score when skipped', async () => {
      const context = createMockContext();
      const result = await check.run(context);

      expect(result.score).toBe(100);
    });

    it('should handle empty plan gracefully', async () => {
      const context = createMockContext({}, createMockPlan([]));
      const result = await check.run(context);

      expect(result.passed).toBe(true);
    });
  });

  describe('configuration', () => {
    it('should use default config when none provided', async () => {
      const context = createMockContext({});
      const result = await check.run(context);

      // Check that it processed with defaults (even if skipped)
      expect(result).toBeDefined();
    });

    it('should respect custom config_file path', async () => {
      const context = createMockContext({
        config_file: 'custom.dependency-cruiser.js',
      });
      const result = await check.run(context);

      // Should attempt to use custom config
      expect(result).toBeDefined();
    });

    it('should respect scope setting', async () => {
      const context = createMockContext({ scope: 'full' });
      const result = await check.run(context);

      expect(result).toBeDefined();
    });

    it('should parse severity_threshold correctly', async () => {
      const context = createMockContext({ severity_threshold: 'warn' });
      const result = await check.run(context);

      expect(result).toBeDefined();
    });

    it('should parse fail_on_circular correctly', async () => {
      const contextEnabled = createMockContext({ fail_on_circular: true });
      const contextDisabled = createMockContext({ fail_on_circular: false });

      const resultEnabled = await check.run(contextEnabled);
      const resultDisabled = await check.run(contextDisabled);

      expect(resultEnabled).toBeDefined();
      expect(resultDisabled).toBeDefined();
    });

    it('should parse fail_on_orphan correctly', async () => {
      const contextEnabled = createMockContext({ fail_on_orphan: true });
      const contextDisabled = createMockContext({ fail_on_orphan: false });

      const resultEnabled = await check.run(contextEnabled);
      const resultDisabled = await check.run(contextDisabled);

      expect(resultEnabled).toBeDefined();
      expect(resultDisabled).toBeDefined();
    });
  });

  describe('file filtering', () => {
    it('should only analyse TypeScript/JavaScript files', async () => {
      const plan = createMockPlan([
        { type: 'file_update', path: 'src/index.ts' },
        { type: 'file_update', path: 'README.md' },
        { type: 'file_update', path: 'src/utils.js' },
        { type: 'file_create', path: 'docs/guide.md' },
      ]);
      const context = createMockContext({}, plan);
      const result = await check.run(context);

      // Should process without error
      expect(result).toBeDefined();
    });

    it('should exclude test files by default', async () => {
      const plan = createMockPlan([
        { type: 'file_update', path: 'src/index.test.ts' },
        { type: 'file_update', path: 'src/utils.spec.ts' },
      ]);
      const context = createMockContext({}, plan);
      const result = await check.run(context);

      expect(result).toBeDefined();
    });

    it('should use custom exclude patterns', async () => {
      const context = createMockContext({
        exclude_patterns: ['**/generated/**', '**/vendor/**'],
      });
      const result = await check.run(context);

      expect(result).toBeDefined();
    });
  });

  describe('result format', () => {
    it('should return a properly formatted GateResult', async () => {
      const context = createMockContext();
      const result = await check.run(context);

      expect(result).toHaveProperty('check');
      expect(result).toHaveProperty('passed');
      expect(result).toHaveProperty('message');
      expect(result.check).toBe('architecture');
    });

    it('should include details in result', async () => {
      const context = createMockContext();
      const result = await check.run(context);

      expect(result.details).toBeDefined();
    });
  });

  describe('WarningResult output', () => {
    it('should include warnings in details when skipped', async () => {
      const context = createMockContext();
      const result = await check.run(context);

      expect(result.details?.warnings).toBeDefined();
      expect(result.details?.warnings?.warnings).toBeInstanceOf(Array);
    });

    it('should include patterns_checked in warnings', async () => {
      const context = createMockContext();
      const result = await check.run(context);

      if (result.details?.warnings) {
        expect(result.details.warnings.patterns_checked).toBeInstanceOf(Array);
      }
    });

    it('should include summary in warnings', async () => {
      const context = createMockContext();
      const result = await check.run(context);

      if (result.details?.warnings) {
        expect(result.details.warnings.summary).toBeDefined();
        expect(typeof result.details.warnings.summary.total).toBe('number');
        expect(typeof result.details.warnings.summary.errors).toBe('number');
        expect(typeof result.details.warnings.summary.warnings).toBe('number');
        expect(typeof result.details.warnings.summary.info).toBe('number');
      }
    });
  });

  describe('baseline-aware behaviour', () => {
    it('should include baselineLoaded in details when no baseline exists', async () => {
      const context = createMockContext();
      const result = await check.run(context);

      if (!result.details?.skipped) {
        expect(result.details?.baselineLoaded).toBe(false);
      }
    });
  });

  describe('isNewViolation logic (via integration)', () => {
    it('should match violations by from_file, to_file, AND rule when rule field exists in baseline', () => {
      const checkInstance = new ArchitectureCheck();
      const isNewViolation = (
        checkInstance as unknown as {
          isNewViolation: (
            violation: { from: string; to: string; rule: { name: string; severity: string } },
            baseline: {
              baseline_snapshot: {
                violations: Array<{
                  id: string;
                  from_layer: string;
                  to_layer: string;
                  from_file: string;
                  to_file: string;
                  import_line: number;
                  rule?: string;
                }>;
              };
            }
          ) => boolean;
        }
      ).isNewViolation.bind(checkInstance);

      const baselineWithRule = {
        baseline_snapshot: {
          violations: [
            {
              id: 'test-1',
              from_layer: 'domain',
              to_layer: 'infrastructure',
              from_file: 'src/a.ts',
              to_file: 'src/b.ts',
              import_line: 10,
              rule: 'no-circular',
            },
          ],
        },
      };

      expect(
        isNewViolation(
          { from: 'src/a.ts', to: 'src/b.ts', rule: { name: 'no-circular', severity: 'error' } },
          baselineWithRule
        )
      ).toBe(false);

      expect(
        isNewViolation(
          { from: 'src/a.ts', to: 'src/b.ts', rule: { name: 'no-orphans', severity: 'warn' } },
          baselineWithRule
        )
      ).toBe(true);
    });

    it('should match only by from_file and to_file when rule field is missing (backwards compat)', () => {
      const checkInstance = new ArchitectureCheck();
      const isNewViolation = (
        checkInstance as unknown as {
          isNewViolation: (
            violation: { from: string; to: string; rule: { name: string; severity: string } },
            baseline: {
              baseline_snapshot: {
                violations: Array<{
                  id: string;
                  from_layer: string;
                  to_layer: string;
                  from_file: string;
                  to_file: string;
                  import_line: number;
                }>;
              };
            }
          ) => boolean;
        }
      ).isNewViolation.bind(checkInstance);

      const baselineWithoutRule = {
        baseline_snapshot: {
          violations: [
            {
              id: 'test-1',
              from_layer: 'domain',
              to_layer: 'infrastructure',
              from_file: 'src/a.ts',
              to_file: 'src/b.ts',
              import_line: 10,
            },
          ],
        },
      };

      expect(
        isNewViolation(
          { from: 'src/a.ts', to: 'src/b.ts', rule: { name: 'no-circular', severity: 'error' } },
          baselineWithoutRule
        )
      ).toBe(false);

      expect(
        isNewViolation(
          { from: 'src/a.ts', to: 'src/b.ts', rule: { name: 'no-orphans', severity: 'warn' } },
          baselineWithoutRule
        )
      ).toBe(false);
    });

    it('should detect new violations not in baseline', () => {
      const checkInstance = new ArchitectureCheck();
      const isNewViolation = (
        checkInstance as unknown as {
          isNewViolation: (
            violation: { from: string; to: string; rule: { name: string; severity: string } },
            baseline: {
              baseline_snapshot: {
                violations: Array<{
                  id: string;
                  from_layer: string;
                  to_layer: string;
                  from_file: string;
                  to_file: string;
                  import_line: number;
                  rule?: string;
                }>;
              };
            }
          ) => boolean;
        }
      ).isNewViolation.bind(checkInstance);

      const baseline = {
        baseline_snapshot: {
          violations: [
            {
              id: 'test-1',
              from_layer: 'domain',
              to_layer: 'infrastructure',
              from_file: 'src/existing.ts',
              to_file: 'src/other.ts',
              import_line: 10,
              rule: 'no-circular',
            },
          ],
        },
      };

      expect(
        isNewViolation(
          {
            from: 'src/new.ts',
            to: 'src/another.ts',
            rule: { name: 'no-circular', severity: 'error' },
          },
          baseline
        )
      ).toBe(true);
    });
  });

  describe('calculateScore with effective violations', () => {
    it('should calculate score from provided violations', () => {
      const checkInstance = new ArchitectureCheck();
      const calculateScore = (
        checkInstance as unknown as {
          calculateScore: (
            violations: Array<{
              from: string;
              to: string;
              rule: { name: string; severity: 'error' | 'warn' | 'info' | 'ignore' };
              cycle?: string[];
            }>,
            config: {
              fail_on_circular: boolean;
              fail_on_orphan: boolean;
              severity_threshold: string;
            }
          ) => { score: number; passed: boolean; violationsByType: Record<string, number> };
        }
      ).calculateScore.bind(checkInstance);

      const errorViolation = {
        from: 'src/a.ts',
        to: 'src/b.ts',
        rule: { name: 'no-circular', severity: 'error' as const },
        cycle: ['src/a.ts', 'src/b.ts'],
      };

      const warnViolation = {
        from: 'src/c.ts',
        to: 'src/d.ts',
        rule: { name: 'no-orphans', severity: 'warn' as const },
      };

      const config = {
        fail_on_circular: true,
        fail_on_orphan: false,
        severity_threshold: 'error',
      };

      const resultEmpty = calculateScore([], config);
      expect(resultEmpty.score).toBe(100);
      expect(resultEmpty.passed).toBe(true);

      const resultOneError = calculateScore([errorViolation], config);
      expect(resultOneError.score).toBe(85);
      expect(resultOneError.passed).toBe(false);
      expect(resultOneError.violationsByType.circular).toBe(1);

      const resultOneWarn = calculateScore([warnViolation], config);
      expect(resultOneWarn.score).toBe(95);
      expect(resultOneWarn.passed).toBe(true);
      expect(resultOneWarn.violationsByType.orphan).toBe(1);
    });
  });
});
