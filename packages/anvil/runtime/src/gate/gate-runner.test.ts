import { describe, it, expect, beforeEach } from 'vitest';
import { GateRunner } from './gate-runner.js';
import { GateConfig, PlanData, CheckContext, GateResult } from '../types/gate.types.js';
import { BaseCheck } from './check.interface.js';
import type { Warning, WarningResult } from '@anvil/core/antipattern';

class MockCheck extends BaseCheck {
  name = 'mock';
  description = 'Mock check for testing';

  async run() {
    return this.createSuccess('Mock check passed', 100);
  }
}

class FailingMockCheck extends BaseCheck {
  name = 'failing-mock';
  description = 'Failing mock check for testing';

  async run() {
    return this.createFailure('Mock check failed');
  }
}

class MockArchitectureCheck extends BaseCheck {
  name = 'architecture';
  description = 'Mock architecture check for testing';

  async run(context: CheckContext): Promise<GateResult> {
    const warnings: Warning[] = [];
    const files = context.targetFiles ?? [];

    for (const file of files) {
      if (file.includes('boundary-violation')) {
        warnings.push({
          id: 'ARCH-001',
          category: 'boundary',
          severity: 'warning',
          confidence: 'high',
          title: 'Cross-boundary import detected',
          message: `File ${file} imports from forbidden module`,
          explanation: 'This violates architecture boundaries',
          suggestion: 'Use the public API instead',
          location: { file, line: 1 },
          pattern: 'boundary-violation',
        });
      }
      if (file.includes('error-boundary')) {
        warnings.push({
          id: 'ARCH-002',
          category: 'boundary',
          severity: 'error',
          confidence: 'high',
          title: 'Critical boundary violation',
          message: `File ${file} has critical violation`,
          explanation: 'This is a blocking error',
          suggestion: 'Fix immediately',
          location: { file, line: 1 },
          pattern: 'critical-boundary',
        });
      }
    }

    const warningResult: WarningResult = {
      warnings,
      summary: {
        total: warnings.length,
        errors: warnings.filter((w) => w.severity === 'error').length,
        warnings: warnings.filter((w) => w.severity === 'warning').length,
        info: 0,
        suppressed: 0,
      },
      patterns_checked: ['boundary-violation', 'critical-boundary'],
    };

    return {
      check: this.name,
      passed: warnings.filter((w) => w.severity === 'error').length === 0,
      message: warnings.length > 0 ? `Found ${warnings.length} warnings` : 'No warnings found',
      score: 100 - warnings.length * 10,
      details: { warnings: warningResult },
    };
  }
}

describe('GateRunner', () => {
  let gateRunner: GateRunner;
  let mockPlan: PlanData;
  let mockConfig: GateConfig;

  beforeEach(() => {
    gateRunner = new GateRunner();
    mockPlan = {
      id: 'aps-test123',
      intent: 'Test plan',
      proposed_changes: [
        {
          type: 'file',
          target: 'src/test.ts',
          action: 'create',
          content: 'console.log("test");',
        },
      ],
      provenance: {
        created_at: '2024-01-01T00:00:00Z',
        created_by: 'test@example.com',
        version: '1.0.0',
      },
    };
    mockConfig = {
      version: 1,
      checks: [
        {
          name: 'mock',
          description: 'Mock check',
          enabled: true,
          config: {},
        },
      ],
      thresholds: {
        overall_score: 80,
      },
    };
  });

  it('should register and run custom checks', async () => {
    const mockCheck = new MockCheck();
    gateRunner.registerCheck(mockCheck);

    const result = await gateRunner.runGate(mockPlan, mockConfig, '/workspace');

    expect(result.overall).toBe(true);
    expect(result.score).toBe(100);
    expect(result.checks).toHaveLength(1);
    expect(result.checks[0].check).toBe('mock');
    expect(result.checks[0].passed).toBe(true);
  });

  it('should handle failing checks', async () => {
    const failingCheck = new FailingMockCheck();
    gateRunner.registerCheck(failingCheck);

    mockConfig.checks = [
      {
        name: 'failing-mock',
        description: 'Failing mock check',
        enabled: true,
        config: {},
      },
    ];

    const result = await gateRunner.runGate(mockPlan, mockConfig, '/workspace');

    expect(result.overall).toBe(false);
    expect(result.checks[0].passed).toBe(false);
  });

  it('should skip disabled checks', async () => {
    mockConfig.checks = [
      {
        name: 'mock',
        description: 'Mock check',
        enabled: false,
        config: {},
      },
    ];

    const result = await gateRunner.runGate(mockPlan, mockConfig, '/workspace');

    expect(result.overall).toBe(true);
    expect(result.checks[0].skipped).toBe(true);
  });

  it('should handle unknown checks', async () => {
    mockConfig.checks = [
      {
        name: 'unknown-check',
        description: 'Unknown check',
        enabled: true,
        config: {},
      },
    ];

    const result = await gateRunner.runGate(mockPlan, mockConfig, '/workspace');

    expect(result.checks[0].passed).toBe(false);
    expect(result.checks[0].error).toBe('Unknown check');
  });

  it('should calculate overall score correctly', async () => {
    const check1 = new MockCheck();
    check1.name = 'check1';
    const check2 = new MockCheck();
    check2.name = 'check2';

    gateRunner.registerCheck(check1);
    gateRunner.registerCheck(check2);

    mockConfig.checks = [
      {
        name: 'check1',
        description: 'Check 1',
        enabled: true,
        config: {},
      },
      {
        name: 'check2',
        description: 'Check 2',
        enabled: true,
        config: {},
      },
    ];

    const result = await gateRunner.runGate(mockPlan, mockConfig, '/workspace');

    expect(result.score).toBe(100);
    expect(result.summary.total).toBe(2);
    expect(result.summary.passed).toBe(2);
  });

  it('should unregister checks', () => {
    const mockCheck = new MockCheck();
    gateRunner.registerCheck(mockCheck);

    expect(gateRunner.getAvailableChecks()).toContain('mock');

    gateRunner.unregisterCheck('mock');

    expect(gateRunner.getAvailableChecks()).not.toContain('mock');
  });
});

