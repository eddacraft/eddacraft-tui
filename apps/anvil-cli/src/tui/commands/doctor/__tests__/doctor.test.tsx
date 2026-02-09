import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { spawnSync } from 'node:child_process';
import { render } from 'ink-testing-library';
import React from 'react';
import { Diagnostics } from '../Diagnostics.js';
import {
  NodeVersionCheck,
  GitCheck,
  GitRepoCheck,
  ConfigExistsCheck,
  ConfigValidCheck,
  AnvilDirCheck,
  HuskyInstalledCheck,
  PreCommitHookCheck,
  AnvilDirWritableCheck,
  PlansDirReadableCheck,
} from '../checks/index.js';
import { calculateSummary } from '../types.js';
import type { DiagnosticContext, DiagnosticResult } from '../types.js';

const gitAvailable = (() => {
  const result = spawnSync('git', ['--version'], { stdio: 'pipe' });
  return !result.error && result.status === 0;
})();

describe('doctor types', () => {
  describe('calculateSummary', () => {
    it('should calculate correct summary for all pass', () => {
      const results: DiagnosticResult[] = [
        { checkId: 'a', name: 'A', status: 'pass', message: 'ok', fixable: false },
        { checkId: 'b', name: 'B', status: 'pass', message: 'ok', fixable: false },
      ];
      const summary = calculateSummary(results);
      expect(summary.total).toBe(2);
      expect(summary.passed).toBe(2);
      expect(summary.failed).toBe(0);
      expect(summary.healthy).toBe(true);
    });

    it('should calculate correct summary with failures', () => {
      const results: DiagnosticResult[] = [
        { checkId: 'a', name: 'A', status: 'pass', message: 'ok', fixable: false },
        { checkId: 'b', name: 'B', status: 'fail', message: 'bad', fixable: true },
        { checkId: 'c', name: 'C', status: 'warn', message: 'meh', fixable: false },
      ];
      const summary = calculateSummary(results);
      expect(summary.total).toBe(3);
      expect(summary.passed).toBe(1);
      expect(summary.failed).toBe(1);
      expect(summary.warnings).toBe(1);
      expect(summary.fixable).toBe(1);
      expect(summary.healthy).toBe(false);
    });

    it('should count skipped checks', () => {
      const results: DiagnosticResult[] = [
        { checkId: 'a', name: 'A', status: 'skip', message: 'skipped', fixable: false },
      ];
      const summary = calculateSummary(results);
      expect(summary.skipped).toBe(1);
      expect(summary.healthy).toBe(true);
    });
  });
});

describe('SystemChecks', () => {
  const context: DiagnosticContext = {
    projectRoot: process.cwd(),
    verbose: false,
  };

  describe('NodeVersionCheck', () => {
    it('should pass for current Node.js version', async () => {
      const check = new NodeVersionCheck();
      const result = await check.run(context);
      expect(result.status).toBe('pass');
      expect(result.message).toContain('Node.js');
    });
  });

  describe('GitCheck', () => {
    it('should pass when git is available', async () => {
      const check = new GitCheck();
      const result = await check.run(context);
      if (gitAvailable) {
        expect(result.status).toBe('pass');
        expect(result.message).toContain('git');
      } else {
        // git missing (ENOENT) → fail; only EPERM → skip
        expect(result.status).toBe('fail');
      }
    });
  });

  describe('GitRepoCheck', () => {
    it('should pass for anvil repository', async () => {
      const check = new GitRepoCheck();
      const result = await check.run(context);
      if (gitAvailable) {
        expect(result.status).toBe('pass');
      } else {
        // git missing (ENOENT) → warn; only EPERM → skip
        expect(result.status).toBe('warn');
      }
    });
  });
});

