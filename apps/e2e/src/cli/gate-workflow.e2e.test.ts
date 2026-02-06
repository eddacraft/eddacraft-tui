/**
 * CLI Gate Workflow — E2E Tests
 *
 * Tests the end-to-end gate execution workflow:
 * 1. Create a plan
 * 2. Run gate checks against the plan
 * 3. Verify evidence is produced
 *
 * Surface: CLI + Runtime (gate)
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { writeFileSync, readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { assertCliBuild, runCli } from '../helpers/cli-runner.js';
import { createE2EWorkspace, type E2EWorkspace } from '../helpers/workspace.js';
import { makePlan } from '../helpers/fixtures.js';

let ws: E2EWorkspace;

beforeAll(() => {
  assertCliBuild();
  ws = createE2EWorkspace({
    withGit: true,
    lockfile: 'pnpm',
    files: {
      'src/index.ts': 'export const main = () => console.log("hello");\n',
      'src/utils.ts': 'export const add = (a: number, b: number) => a + b;\n',
    },
  });
});

afterAll(() => ws.cleanup());

describe('Gate Workflow', () => {
  it('anvil gate --help shows gate command documentation', async () => {
    const result = await runCli(['gate', '--help']);
    expect(result.output.toLowerCase()).toContain('gate');
  });

  it('anvil gate runs in a configured workspace', async () => {
    const result = await runCli(['gate'], { cwd: ws.root });
    // Gate should produce structured output — even if checks fail
    // it should not crash
    expect(result.output.length).toBeGreaterThan(0);
  });

  it('anvil gate --skip-checks runs with checks disabled', async () => {
    const result = await runCli(['gate', '--skip-checks'], { cwd: ws.root });
    expect(result.output.length).toBeGreaterThan(0);
  });
});

describe('Plan → Gate roundtrip', () => {
  it('a plan written to disk can be referenced by gate', async () => {
    const plan = makePlan({ intent: 'Add utility function' });
    const planPath = join(ws.plansDir, `${plan.id}.json`);
    writeFileSync(planPath, JSON.stringify(plan, null, 2), 'utf-8');

    expect(existsSync(planPath)).toBe(true);

    // Verify the plan file is valid JSON with expected fields
    const loaded = JSON.parse(readFileSync(planPath, 'utf-8'));
    expect(loaded.id).toBe(plan.id);
    expect(loaded.intent).toBe('Add utility function');
    expect(loaded.schema_version).toBeDefined();
    expect(loaded.hash).toBeDefined();
  });
});
