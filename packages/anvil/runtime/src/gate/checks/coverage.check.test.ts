/**
 * Unit Tests for Coverage Check
 *
 * Tests coverage threshold validation
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { CoverageCheck } from './coverage.check.js';
import { CheckContext, PlanData } from '../../types/gate.types.js';
import { writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { safeCleanup } from '../../../../../../tools/test-utils/safe-cleanup.js';

describe('CoverageCheck', () => {
  let coverageCheck: CoverageCheck;
  let tempDir: string;
  let context: CheckContext;

  beforeEach(() => {
    coverageCheck = new CoverageCheck();
    tempDir = join(tmpdir(), 'anvil-coverage-test', Math.random().toString(36));
    mkdirSync(join(tempDir, 'coverage'), { recursive: true });

    const mockPlan: PlanData = {
      id: 'aps-test123',
      schema_version: '0.1.0',
      hash: 'test-hash',
      intent: 'Test plan',
      proposed_changes: [],
      provenance: {
        timestamp: '2024-01-01T00:00:00Z',
        author: 'test@example.com',
        source: 'cli',
        version: '1.0.0',
      },
      validations: {
        required_checks: [],
        skip_checks: [],
      },
      evidence: [],
      executions: [],
    };

    context = {
      plan: mockPlan,
      workspace_root: tempDir,
      config: {
        version: 1,
        checks: [],
        thresholds: { overall_score: 80 },
      },
      check_config: {
        min_score: 80,
        thresholds: {
          lines: 80,
          functions: 80,
          branches: 80,
          statements: 80,
        },
      },
    };
  });

  afterEach(async () => {
    await safeCleanup(tempDir);
  });

  function createCoverageSummary(coverage: {
    lines?: number;
    functions?: number;
    branches?: number;
    statements?: number;
  }) {
    const makeCoverage = (pct: number) => ({
      total: 100,
      covered: pct,
      skipped: 0,
      pct,
    });

    return {
      total: {
        lines: makeCoverage(coverage.lines ?? 80),
        functions: makeCoverage(coverage.functions ?? 80),
        branches: makeCoverage(coverage.branches ?? 80),
        statements: makeCoverage(coverage.statements ?? 80),
      },
    };
  }

  describe('missing coverage report', () => {
    it('should fail when coverage report does not exist', async () => {
      const result = await coverageCheck.run(context);

      expect(result.passed).toBe(false);
      expect(result.message).toContain('Coverage report not found');
    });

    it('should provide helpful error message', async () => {
      const result = await coverageCheck.run(context);

      expect(result.message).toContain('Coverage report not found');
    });
  });

  describe('coverage passing thresholds', () => {
    it('should pass when all metrics meet thresholds', async () => {
      const summary = createCoverageSummary({
        lines: 85,
        functions: 85,
        branches: 85,
        statements: 85,
      });

      writeFileSync(
        join(tempDir, 'coverage', 'coverage-summary.json'),
        JSON.stringify(summary),
        'utf-8'
      );

      const result = await coverageCheck.run(context);

      expect(result.passed).toBe(true);
      expect(result.message).toContain('Coverage passed');
    });

    it('should pass when coverage exactly meets thresholds', async () => {
      const summary = createCoverageSummary({
        lines: 80,
        functions: 80,
        branches: 80,
        statements: 80,
      });

      writeFileSync(
        join(tempDir, 'coverage', 'coverage-summary.json'),
        JSON.stringify(summary),
        'utf-8'
      );

      const result = await coverageCheck.run(context);

      expect(result.passed).toBe(true);
      expect(result.score).toBe(80);
    });

    it('should pass with high coverage', async () => {
      const summary = createCoverageSummary({
        lines: 95,
        functions: 98,
        branches: 92,
        statements: 96,
      });

      writeFileSync(
        join(tempDir, 'coverage', 'coverage-summary.json'),
        JSON.stringify(summary),
        'utf-8'
      );

      const result = await coverageCheck.run(context);

      expect(result.passed).toBe(true);
      expect(result.score).toBeGreaterThan(90);
    });

    it('should pass with 100% coverage', async () => {
      const summary = createCoverageSummary({
        lines: 100,
        functions: 100,
        branches: 100,
        statements: 100,
      });

      writeFileSync(
        join(tempDir, 'coverage', 'coverage-summary.json'),
        JSON.stringify(summary),
        'utf-8'
      );

      const result = await coverageCheck.run(context);

      expect(result.passed).toBe(true);
      expect(result.score).toBe(100);
    });
  });

  describe('coverage failing thresholds', () => {
    it('should fail when overall score is below min_score', async () => {
      const summary = createCoverageSummary({
        lines: 70,
        functions: 75,
        branches: 70,
        statements: 72,
      });

      writeFileSync(
        join(tempDir, 'coverage', 'coverage-summary.json'),
        JSON.stringify(summary),
        'utf-8'
      );

      const result = await coverageCheck.run(context);

      // Overall: (70+75+70+72)/4 = 71.75 < 80
      expect(result.passed).toBe(false);
      expect(result.message).toContain('Coverage failed');
    });

    it('should fail when some metrics are very low', async () => {
      const summary = createCoverageSummary({
        lines: 60,
        functions: 65,
        branches: 70,
        statements: 75,
      });

      writeFileSync(
        join(tempDir, 'coverage', 'coverage-summary.json'),
        JSON.stringify(summary),
        'utf-8'
      );

      const result = await coverageCheck.run(context);

      // Overall: (60+65+70+75)/4 = 67.5 < 80
      expect(result.passed).toBe(false);
    });

    it('should fail when coverage is just below threshold', async () => {
      const summary = createCoverageSummary({
        lines: 79,
        functions: 79,
        branches: 79,
        statements: 79,
      });

      writeFileSync(
        join(tempDir, 'coverage', 'coverage-summary.json'),
        JSON.stringify(summary),
        'utf-8'
      );

      const result = await coverageCheck.run(context);

      // Overall: 79 < 80
      expect(result.passed).toBe(false);
    });

    it('should fail when multiple metrics are below threshold', async () => {
      const summary = createCoverageSummary({
        lines: 70,
        functions: 75,
        branches: 65,
        statements: 72,
      });

      writeFileSync(
        join(tempDir, 'coverage', 'coverage-summary.json'),
        JSON.stringify(summary),
        'utf-8'
      );

      const result = await coverageCheck.run(context);

      expect(result.passed).toBe(false);
    });
  });

  describe('custom thresholds', () => {
    it('should handle custom thresholds from config', async () => {
      context.check_config.thresholds = {
        lines: 90,
        functions: 85,
        branches: 75,
        statements: 80,
      };

      const summary = createCoverageSummary({
        lines: 88,
        functions: 86,
        branches: 76,
        statements: 81,
      });

      writeFileSync(
        join(tempDir, 'coverage', 'coverage-summary.json'),
        JSON.stringify(summary),
        'utf-8'
      );

      const result = await coverageCheck.run(context);

      // Overall: (88+86+76+81)/4 = 82.75 > 80 min_score
      // Note: Custom thresholds are stored but overall score is what matters
      expect(result.passed).toBe(true);
    });

    it('should use custom min_score from config', async () => {
      context.check_config.min_score = 90;

      const summary = createCoverageSummary({
        lines: 85,
        functions: 85,
        branches: 85,
        statements: 85,
      });

      writeFileSync(
        join(tempDir, 'coverage', 'coverage-summary.json'),
        JSON.stringify(summary),
        'utf-8'
      );

      const result = await coverageCheck.run(context);

      // Should fail because overall score (85) < min_score (90)
      expect(result.passed).toBe(false);
    });

    it('should handle missing threshold config', async () => {
      context.check_config.thresholds = undefined;

      const summary = createCoverageSummary({
        lines: 85,
        functions: 85,
        branches: 85,
        statements: 85,
      });

      writeFileSync(
        join(tempDir, 'coverage', 'coverage-summary.json'),
        JSON.stringify(summary),
        'utf-8'
      );

      const result = await coverageCheck.run(context);

      // Should use default thresholds (80)
      expect(result.passed).toBe(true);
    });
  });

  describe('score calculation', () => {
    it('should calculate overall score as average of metrics', async () => {
      const summary = createCoverageSummary({
        lines: 80,
        functions: 90,
        branches: 70,
        statements: 100,
      });

      writeFileSync(
        join(tempDir, 'coverage', 'coverage-summary.json'),
        JSON.stringify(summary),
        'utf-8'
      );

      const result = await coverageCheck.run(context);

      // Average: (80 + 90 + 70 + 100) / 4 = 85
      expect(result.score).toBe(85);
    });

    it('should include details for each metric', async () => {
      const summary = createCoverageSummary({
        lines: 85,
        functions: 88,
        branches: 82,
        statements: 90,
      });

      writeFileSync(
        join(tempDir, 'coverage', 'coverage-summary.json'),
        JSON.stringify(summary),
        'utf-8'
      );

      const result = await coverageCheck.run(context);

      expect(result.details).toBeDefined();
      const details = result.details as Record<string, unknown>;
      const innerDetails = details.details as Record<string, unknown>;
      expect(innerDetails).toBeDefined();
      expect(innerDetails.lines).toBeDefined();
      expect(innerDetails.functions).toBeDefined();
      expect(innerDetails.branches).toBeDefined();
      expect(innerDetails.statements).toBeDefined();
    });
  });

  describe('error handling', () => {
    it('should handle malformed coverage JSON', async () => {
      writeFileSync(join(tempDir, 'coverage', 'coverage-summary.json'), 'invalid json', 'utf-8');

      const result = await coverageCheck.run(context);

      expect(result.passed).toBe(false);
      expect(result.message).toContain('Coverage check failed');
    });

    it('should handle missing coverage metrics', async () => {
      writeFileSync(join(tempDir, 'coverage', 'coverage-summary.json'), '{}', 'utf-8');

      const result = await coverageCheck.run(context);

      expect(result.passed).toBe(false);
    });
  });

  describe('check metadata', () => {
    it('should have correct name', () => {
      expect(coverageCheck.name).toBe('coverage');
    });

    it('should have correct description', () => {
      expect(coverageCheck.description).toBe('Check test coverage thresholds');
    });
  });
});
