import { execFile } from 'node:child_process';
import { join } from 'node:path';
import { mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import ora from 'ora';
import chalk from 'chalk';
import type { PreflightCheck, PreflightCheckResult, PreflightResult } from './release-types.js';
import { PREFLIGHT_CHECKS } from './release-types.js';

function runCommand(
  command: string,
  args: string[],
  cwd: string
): Promise<{ stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    execFile(
      command,
      args,
      { cwd, maxBuffer: 10 * 1024 * 1024, timeout: 300_000 },
      (error, stdout, stderr) => {
        if (error) {
          reject(new Error(stderr || stdout || error.message, { cause: error }));
        } else {
          resolve({ stdout, stderr });
        }
      }
    );
  });
}

async function runCheck(
  check: PreflightCheck,
  workspaceRoot: string,
  verbose: boolean
): Promise<PreflightCheckResult> {
  const cwd = check.cwd ? join(workspaceRoot, check.cwd) : workspaceRoot;
  const start = Date.now();

  try {
    const { stdout, stderr } = await runCommand(check.command, check.args, cwd);
    return {
      name: check.name,
      passed: true,
      output: verbose ? stdout + stderr : '',
      durationMs: Date.now() - start,
    };
  } catch (err) {
    return {
      name: check.name,
      passed: false,
      output: err instanceof Error ? err.message : String(err),
      durationMs: Date.now() - start,
    };
  }
}

async function runSmokeCheck(workspaceRoot: string): Promise<PreflightCheckResult> {
  const start = Date.now();
  const tmpDir = mkdtempSync(join(tmpdir(), 'anvil-smoke-'));

  try {
    // Pack the CLI
    await runCommand(
      'pnpm',
      ['pack', '--pack-destination', tmpDir],
      join(workspaceRoot, 'apps/anvil-cli')
    );

    // Find the tarball deterministically
    const files = readdirSync(tmpDir);
    const tarballs = files.filter((file) => file.endsWith('.tgz')).sort();
    const tarball = tarballs[0];
    if (!tarball) throw new Error('No tarball produced by pnpm pack');

    // Run anvil --help from the tarball
    await runCommand(
      'npx',
      ['-y', '--package', join(tmpDir, tarball), 'anvil', '--help'],
      workspaceRoot
    );

    return {
      name: 'smoke',
      passed: true,
      output: '',
      durationMs: Date.now() - start,
    };
  } catch (err) {
    return {
      name: 'smoke',
      passed: false,
      output: err instanceof Error ? err.message : String(err),
      durationMs: Date.now() - start,
    };
  } finally {
    rmSync(tmpDir, { recursive: true, force: true });
  }
}

export async function runPreflight(
  workspaceRoot: string,
  verbose: boolean
): Promise<PreflightResult> {
  const results: PreflightCheckResult[] = [];
  const totalStart = Date.now();

  for (const check of PREFLIGHT_CHECKS) {
    const spinner = ora({ text: check.label, prefixText: '  ' }).start();
    const result = await runCheck(check, workspaceRoot, verbose);
    results.push(result);

    if (result.passed) {
      const duration = (result.durationMs / 1000).toFixed(1);
      spinner.succeed(`${check.label}  ${chalk.dim(`${duration}s`)}`);
    } else {
      spinner.fail(`${check.label}`);
      if (result.output) {
        console.log(chalk.red(result.output.split('\n').slice(0, 20).join('\n')));
      }
      return { checks: results, allPassed: false, totalDurationMs: Date.now() - totalStart };
    }
  }

  // Smoke check
  const smokeSpinner = ora({
    text: 'smoke check (anvil --help from tarball)',
    prefixText: '  ',
  }).start();
  const smokeResult = await runSmokeCheck(workspaceRoot);
  results.push(smokeResult);

  if (smokeResult.passed) {
    const duration = (smokeResult.durationMs / 1000).toFixed(1);
    smokeSpinner.succeed(`smoke check  ${chalk.dim(`${duration}s`)}`);
  } else {
    smokeSpinner.fail('smoke check');
    if (smokeResult.output) {
      console.log(chalk.red(smokeResult.output.split('\n').slice(0, 20).join('\n')));
    }
    return { checks: results, allPassed: false, totalDurationMs: Date.now() - totalStart };
  }

  return { checks: results, allPassed: true, totalDurationMs: Date.now() - totalStart };
}
