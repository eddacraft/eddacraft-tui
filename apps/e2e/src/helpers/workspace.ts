/**
 * Test Workspace Management
 *
 * Creates isolated temporary workspaces that mimic real Anvil project
 * structures. Each workspace gets its own directory tree, config files,
 * and git repository so tests never interfere with each other.
 */

import { execSync } from 'node:child_process';
import { mkdirSync, rmSync, existsSync, writeFileSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

export interface E2EWorkspace {
  /** Workspace root directory */
  root: string;
  /** .anvil/ directory */
  anvilDir: string;
  /** .anvil/plans/ directory */
  plansDir: string;
  /** Remove the workspace entirely */
  cleanup: () => void;
  /** Write a file relative to workspace root */
  writeFile: (relativePath: string, content: string) => void;
  /** Read a file relative to workspace root */
  readFile: (relativePath: string) => string;
  /** Check if a file exists relative to workspace root */
  fileExists: (relativePath: string) => boolean;
}

export interface WorkspaceOptions {
  /** Include a package.json (default: true) */
  withPackageJson?: boolean;
  /** Include .anvilrc config (default: true) */
  withAnvilrc?: boolean;
  /** Initialise a git repo (default: false) */
  withGit?: boolean;
  /** Package manager lockfile to create */
  lockfile?: 'npm' | 'pnpm' | 'yarn';
  /** Extra files to write: { relativePath: content } */
  files?: Record<string, string>;
}

const LOCKFILE_NAMES = {
  npm: 'package-lock.json',
  pnpm: 'pnpm-lock.yaml',
  yarn: 'yarn.lock',
} as const;

/**
 * Create an isolated test workspace with configurable project structure.
 *
 * @example
 * ```ts
 * const ws = createE2EWorkspace({ withGit: true, lockfile: 'pnpm' });
 * // use ws.root as cwd for CLI commands
 * ws.cleanup();
 * ```
 */
export function createE2EWorkspace(options: WorkspaceOptions = {}): E2EWorkspace {
  const {
    withPackageJson = true,
    withAnvilrc = true,
    withGit = false,
    lockfile,
    files = {},
  } = options;

  const root = join(
    tmpdir(),
    'anvil-e2e',
    `ws-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
  );
  const anvilDir = join(root, '.anvil');
  const plansDir = join(anvilDir, 'plans');

  mkdirSync(plansDir, { recursive: true });

  const ws: E2EWorkspace = {
    root,
    anvilDir,
    plansDir,
    cleanup: () => {
      if (existsSync(root)) {
        rmSync(root, { recursive: true, force: true });
      }
    },
    writeFile: (relativePath: string, content: string) => {
      const fullPath = join(root, relativePath);
      mkdirSync(join(fullPath, '..'), { recursive: true });
      writeFileSync(fullPath, content, 'utf-8');
    },
    readFile: (relativePath: string) => {
      return readFileSync(join(root, relativePath), 'utf-8');
    },
    fileExists: (relativePath: string) => {
      return existsSync(join(root, relativePath));
    },
  };

  // Seed standard project files
  if (withPackageJson) {
    ws.writeFile(
      'package.json',
      JSON.stringify(
        {
          name: 'e2e-test-project',
          version: '1.0.0',
          private: true,
          scripts: { test: 'echo "ok"', lint: 'echo "ok"', build: 'echo "ok"' },
        },
        null,
        2
      )
    );
  }

  if (withAnvilrc) {
    ws.writeFile(
      '.anvilrc',
      JSON.stringify(
        {
          planningDir: 'docs/planning',
          gateChecks: {
            eslint: { enabled: true, min_score: 80 },
            test: { enabled: true },
            coverage: { enabled: true, min_score: 80 },
            secrets: { enabled: true },
          },
          evidenceDir: '.anvil/evidence',
        },
        null,
        2
      )
    );
  }

  if (lockfile) {
    ws.writeFile(LOCKFILE_NAMES[lockfile], '');
  }

  if (withGit) {
    try {
      execSync('git init', { cwd: root, stdio: 'pipe' });
      execSync('git config user.email "e2e@test.local"', { cwd: root, stdio: 'pipe' });
      execSync('git config user.name "E2E Test"', { cwd: root, stdio: 'pipe' });
    } catch {
      // If git is unavailable, at least create the directory marker
      mkdirSync(join(root, '.git'), { recursive: true });
    }
  }

  // Write any extra files
  for (const [relativePath, content] of Object.entries(files)) {
    ws.writeFile(relativePath, content);
  }

  return ws;
}
