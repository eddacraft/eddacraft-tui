import * as fs from 'node:fs';
import * as path from 'node:path';

import type { DiagnosticCheck, DiagnosticContext, DiagnosticResult, FixResult } from '../types.js';

const DEFAULT_ANVILRC = {
  $schema: 'https://anvil.dev/schemas/anvilrc.json',
  planning: {
    directory: 'plans',
    format: 'aps',
  },
  checks: {
    eslint: { enabled: true },
    coverage: { enabled: true, threshold: 80 },
    secret: { enabled: true },
  },
};

export class ConfigExistsCheck implements DiagnosticCheck {
  readonly id = 'config-exists';
  readonly name = 'Configuration File';
  readonly description = 'Verifies .anvilrc exists';

  async run(context: DiagnosticContext): Promise<DiagnosticResult> {
    const configPath = path.join(context.projectRoot, '.anvilrc');

    if (fs.existsSync(configPath)) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'pass',
        message: '.anvilrc found',
        fixable: false,
      };
    }

    return {
      checkId: this.id,
      name: this.name,
      status: 'warn',
      message: '.anvilrc not found',
      fixable: true,
      suggestion: 'Run: anvil init',
    };
  }

  async fix(context: DiagnosticContext): Promise<FixResult> {
    const configPath = path.join(context.projectRoot, '.anvilrc');

    try {
      fs.writeFileSync(configPath, JSON.stringify(DEFAULT_ANVILRC, null, 2) + '\n', 'utf8');
      return {
        success: true,
        message: 'Created default .anvilrc',
        filesModified: [configPath],
      };
    } catch (error) {
      return {
        success: false,
        message: `Failed to create .anvilrc: ${error instanceof Error ? error.message : 'Unknown error'}`,
      };
    }
  }
}

export class ConfigValidCheck implements DiagnosticCheck {
  readonly id = 'config-valid';
  readonly name = 'Configuration Valid';
  readonly description = 'Verifies .anvilrc is valid JSON';

  async run(context: DiagnosticContext): Promise<DiagnosticResult> {
    const configPath = path.join(context.projectRoot, '.anvilrc');

    if (!fs.existsSync(configPath)) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'skip',
        message: 'Skipped (no .anvilrc)',
        fixable: false,
      };
    }

    try {
      const content = fs.readFileSync(configPath, 'utf8');
      JSON.parse(content);

      return {
        checkId: this.id,
        name: this.name,
        status: 'pass',
        message: 'Valid JSON configuration',
        fixable: false,
      };
    } catch (error) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'fail',
        message: 'Invalid JSON in .anvilrc',
        fixable: true,
        details: error instanceof Error ? error.message : 'Parse error',
        suggestion: 'Fix JSON syntax errors or run: anvil init --force',
      };
    }
  }

  async fix(context: DiagnosticContext): Promise<FixResult> {
    const configPath = path.join(context.projectRoot, '.anvilrc');
    const backupPath = configPath + '.backup';

    try {
      if (fs.existsSync(configPath)) {
        fs.copyFileSync(configPath, backupPath);
      }
      fs.writeFileSync(configPath, JSON.stringify(DEFAULT_ANVILRC, null, 2) + '\n', 'utf8');
      return {
        success: true,
        message: 'Recreated .anvilrc with defaults (backup saved)',
        filesModified: [configPath, backupPath],
      };
    } catch (error) {
      return {
        success: false,
        message: `Failed to fix .anvilrc: ${error instanceof Error ? error.message : 'Unknown error'}`,
      };
    }
  }
}

export class AnvilDirCheck implements DiagnosticCheck {
  readonly id = 'anvil-dir';
  readonly name = 'Anvil Directory';
  readonly description = 'Verifies .anvil/ directory exists';

  async run(context: DiagnosticContext): Promise<DiagnosticResult> {
    const anvilDir = path.join(context.projectRoot, '.anvil');

    if (fs.existsSync(anvilDir) && fs.statSync(anvilDir).isDirectory()) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'pass',
        message: '.anvil/ directory exists',
        fixable: false,
      };
    }

    return {
      checkId: this.id,
      name: this.name,
      status: 'warn',
      message: '.anvil/ directory missing',
      fixable: true,
      suggestion: 'Directory will be created automatically',
    };
  }

  async fix(context: DiagnosticContext): Promise<FixResult> {
    const anvilDir = path.join(context.projectRoot, '.anvil');
    const cacheDir = path.join(anvilDir, 'cache');

    try {
      fs.mkdirSync(cacheDir, { recursive: true });
      return {
        success: true,
        message: 'Created .anvil/ directory',
        filesModified: [anvilDir, cacheDir],
      };
    } catch (error) {
      return {
        success: false,
        message: `Failed to create .anvil/: ${error instanceof Error ? error.message : 'Unknown error'}`,
      };
    }
  }
}
