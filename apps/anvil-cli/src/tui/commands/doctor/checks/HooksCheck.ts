import { execFileSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { debug } from '../../../../utils/output.js';
import { readJsonFileSync } from '../../../../utils/file-io.js';

import type { DiagnosticCheck, DiagnosticContext, DiagnosticResult, FixResult } from '../types.js';

export class HuskyInstalledCheck implements DiagnosticCheck {
  readonly id = 'husky-installed';
  readonly name = 'Husky Installed';
  readonly description = 'Verifies Husky git hooks manager is installed';

  private checkPackageJsonForHusky(projectRoot: string): boolean {
    const pkg = readJsonFileSync<Record<string, Record<string, unknown>>>(
      path.join(projectRoot, 'package.json')
    );
    if (!pkg) return false;

    return !!(
      pkg.devDependencies?.husky ||
      pkg.dependencies?.husky ||
      pkg.optionalDependencies?.husky
    );
  }

  async run(context: DiagnosticContext): Promise<DiagnosticResult> {
    const huskyDir = path.join(context.projectRoot, '.husky');

    if (fs.existsSync(huskyDir) && fs.statSync(huskyDir).isDirectory()) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'pass',
        message: 'Husky directory found',
        fixable: false,
      };
    }

    const hasHusky = this.checkPackageJsonForHusky(context.projectRoot);
    if (hasHusky) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'warn',
        message: 'Husky in package.json but .husky/ missing',
        fixable: true,
        suggestion: 'Run: npx husky init',
      };
    }

    return {
      checkId: this.id,
      name: this.name,
      status: 'skip',
      message: 'Husky not configured (optional)',
      fixable: false,
    };
  }

  async fix(context: DiagnosticContext): Promise<FixResult> {
    try {
      execFileSync('npx', ['husky', 'init'], {
        cwd: context.projectRoot,
        stdio: ['pipe', 'pipe', 'pipe'],
        timeout: 60_000,
      });
      return {
        success: true,
        message: 'Initialised Husky',
        commandsRun: ['npx husky init'],
      };
    } catch (error) {
      return {
        success: false,
        message: `Failed to initialise Husky: ${error instanceof Error ? error.message : 'Unknown error'}`,
      };
    }
  }
}

export class PreCommitHookCheck implements DiagnosticCheck {
  readonly id = 'pre-commit-hook';
  readonly name = 'Pre-commit Hook';
  readonly description = 'Verifies pre-commit hook exists and is executable';

  /**
   * Resolve the git hooks directory, handling worktrees where .git is a file.
   */
  private resolveGitHooksDir(projectRoot: string): string | null {
    const gitPath = path.join(projectRoot, '.git');
    if (!fs.existsSync(gitPath)) return null;

    try {
      const stat = fs.statSync(gitPath);
      if (stat.isDirectory()) return path.join(gitPath, 'hooks');

      // Worktree: .git is a file containing "gitdir: <path>"
      const content = fs.readFileSync(gitPath, 'utf-8').trim();
      const match = content.match(/^gitdir:\s+(.+)$/);
      if (!match) return null;

      const gitDir = path.resolve(projectRoot, match[1]);
      return fs.existsSync(gitDir) ? path.join(gitDir, 'hooks') : null;
    } catch {
      debug('HooksCheck: failed to resolve git hooks dir, returning null');
      return null;
    }
  }

  async run(context: DiagnosticContext): Promise<DiagnosticResult> {
    // Check .husky first, then fall back to .git/hooks
    const huskyDir = path.join(context.projectRoot, '.husky');
    const gitHooksDir = this.resolveGitHooksDir(context.projectRoot);

    let hookPath: string | null = null;

    if (fs.existsSync(huskyDir)) {
      const huskyHook = path.join(huskyDir, 'pre-commit');
      if (fs.existsSync(huskyHook)) hookPath = huskyHook;
    }

    if (!hookPath && gitHooksDir) {
      const gitHook = path.join(gitHooksDir, 'pre-commit');
      if (fs.existsSync(gitHook)) hookPath = gitHook;
    }

    if (!hookPath) {
      if (!fs.existsSync(huskyDir) && !gitHooksDir) {
        return {
          checkId: this.id,
          name: this.name,
          status: 'skip',
          message: 'Skipped (no hooks directory found)',
          fixable: false,
        };
      }

      return {
        checkId: this.id,
        name: this.name,
        status: 'warn',
        message: 'pre-commit hook missing',
        fixable: true,
        suggestion: 'Run: anvil hooks install',
      };
    }

    try {
      const stats = fs.statSync(hookPath);
      const isExecutable = (stats.mode & 0o111) !== 0;

      if (!isExecutable) {
        return {
          checkId: this.id,
          name: this.name,
          status: 'warn',
          message: 'pre-commit hook not executable',
          fixable: true,
          suggestion: `Run: chmod +x ${hookPath}`,
        };
      }

      return {
        checkId: this.id,
        name: this.name,
        status: 'pass',
        message: 'pre-commit hook active',
        fixable: false,
      };
    } catch {
      return {
        checkId: this.id,
        name: this.name,
        status: 'fail',
        message: 'Cannot read pre-commit hook',
        fixable: false,
      };
    }
  }

  async fix(context: DiagnosticContext): Promise<FixResult> {
    const hookPath = path.join(context.projectRoot, '.husky', 'pre-commit');

    try {
      if (!fs.existsSync(hookPath)) {
        const hookContent = `#!/usr/bin/env sh
. "$(dirname -- "$0")/_/husky.sh"

npx lint-staged
`;
        fs.writeFileSync(hookPath, hookContent, { mode: 0o755 });
        return {
          success: true,
          message: 'Created pre-commit hook',
          filesModified: [hookPath],
        };
      }

      fs.chmodSync(hookPath, 0o755);
      return {
        success: true,
        message: 'Made pre-commit hook executable',
        filesModified: [hookPath],
      };
    } catch (error) {
      return {
        success: false,
        message: `Failed to fix pre-commit hook: ${error instanceof Error ? error.message : 'Unknown error'}`,
      };
    }
  }
}
