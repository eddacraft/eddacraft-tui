import { execSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';

import type { DiagnosticCheck, DiagnosticContext, DiagnosticResult, FixResult } from '../types.js';

export class HuskyInstalledCheck implements DiagnosticCheck {
  readonly id = 'husky-installed';
  readonly name = 'Husky Installed';
  readonly description = 'Verifies Husky git hooks manager is installed';

  private checkPackageJsonForHusky(projectRoot: string): boolean {
    const packageJsonPath = path.join(projectRoot, 'package.json');
    if (!fs.existsSync(packageJsonPath)) return false;

    try {
      const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
      return !!(
        pkg.devDependencies?.husky ||
        pkg.dependencies?.husky ||
        pkg.optionalDependencies?.husky
      );
    } catch {
      return false;
    }
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
      execSync('npx husky init', {
        cwd: context.projectRoot,
        stdio: ['pipe', 'pipe', 'pipe'],
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

  async run(context: DiagnosticContext): Promise<DiagnosticResult> {
    const huskyDir = path.join(context.projectRoot, '.husky');
    if (!fs.existsSync(huskyDir)) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'skip',
        message: 'Skipped (no .husky directory)',
        fixable: false,
      };
    }

    const hookPath = path.join(huskyDir, 'pre-commit');
    if (!fs.existsSync(hookPath)) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'warn',
        message: 'pre-commit hook missing',
        fixable: true,
        suggestion: 'Create pre-commit hook with lint-staged',
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
          suggestion: 'Run: chmod +x .husky/pre-commit',
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