describe('GateRunner.analyzeFiles', () => {
  let gateRunner: GateRunner;

  beforeEach(() => {
    gateRunner = new GateRunner();
    gateRunner.unregisterCheck('architecture');
    gateRunner.registerCheck(new MockArchitectureCheck());
  });

  it('should analyze files and return warnings', async () => {
    const files = ['src/boundary-violation.ts', 'src/clean-file.ts'];

    const result = await gateRunner.analyzeFiles(files, '/workspace');

    expect(result.checksRun).toContain('architecture');
    expect(result.warnings.warnings).toHaveLength(1);
    expect(result.warnings.warnings[0].id).toBe('ARCH-001');
    expect(result.hasBlockingWarnings).toBe(false);
  });

  it('should detect blocking warnings (severity: error)', async () => {
    const files = ['src/error-boundary.ts'];

    const result = await gateRunner.analyzeFiles(files, '/workspace');

    expect(result.hasBlockingWarnings).toBe(true);
    expect(result.warnings.summary.errors).toBe(1);
  });

  it('should return empty warnings for clean files', async () => {
    const files = ['src/clean-file.ts', 'src/another-clean.ts'];

    const result = await gateRunner.analyzeFiles(files, '/workspace');

    expect(result.warnings.warnings).toHaveLength(0);
    expect(result.hasBlockingWarnings).toBe(false);
  });

  it('should skip unknown checks gracefully', async () => {
    const files = ['src/test.ts'];

    const result = await gateRunner.analyzeFiles(files, '/workspace', {
      checks: ['nonexistent' as 'architecture'],
    });

    expect(result.checksRun).toHaveLength(0);
    expect(result.warnings.warnings).toHaveLength(0);
  });

  it('should exclude failed checks from checksRun', async () => {
    class FailingArchitectureCheck extends BaseCheck {
      name = 'architecture';
      description = 'Mock failing architecture check';

      async run(): Promise<GateResult> {
        return this.createFailure('Simulated failure');
      }
    }

    gateRunner.unregisterCheck('architecture');
    gateRunner.registerCheck(new FailingArchitectureCheck());

    const result = await gateRunner.analyzeFiles(['src/test.ts'], '/workspace');

    expect(result.checksRun).not.toContain('architecture');
    expect(result.warnings.warnings).toHaveLength(0);
  });

  it('should include execution timing', async () => {
    const files = ['src/test.ts'];

    const result = await gateRunner.analyzeFiles(files, '/workspace');

    expect(result.executionTimeMs).toBeGreaterThanOrEqual(0);
  });

  it('should aggregate warnings from multiple files', async () => {
    const files = ['src/boundary-violation.ts', 'src/error-boundary.ts'];

    const result = await gateRunner.analyzeFiles(files, '/workspace');

    expect(result.warnings.warnings).toHaveLength(2);
    expect(result.warnings.summary.total).toBe(2);
    expect(result.warnings.summary.warnings).toBe(1);
    expect(result.warnings.summary.errors).toBe(1);
    expect(result.hasBlockingWarnings).toBe(true);
  });

  it('should deduplicate patterns_checked', async () => {
    const files = ['src/boundary-violation.ts', 'src/another-boundary-violation.ts'];

    const result = await gateRunner.analyzeFiles(files, '/workspace');

    const uniquePatterns = new Set(result.warnings.patterns_checked);
    expect(uniquePatterns.size).toBe(result.warnings.patterns_checked.length);
  });
});
