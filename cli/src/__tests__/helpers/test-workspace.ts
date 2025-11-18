/**
 * Test Workspace Utilities
 *
 * Shared utilities for setting up and tearing down test workspaces
 */

import { mkdirSync, rmSync, existsSync, writeFileSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import type { APSPlan } from '@anvil/core';
import { APS_SCHEMA_VERSION, generatePlanId, generateHash } from '@anvil/core';

export interface TestWorkspace {
  root: string;
  plansDir: string;
  anvilDir: string;
  cleanup: () => void;
}

/**
 * Create a temporary test workspace
 */
export function createTestWorkspace(): TestWorkspace {
  const root = join(tmpdir(), 'anvil-test', Math.random().toString(36).substring(7));
  const anvilDir = join(root, '.anvil');
  const plansDir = join(anvilDir, 'plans');

  mkdirSync(plansDir, { recursive: true });

  return {
    root,
    plansDir,
    anvilDir,
    cleanup: () => {
      if (existsSync(root)) {
        rmSync(root, { recursive: true, force: true });
      }
    },
  };
}

/**
 * Create a package.json file in the workspace
 */
export function createPackageJson(
  workspace: string,
  options: {
    name?: string;
    version?: string;
    scripts?: Record<string, string>;
    devDependencies?: Record<string, string>;
  } = {}
): void {
  const {
    name = 'test-workspace',
    version = '1.0.0',
    scripts = {},
    devDependencies = {},
  } = options;

  const packageJson = {
    name,
    version,
    scripts,
    devDependencies,
  };

  writeFileSync(join(workspace, 'package.json'), JSON.stringify(packageJson, null, 2), 'utf-8');
}

/**
 * Create an .anvilrc configuration file
 */
export function createAnvilrc(
  workspace: string,
  config: {
    planningDir?: string;
    gateChecks?: Record<string, unknown>;
    evidenceDir?: string;
  } = {}
): void {
  const defaultConfig = {
    planningDir: config.planningDir ?? 'docs/planning',
    gateChecks: config.gateChecks ?? {
      eslint: { enabled: true, min_score: 80 },
      test: { enabled: true },
      coverage: { enabled: true, min_score: 80 },
      secrets: { enabled: true },
    },
    evidenceDir: config.evidenceDir ?? '.anvil/evidence',
  };

  writeFileSync(join(workspace, '.anvilrc'), JSON.stringify(defaultConfig, null, 2), 'utf-8');
}

/**
 * Create a minimal APS plan for testing
 */
export function createMinimalAPSPlan(overrides: Partial<APSPlan> = {}): APSPlan {
  const planId = generatePlanId();
  const intent = overrides.intent ?? 'Test plan for unit testing';

  const planData = {
    id: planId,
    schema_version: APS_SCHEMA_VERSION,
    intent,
    proposed_changes: [],
    provenance: {
      timestamp: new Date().toISOString(),
      author: 'test-user',
      source: 'cli' as const,
      version: '1.0.0',
    },
    validations: {
      required_checks: ['lint', 'test', 'coverage', 'secrets'],
      skip_checks: [],
    },
    ...overrides,
  };

  const hash = generateHash(planData);
  return { ...planData, hash } as APSPlan;
}

/**
 * Create an ESLint configuration file
 */
export function createEslintConfig(workspace: string): void {
  const eslintConfig = {
    extends: ['eslint:recommended'],
    env: {
      node: true,
      es2022: true,
    },
    parserOptions: {
      ecmaVersion: 2022,
      sourceType: 'module',
    },
  };

  writeFileSync(join(workspace, '.eslintrc.json'), JSON.stringify(eslintConfig, null, 2), 'utf-8');
}

/**
 * Create a TypeScript configuration file
 */
export function createTsConfig(workspace: string): void {
  const tsConfig = {
    compilerOptions: {
      target: 'ES2022',
      module: 'NodeNext',
      moduleResolution: 'NodeNext',
      strict: true,
      esModuleInterop: true,
      skipLibCheck: true,
      forceConsistentCasingInFileNames: true,
    },
    include: ['src/**/*'],
    exclude: ['node_modules', 'dist'],
  };

  writeFileSync(join(workspace, 'tsconfig.json'), JSON.stringify(tsConfig, null, 2), 'utf-8');
}

/**
 * Create a .gitignore file
 */
export function createGitignore(workspace: string): void {
  const gitignore = `node_modules/
dist/
.anvil/
coverage/
.env
`;

  writeFileSync(join(workspace, '.gitignore'), gitignore, 'utf-8');
}

/**
 * Create a git repository
 */
export function initGitRepo(workspace: string): void {
  mkdirSync(join(workspace, '.git'), { recursive: true });
}

/**
 * Create lockfile for a specific package manager
 */
export function createLockfile(workspace: string, manager: 'npm' | 'pnpm' | 'yarn'): void {
  const lockfiles = {
    npm: 'package-lock.json',
    pnpm: 'pnpm-lock.yaml',
    yarn: 'yarn.lock',
  };

  writeFileSync(join(workspace, lockfiles[manager]), '', 'utf-8');
}

/**
 * Create a complete project setup with common development tools
 */
export function createFullProjectSetup(workspace: string): void {
  createPackageJson(workspace, {
    scripts: {
      test: 'vitest',
      lint: 'eslint .',
      build: 'tsc',
    },
    devDependencies: {
      vitest: '^3.0.0',
      eslint: '^8.0.0',
      typescript: '^5.0.0',
    },
  });
  createEslintConfig(workspace);
  createTsConfig(workspace);
  createGitignore(workspace);
  initGitRepo(workspace);
  createLockfile(workspace, 'pnpm');
}
