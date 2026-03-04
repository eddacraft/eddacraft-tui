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

  describe('error surfacing (CRB-005)', () => {
    it('should surface parse failures instead of reporting clean audit', async () => {
      const check = new DependencyCheck();
      const internal = check as unknown as DependencyCheckInternals;
      vi.spyOn(internal, 'detectPackageManager').mockReturnValue('npm');
      vi.spyOn(internal, 'runAudit').mockRejectedValue(
        new Error('Failed to parse npm audit output: Unexpected token < in JSON at position 0')
      );

      const result = await check.run(mockContext);

      expect(result.passed).toBe(false);
      expect(result.error).toBeDefined();
      expect(result.error).toContain('Failed to parse');
      expect(result.message).toBe('Dependency audit error');
    });

    it('should surface network errors from audit command', async () => {
      const check = new DependencyCheck();
      const internal = check as unknown as DependencyCheckInternals;
      vi.spyOn(internal, 'detectPackageManager').mockReturnValue('pnpm');
      vi.spyOn(internal, 'runAudit').mockRejectedValue(
        new Error('Command failed: pnpm audit --json\nEAI_AGAIN registry.npmjs.org')
      );

      const result = await check.run(mockContext);

      expect(result.passed).toBe(false);
      expect(result.error).toBeDefined();
      expect(result.error).toContain('EAI_AGAIN');
    });

    it('should surface timeout errors from audit command', async () => {
      const check = new DependencyCheck();
      const internal = check as unknown as DependencyCheckInternals;
      vi.spyOn(internal, 'detectPackageManager').mockReturnValue('npm');
      vi.spyOn(internal, 'runAudit').mockRejectedValue(
        new Error('Command timed out after 120000ms')
      );

      const result = await check.run(mockContext);

      expect(result.passed).toBe(false);
      expect(result.error).toBeDefined();
      expect(result.error).toContain('timed out');
    });

    it('should still handle EAUDITNOLOCK as skip', async () => {
      const check = new DependencyCheck();
      const internal = check as unknown as DependencyCheckInternals;
      vi.spyOn(internal, 'detectPackageManager').mockReturnValue('npm');
      vi.spyOn(internal, 'runAudit').mockRejectedValue(
        new Error('EAUDITNOLOCK: No lock file found')
      );

      const result = await check.run(mockContext);

      expect(result.passed).toBe(true);
      expect(result.message).toContain('No lock file');
    });

    it('should return clean audit when runAudit returns null legitimately', async () => {
      const check = new DependencyCheck();
      const internal = check as unknown as DependencyCheckInternals;
      vi.spyOn(internal, 'detectPackageManager').mockReturnValue('npm');
      vi.spyOn(internal, 'runAudit').mockResolvedValue(null);

      const result = await check.run(mockContext);

      expect(result.passed).toBe(true);
      expect(result.error).toBeUndefined();
      expect(result.message).toContain('No vulnerabilities');
    });

    it('should distinguish tool errors from vulnerability failures via error field', async () => {
      const check = new DependencyCheck();
      const internal = check as unknown as DependencyCheckInternals;
      vi.spyOn(internal, 'detectPackageManager').mockReturnValue('npm');

      // Vulnerability failure: passed=false, no error field
      vi.spyOn(internal, 'runAudit').mockResolvedValue({
        advisories: {
          '1': {
            id: 1,
            title: 'Test vulnerability',
            severity: 'critical',
            url: 'https://example.com',
            cves: ['CVE-2024-0001'],
            module_name: 'vulnerable-pkg',
            vulnerable_versions: '<1.0.0',
            patched_versions: '>=1.0.0',
            recommendation: 'Update to 1.0.0',
            findings: [{ version: '0.9.0', paths: ['vulnerable-pkg'] }],
          },
        },
        metadata: {
          vulnerabilities: { info: 0, low: 0, moderate: 0, high: 0, critical: 1, total: 1 },
        },
      });

      const vulnResult = await check.run(mockContext);
      expect(vulnResult.passed).toBe(false);
      expect(vulnResult.error).toBeUndefined();

      // Tool error: passed=false, error field populated
      vi.spyOn(internal, 'runAudit').mockRejectedValue(new Error('Registry unavailable'));

      const errorResult = await check.run(mockContext);
      expect(errorResult.passed).toBe(false);
      expect(errorResult.error).toBeDefined();
      expect(errorResult.error).toContain('Registry unavailable');
    });
  });
});
