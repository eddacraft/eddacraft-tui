import * as fs from 'node:fs';
import * as path from 'node:path';

import type { DiagnosticCheck, DiagnosticContext, DiagnosticResult, FixResult } from '../types.js';

export class AnvilDirWritableCheck implements DiagnosticCheck {
  readonly id = 'anvil-dir-writable';
  readonly name = 'Anvil Directory Writable';
  readonly description = 'Verifies write access to .anvil/ directory';

  async run(context: DiagnosticContext): Promise<DiagnosticResult> {
    const anvilDir = path.join(context.projectRoot, '.anvil');

    if (!fs.existsSync(anvilDir)) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'skip',
        message: 'Skipped (.anvil/ does not exist)',
        fixable: false,
      };
    }

    try {
      const testFile = path.join(anvilDir, '.write-test');
      fs.writeFileSync(testFile, 'test', 'utf8');
      fs.unlinkSync(testFile);

      return {
        checkId: this.id,
        name: this.name,
        status: 'pass',
        message: '.anvil/ is writable',
        fixable: false,
      };
    } catch {
      return {
        checkId: this.id,
        name: this.name,
        status: 'fail',
        message: '.anvil/ is not writable',
        fixable: true,
        suggestion: 'Check directory permissions',
      };
    }
  }

  async fix(context: DiagnosticContext): Promise<FixResult> {
    const anvilDir = path.join(context.projectRoot, '.anvil');

    try {
      fs.chmodSync(anvilDir, 0o755);
      return {
        success: true,
        message: 'Fixed .anvil/ permissions',
        filesModified: [anvilDir],
      };
    } catch (error) {
      return {
        success: false,
        message: `Failed to fix permissions: ${error instanceof Error ? error.message : 'Unknown error'}`,
      };
    }
  }
}

export class PlansDirReadableCheck implements DiagnosticCheck {
  readonly id = 'plans-dir-readable';
  readonly name = 'Plans Directory Readable';
  readonly description = 'Verifies planning directory is readable';

  async run(context: DiagnosticContext): Promise<DiagnosticResult> {
    const possibleDirs = ['plans', 'docs/plans', '.plans'];

    for (const dir of possibleDirs) {
      const fullPath = path.join(context.projectRoot, dir);
      if (fs.existsSync(fullPath)) {
        try {
          fs.readdirSync(fullPath);
          return {
            checkId: this.id,
            name: this.name,
            status: 'pass',
            message: `${dir}/ is readable`,
            fixable: false,
          };
        } catch {
          return {
            checkId: this.id,
            name: this.name,
            status: 'fail',
            message: `${dir}/ exists but is not readable`,
            fixable: true,
            suggestion: 'Check directory permissions',
          };
        }
      }
    }

    return {
      checkId: this.id,
      name: this.name,
      status: 'skip',
      message: 'No plans directory found (optional)',
      fixable: false,
    };
  }

  async fix(context: DiagnosticContext): Promise<FixResult> {
    const possibleDirs = ['plans', 'docs/plans', '.plans'];

    for (const dir of possibleDirs) {
      const fullPath = path.join(context.projectRoot, dir);
      if (fs.existsSync(fullPath)) {
        try {
          fs.chmodSync(fullPath, 0o755);
          return {
            success: true,
            message: `Fixed ${dir}/ permissions`,
            filesModified: [fullPath],
          };
        } catch (error) {
          return {
            success: false,
            message: `Failed to fix ${dir}/ permissions: ${error instanceof Error ? error.message : 'Unknown error'}`,
          };
        }
      }
    }

    return {
      success: false,
      message: 'No plans directory to fix',
    };
  }
}
