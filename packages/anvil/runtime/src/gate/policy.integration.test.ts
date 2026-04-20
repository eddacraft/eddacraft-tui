/**
 * Gate pipeline + real-OPA integration test (TCOV-012).
 *
 * Exercises gate-runner -> policy.check -> OPAExecutor against the real
 * `opa` binary using fixtures from `policies/fixtures/`. Skipped when no
 * binary is available so local dev without OPA still passes.
 */

import { describe, it, expect, beforeAll, afterAll, vi } from 'vitest';
import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

import { GateRunner } from './gate-runner.js';
import type { GateConfig, PlanData } from '../types/gate.types.js';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

function findOpaBinary(): string | null {
  const envPath = process.env.ANVIL_OPA_PATH;
  if (envPath) return envPath;
  // Cross-platform PATH lookup: `where` on Windows, `which` elsewhere.
  // `OpaBinaryManager` validates that ANVIL_OPA_PATH points at a real
  // file, so we resolve to an absolute path here rather than just `'opa'`.
  const lookup = process.platform === 'win32' ? 'where' : 'which';
  const result = spawnSync(lookup, ['opa'], { encoding: 'utf-8' });
  if (result.status === 0) {
    return result.stdout.trim().split(/\r?\n/)[0] ?? null;
  }
  return null;
}

const opaPath = findOpaBinary();

const HERE = dirname(fileURLToPath(import.meta.url));
// HERE = packages/anvil/runtime/src/gate — repo root is five levels up.
const REPO_ROOT = resolve(HERE, '../../../../..');
const FIXTURES_DIR = resolve(REPO_ROOT, 'policies/fixtures');

function copyFixturesInto(targetDir: string): string[] {
  mkdirSync(targetDir, { recursive: true });
  const copied: string[] = [];
  for (const entry of readdirSync(FIXTURES_DIR)) {
    if (entry.endsWith('.rego') && !entry.endsWith('_test.rego')) {
      copyFileSync(join(FIXTURES_DIR, entry), join(targetDir, entry));
      copied.push(entry);
    }
  }
  return copied;
}

function basePlan(overrides: Partial<PlanData> = {}): PlanData {
  return {
    id: 'aps-policy-int',
    schema_version: '0.1.0',
    hash: 'h',
    intent: 'gate+OPA pipeline integration',
    proposed_changes: [],
    provenance: {
      timestamp: '2026-04-21T00:00:00Z',
      author: 'integration-test@anvil',
      source: 'cli',
      version: '1.0.0',
    },
    validations: { required_checks: [], skip_checks: [] },
    evidence: [],
    executions: [],
    tags: [],
    ...overrides,
  } as PlanData;
}

function policyOnlyConfig(): GateConfig {
  return {
    version: 1,
    checks: [
      {
        name: 'policy',
        description: 'OPA policy check',
        enabled: true,
        config: {
          policy_dir: '.anvil/policies',
          severity_threshold: 'error',
          include_git_context: false,
        },
      },
    ],
    thresholds: { overall_score: 80 },
  };
}

describe.skipIf(!opaPath)('Gate pipeline + real OPA (TCOV-012)', () => {
  let workspace: string;
  let copiedFixtures: string[];

  beforeAll(() => {
    vi.stubEnv('ANVIL_OPA_PATH', opaPath as string);
    workspace = mkdtempSync(join(tmpdir(), 'anvil-gate-opa-'));
    copiedFixtures = copyFixturesInto(join(workspace, '.anvil', 'policies'));
  });

  afterAll(async () => {
    vi.unstubAllEnvs();
    await safeCleanup(workspace);
  });

  it('does not copy *_test.rego fixtures as policies', () => {
    expect(copiedFixtures.every((name) => !name.endsWith('_test.rego'))).toBe(true);
    expect(copiedFixtures.length).toBeGreaterThan(0);
  });

  it('fails the gate when policies are violated by a large plan', async () => {
    const proposed_changes: PlanData['proposed_changes'] = Array.from({ length: 25 }, (_, i) => ({
      type: 'file_create' as const,
      path: `src/file_${i}.ts`,
      description: `change ${i}`,
    }));
    const plan = basePlan({ proposed_changes });
    const config = policyOnlyConfig();

    const runner = new GateRunner();
    const result = await runner.runGate(plan, config, workspace);

    expect(result.checks).toHaveLength(1);
    const policyResult = result.checks[0];
    expect(policyResult.check).toBe('policy');
    expect(policyResult.passed).toBe(false);
    expect(result.overall).toBe(false);

    const violations = (policyResult.details?.violations ?? []) as Array<{
      policy?: string;
      message: string;
    }>;
    expect(violations.length).toBeGreaterThan(0);
    expect(violations.some((v) => v.policy === 'change_scope')).toBe(true);
    expect(violations.some((v) => v.message.includes('25 files'))).toBe(true);
  });

  it('passes the gate when the plan is small and tagged for review', async () => {
    const plan = basePlan({
      proposed_changes: [
        {
          type: 'file_create' as const,
          path: 'src/index.ts',
          description: 'small change',
        },
      ],
      tags: ['security-review'],
    });
    const config = policyOnlyConfig();

    const runner = new GateRunner();
    const result = await runner.runGate(plan, config, workspace);

    expect(result.checks).toHaveLength(1);
    const policyResult = result.checks[0];
    expect(policyResult.check).toBe('policy');
    expect(policyResult.passed).toBe(true);
    expect(result.overall).toBe(true);

    const errorViolations = (
      (policyResult.details?.violations ?? []) as Array<{ severity: string }>
    ).filter((v) => v.severity === 'error');
    expect(errorViolations).toEqual([]);
  });
});

if (!opaPath) {
  describe('Gate pipeline + real OPA (skipped)', () => {
    it.skip('OPA binary not found; install opa or set ANVIL_OPA_PATH to enable', () => {
      // documented skip
    });
  });
}
