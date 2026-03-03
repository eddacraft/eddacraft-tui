import { describe, it, expect, afterEach, vi } from 'vitest';
import { DependencyCheck } from './dependency.check.js';
import type { CheckContext } from '../../types/gate.types.js';

type DependencyCheckInternals = {
  runAudit: (workspaceRoot: string, packageManager: 'npm' | 'yarn' | 'pnpm') => Promise<unknown>;
  detectPackageManager: (workspaceRoot: string) => 'npm' | 'yarn' | 'pnpm' | null;
};

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

  afterEach(() => {
    vi.restoreAllMocks();
  });

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
    const internal = check as unknown as DependencyCheckInternals;
    vi.spyOn(internal, 'runAudit').mockResolvedValue(null);
    vi.spyOn(internal, 'detectPackageManager').mockReturnValue('pnpm');

    await expect(check.run(mockContext)).resolves.toBeDefined();
  });

  it('should return a properly formatted result', async () => {
    const check = new DependencyCheck();
    const internal = check as unknown as DependencyCheckInternals;
    vi.spyOn(internal, 'runAudit').mockResolvedValue(null);
    vi.spyOn(internal, 'detectPackageManager').mockReturnValue('pnpm');
    const result = await check.run(mockContext);

    expect(result).toHaveProperty('check');
    expect(result).toHaveProperty('passed');
    expect(result).toHaveProperty('message');
    expect(result.check).toBe('dependency');
  });

  it('should surface audit parse failure as a check failure', async () => {
    const check = new DependencyCheck();
    const internal = check as unknown as DependencyCheckInternals;
    vi.spyOn(internal, 'runAudit').mockRejectedValue(
      new Error('Failed to parse pnpm audit output: Unexpected token')
    );
    vi.spyOn(internal, 'detectPackageManager').mockReturnValue('pnpm');

    const result = await check.run(mockContext);

    expect(result.passed).toBe(false);
    expect(result.error).toContain('Failed to parse pnpm audit output');
    expect(result.message).toBe('Dependency check failed');
  });

  it('should surface audit command errors as a check failure', async () => {
    const check = new DependencyCheck();
    const internal = check as unknown as DependencyCheckInternals;
    vi.spyOn(internal, 'runAudit').mockRejectedValue(new Error('ENOENT: pnpm not found'));
    vi.spyOn(internal, 'detectPackageManager').mockReturnValue('pnpm');

    const result = await check.run(mockContext);

    expect(result.passed).toBe(false);
    expect(result.error).toContain('pnpm not found');
  });
});
