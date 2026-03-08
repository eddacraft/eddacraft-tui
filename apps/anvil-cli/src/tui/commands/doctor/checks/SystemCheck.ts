import { gitExecSync } from '@eddacraft/anvil-core';

import type { DiagnosticCheck, DiagnosticContext, DiagnosticResult } from '../types.js';

const MIN_NODE_VERSION = 20;

function isSpawnBlocked(error: unknown): boolean {
  if (!error || typeof error !== 'object') return false;
  const err = error as {
    code?: string;
    message?: string;
    cause?: { code?: string; message?: string };
  };
  if (err.code === 'EPERM') return true;
  if (typeof err.message === 'string' && err.message.includes('EPERM')) return true;
  // GitOperationError wraps the original error as `cause`
  if (err.cause) {
    if (err.cause.code === 'EPERM') return true;
    if (typeof err.cause.message === 'string' && err.cause.message.includes('EPERM')) return true;
  }
  return false;
}

export class NodeVersionCheck implements DiagnosticCheck {
  readonly id = 'node-version';
  readonly name = 'Node.js Version';
  readonly description = `Verifies Node.js version >= ${MIN_NODE_VERSION}`;

  async run(_context: DiagnosticContext): Promise<DiagnosticResult> {
    const version = process.versions.node;
    const major = parseInt(version.split('.')[0], 10);

    if (major >= MIN_NODE_VERSION) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'pass',
        message: `Node.js v${version}`,
        fixable: false,
      };
    }

    return {
      checkId: this.id,
      name: this.name,
      status: 'fail',
      message: `Node.js v${version} (requires >= v${MIN_NODE_VERSION})`,
      fixable: false,
      suggestion: `Install Node.js ${MIN_NODE_VERSION} or later from https://nodejs.org`,
    };
  }
}

export class GitCheck implements DiagnosticCheck {
  readonly id = 'git-available';
  readonly name = 'Git Available';
  readonly description = 'Verifies git is installed and accessible';

  async run(_context: DiagnosticContext): Promise<DiagnosticResult> {
    try {
      const output = gitExecSync(['--version']);
      const version = output.replace('git version ', '');

      return {
        checkId: this.id,
        name: this.name,
        status: 'pass',
        message: `git ${version}`,
        fixable: false,
      };
    } catch (error) {
      if (isSpawnBlocked(error)) {
        return {
          checkId: this.id,
          name: this.name,
          status: 'skip',
          message: 'git execution blocked by environment',
          fixable: false,
        };
      }

      return {
        checkId: this.id,
        name: this.name,
        status: 'fail',
        message: 'git not found in PATH',
        fixable: false,
        suggestion: 'Install git from https://git-scm.com',
      };
    }
  }
}

export class GitRepoCheck implements DiagnosticCheck {
  readonly id = 'git-repo';
  readonly name = 'Git Repository';
  readonly description = 'Verifies current directory is a git repository';

  async run(context: DiagnosticContext): Promise<DiagnosticResult> {
    try {
      gitExecSync(['rev-parse', '--git-dir'], { cwd: context.projectRoot });

      return {
        checkId: this.id,
        name: this.name,
        status: 'pass',
        message: 'Git repository detected',
        fixable: false,
      };
    } catch (error) {
      if (isSpawnBlocked(error)) {
        return {
          checkId: this.id,
          name: this.name,
          status: 'skip',
          message: 'git execution blocked by environment',
          fixable: false,
        };
      }

      return {
        checkId: this.id,
        name: this.name,
        status: 'warn',
        message: 'Not a git repository',
        fixable: true,
        suggestion: 'Run: git init',
      };
    }
  }

  async fix(
    context: DiagnosticContext
  ): Promise<{ success: boolean; message: string; commandsRun?: string[] }> {
    try {
      gitExecSync(['init'], { cwd: context.projectRoot });
      return {
        success: true,
        message: 'Initialised git repository',
        commandsRun: ['git init'],
      };
    } catch (error) {
      if (isSpawnBlocked(error)) {
        return {
          success: false,
          message: 'git execution blocked by environment',
        };
      }

      return {
        success: false,
        message: `Failed to initialise git: ${error instanceof Error ? error.message : 'Unknown error'}`,
      };
    }
  }
}
