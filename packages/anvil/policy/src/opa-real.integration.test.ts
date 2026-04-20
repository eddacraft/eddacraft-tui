/**
 * Real-binary OPA integration tests.
 *
 * Skipped when `opa` is not on PATH and `ANVIL_OPA_PATH` is unset, so local
 * dev environments without OPA installed don't fail. CI installs OPA pinned
 * to DEFAULT_OPA_VERSION, so these run in CI.
 *
 * Covers:
 *   - TCOV-009: real `opa eval` against fixture rego policies
 *   - TCOV-011: real `opa test` against fixture `*_test.rego`
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { OPAExecutor, type OPAInput } from './opa-executor.js';
import { PolicyLoader, type LoadedPolicy } from './policy-loader.js';

function findOpaBinary(): string | null {
  const envPath = process.env.ANVIL_OPA_PATH;
  if (envPath) return envPath;
  const which = spawnSync('which', ['opa'], { encoding: 'utf-8' });
  if (which.status === 0) {
    return which.stdout.trim();
  }
  return null;
}

const opaPath = findOpaBinary();

const HERE = dirname(fileURLToPath(import.meta.url));
// HERE = packages/anvil/policy/src — repo root is four levels up.
const REPO_ROOT = resolve(HERE, '../../../..');
const FIXTURES_DIR = resolve(REPO_ROOT, 'policies/fixtures');

const baseInput = (overrides: Partial<OPAInput['plan']> = {}): OPAInput => ({
  plan: {
    id: 'plan-real-opa',
    hash: 'h',
    intent: 'integration test',
    schema_version: '0.1.0',
    proposed_changes: [],
    tags: [],
    ...overrides,
  },
  context: {
    workspace_root: '/tmp',
    timestamp: 0,
  },
});

describe.skipIf(!opaPath)('OPA real-binary integration (TCOV-009/-011)', () => {
  let executor: OPAExecutor;
  let policies: LoadedPolicy[];

  beforeAll(async () => {
    executor = new OPAExecutor(opaPath as string, { timeout: 10_000 });
    const loader = new PolicyLoader();
    const result = await loader.loadPolicies(REPO_ROOT, {
      policyDir: 'policies/fixtures',
    });
    policies = result.policies;
    expect(result.errors).toEqual([]);
    expect(policies.map((p) => p.name).sort()).toEqual([
      'change_scope',
      'coverage_min',
      'security_baseline',
    ]);
  });

  describe('change_scope.rego', () => {
    it('flags too-many-files violations', async () => {
      const proposed_changes = Array.from({ length: 25 }, (_, i) => ({
        type: 'file_create',
        path: `src/file_${i}.ts`,
        directory: 'src',
      }));
      const input = baseInput({ proposed_changes, change_count: 25 });

      const result = await executor.evaluate(policies, input);

      expect(result.success).toBe(true);
      const changeViolations = result.violations.filter((v) => v.policy === 'change_scope');
      expect(changeViolations.length).toBeGreaterThan(0);
      expect(changeViolations.some((v) => v.message.includes('25 files'))).toBe(true);
    });

    it('passes a small plan with no violations', async () => {
      const input = baseInput({
        proposed_changes: [{ type: 'file_create', path: 'src/a.ts', directory: 'src' }],
      });

      const result = await executor.evaluate(policies, input);

      const changeViolations = result.violations.filter((v) => v.policy === 'change_scope');
      expect(changeViolations).toEqual([]);
    });
  });

  describe('security_baseline.rego', () => {
    it('flags sensitive paths missing security-review tag', async () => {
      const input = baseInput({
        proposed_changes: [
          { type: 'file_modify', path: 'src/auth/login.ts', directory: 'src/auth' },
        ],
        tags: [],
      });

      const result = await executor.evaluate(policies, input);

      const securityViolations = result.violations.filter((v) => v.policy === 'security_baseline');
      expect(securityViolations.length).toBeGreaterThan(0);
      expect(securityViolations[0].message).toMatch(/security-review/);
    });

    it('passes when security-review tag is present', async () => {
      const input = baseInput({
        proposed_changes: [
          { type: 'file_modify', path: 'src/auth/login.ts', directory: 'src/auth' },
        ],
        tags: ['security-review'],
      });

      const result = await executor.evaluate(policies, input);

      const securityViolations = result.violations.filter(
        (v) => v.policy === 'security_baseline' && v.severity === 'error'
      );
      expect(securityViolations).toEqual([]);
    });
  });

  describe('coverage_min.rego', () => {
    it('flags below-threshold coverage', async () => {
      const input: OPAInput = {
        ...baseInput(),
        context: {
          workspace_root: '/tmp',
          timestamp: 0,
          coverage: { lines: 50 },
        },
      };

      const result = await executor.evaluate(policies, input);

      const covViolations = result.violations.filter((v) => v.policy === 'coverage_min');
      expect(covViolations.length).toBe(1);
      expect(covViolations[0].message).toMatch(/50.*below.*80/);
    });

    it('passes at threshold', async () => {
      const input: OPAInput = {
        ...baseInput(),
        context: {
          workspace_root: '/tmp',
          timestamp: 0,
          coverage: { lines: 95 },
        },
      };

      const result = await executor.evaluate(policies, input);

      const covViolations = result.violations.filter((v) => v.policy === 'coverage_min');
      expect(covViolations).toEqual([]);
    });
  });

  describe('opa test against fixture *_test.rego (TCOV-011)', () => {
    it('runs all policy unit tests and they pass', () => {
      const result = spawnSync(opaPath as string, ['test', FIXTURES_DIR], {
        encoding: 'utf-8',
        timeout: 30_000,
      });

      expect(result.status, result.stderr).toBe(0);
      expect(result.stdout).toMatch(/PASS:\s*\d+\/\d+/);
      expect(result.stdout).not.toMatch(/FAIL/);
    });
  });
});

if (!opaPath) {
  describe('OPA real-binary integration (skipped)', () => {
    it.skip('OPA binary not found; install opa or set ANVIL_OPA_PATH to enable', () => {
      // documented skip
    });
  });
}
