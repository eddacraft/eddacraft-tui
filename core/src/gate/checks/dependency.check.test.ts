import { describe, it, expect } from 'vitest';
import { DependencyCheck } from './dependency.check.js';
import type { CheckContext } from '../../types/gate.types.js';

describe('DependencyCheck', () => {
  const mockContext: CheckContext = {
    plan: {
      id: 'test-plan',
      schema_version: '0.1.0',
      hash: 'test-hash',
      intent: 'Test plan',
      provenance: {
        timestamp: new Date().toISOString(),
        author: 'test',
      },
      proposed_changes: [
        {
          type: 'file_update',
          path: 'package.json',
          description: 'Update dependencies',
        },
      ],
      validations: {
        required_checks: ['dependency'],
      },
    },
    workspace_root: process.cwd(),
    config: {
      version: 1,
      checks: [
        {
          name: 'dependency',
          description: 'Dependency vulnerability scanning',
          enabled: true,
          config: {
            min_severity: 'moderate',
            fail_on_critical: true,
            fail_on_high: true,
            fail_on_moderate: false,
          },
        },
      ],
      thresholds: {
        overall_score: 80,
      },
    },
    check_config: {
      min_severity: 'moderate',
      fail_on_critical: true,
      fail_on_high: true,
      fail_on_moderate: false,
    },
  };

  it('should have correct metadata', () => {
    const check = new DependencyCheck();
    expect(check.name).toBe('dependency');
    expect(check.description).toContain('vulnerabilities');
  });

  it('should handle missing package.json gracefully', async () => {
    const check = new DependencyCheck();
    const context = {
      ...mockContext,
      workspace_root: '/nonexistent/path',
    };

    const result = await check.run(context);

    expect(result.passed).toBe(true);
    expect(result.message).toContain('No package.json');
  });

  it('should execute without throwing', async () => {
    const check = new DependencyCheck();

    // This will actually run pnpm audit on the current workspace
    // It should not throw even if vulnerabilities are found
    await expect(check.run(mockContext)).resolves.toBeDefined();
  });

  it('should return a properly formatted result', async () => {
    const check = new DependencyCheck();
    const result = await check.run(mockContext);

    expect(result).toHaveProperty('check');
    expect(result).toHaveProperty('passed');
    expect(result).toHaveProperty('message');
    expect(result.check).toBe('dependency');
  });
});