describe('ConfigChecks', () => {
  const tempDir = path.join(process.cwd(), 'tmp-doctor-test');

  beforeEach(() => {
    fs.mkdirSync(tempDir, { recursive: true });
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  describe('ConfigExistsCheck', () => {
    it('should warn when .anvilrc is missing', async () => {
      const check = new ConfigExistsCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('warn');
      expect(result.fixable).toBe(true);
    });

    it('should pass when .anvilrc exists', async () => {
      fs.writeFileSync(path.join(tempDir, '.anvilrc'), '{}');
      const check = new ConfigExistsCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('pass');
    });

    it('should fix by creating default .anvilrc', async () => {
      const check = new ConfigExistsCheck();
      const fixResult = await check.fix!({ projectRoot: tempDir, verbose: false });
      expect(fixResult.success).toBe(true);
      expect(fs.existsSync(path.join(tempDir, '.anvilrc'))).toBe(true);
    });
  });

  describe('ConfigValidCheck', () => {
    it('should skip when no config exists', async () => {
      const check = new ConfigValidCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('skip');
    });

    it('should pass for valid JSON config', async () => {
      fs.writeFileSync(path.join(tempDir, '.anvilrc'), '{"checks":{}}');
      const check = new ConfigValidCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('pass');
    });

    it('should fail for invalid JSON config', async () => {
      fs.writeFileSync(path.join(tempDir, '.anvilrc'), '{invalid json}');
      const check = new ConfigValidCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('fail');
      expect(result.fixable).toBe(true);
    });
  });

  describe('AnvilDirCheck', () => {
    it('should warn when .anvil/ is missing', async () => {
      const check = new AnvilDirCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('warn');
      expect(result.fixable).toBe(true);
    });

    it('should pass when .anvil/ exists', async () => {
      fs.mkdirSync(path.join(tempDir, '.anvil'));
      const check = new AnvilDirCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('pass');
    });

    it('should fix by creating .anvil/ directory', async () => {
      const check = new AnvilDirCheck();
      const fixResult = await check.fix!({ projectRoot: tempDir, verbose: false });
      expect(fixResult.success).toBe(true);
      expect(fs.existsSync(path.join(tempDir, '.anvil'))).toBe(true);
    });
  });
});

describe('HooksChecks', () => {
  const tempDir = path.join(process.cwd(), 'tmp-hooks-test');

  beforeEach(() => {
    fs.mkdirSync(tempDir, { recursive: true });
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  describe('HuskyInstalledCheck', () => {
    it('should skip when husky is not configured', async () => {
      const check = new HuskyInstalledCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('skip');
    });

    it('should pass when .husky directory exists', async () => {
      fs.mkdirSync(path.join(tempDir, '.husky'));
      const check = new HuskyInstalledCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('pass');
    });
  });

  describe('PreCommitHookCheck', () => {
    it('should skip when no .husky directory', async () => {
      const check = new PreCommitHookCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('skip');
    });

    it('should warn when pre-commit hook is missing', async () => {
      fs.mkdirSync(path.join(tempDir, '.husky'));
      const check = new PreCommitHookCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('warn');
      expect(result.fixable).toBe(true);
    });

    it('should pass when pre-commit hook exists and is executable', async () => {
      const huskyDir = path.join(tempDir, '.husky');
      fs.mkdirSync(huskyDir);
      const hookPath = path.join(huskyDir, 'pre-commit');
      fs.writeFileSync(hookPath, '#!/bin/sh\necho test', { mode: 0o755 });

      const check = new PreCommitHookCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('pass');
    });
  });
});

describe('PermissionsChecks', () => {
  const tempDir = path.join(process.cwd(), 'tmp-perms-test');

  beforeEach(() => {
    fs.mkdirSync(tempDir, { recursive: true });
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  describe('AnvilDirWritableCheck', () => {
    it('should skip when .anvil/ does not exist', async () => {
      const check = new AnvilDirWritableCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('skip');
    });

    it('should pass when .anvil/ is writable', async () => {
      fs.mkdirSync(path.join(tempDir, '.anvil'));
      const check = new AnvilDirWritableCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('pass');
    });
  });

  describe('PlansDirReadableCheck', () => {
    it('should skip when no plans directory exists', async () => {
      const check = new PlansDirReadableCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('skip');
    });

    it('should pass when plans/ is readable', async () => {
      fs.mkdirSync(path.join(tempDir, 'plans'));
      const check = new PlansDirReadableCheck();
      const result = await check.run({ projectRoot: tempDir, verbose: false });
      expect(result.status).toBe('pass');
    });
  });
});

describe('Diagnostics TUI', () => {
  const mockCheck = {
    id: 'mock-check',
    name: 'Mock Check',
    description: 'A mock check for testing',
    run: vi.fn().mockResolvedValue({
      checkId: 'mock-check',
      name: 'Mock Check',
      status: 'pass' as const,
      message: 'All good',
      fixable: false,
    }),
  };

  it('should render and run checks', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(
      <Diagnostics
        checks={[mockCheck]}
        context={{ projectRoot: '/test', verbose: false }}
        onComplete={onComplete}
      />
    );

    await vi.waitFor(() => {
      expect(onComplete).toHaveBeenCalled();
    });

    expect(lastFrame()).toContain('ANVIL DOCTOR');
  });

  it('should show check results', async () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(
      <Diagnostics
        checks={[mockCheck]}
        context={{ projectRoot: '/test', verbose: false }}
        onComplete={onComplete}
      />
    );

    await vi.waitFor(() => {
      expect(onComplete).toHaveBeenCalled();
    });

    expect(lastFrame()).toContain('Mock Check');
  });
});
